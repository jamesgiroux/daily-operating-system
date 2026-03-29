//! PTY Manager for Claude Code subprocess management
//!
//! Spawns Claude Code via pseudo-terminal with timeout handling.
//! This is necessary because Claude Code expects an interactive terminal.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::ExecutionError;
use crate::types::AiModelConfig;

/// Cached resolved path to the `claude` binary.
/// Caches `Some` results; re-probes on `None` so installing Claude while
/// the app is running gets detected on the next check.
static CLAUDE_BINARY: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Resolve the absolute path to the `claude` CLI binary.
///
/// macOS apps launched from Finder/DMG don't inherit the user's shell PATH,
/// so `which claude` fails even when claude is installed. This function checks
/// common install locations as a fallback.
///
/// Caches successful lookups. Re-probes if not yet found.
fn resolve_claude_binary() -> Option<PathBuf> {
    let mut guard = CLAUDE_BINARY.lock().ok()?;
    if let Some(ref path) = *guard {
        return Some(path.clone());
    }

    let found = probe_claude_binary();
    if found.is_some() {
        *guard = found.clone();
    }
    found
}

/// Actual filesystem probe for the claude binary.
fn probe_claude_binary() -> Option<PathBuf> {
    // 1. Try `which claude` (works in terminal, dev mode, or if PATH is correct)
    if let Ok(output) = Command::new("which").arg("claude").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                log::info!("Resolved claude binary via PATH: {}", path);
                return Some(PathBuf::from(path));
            }
        }
    }

    // 2. Check common install locations (Finder-launched apps won't have shell PATH)
    let home = dirs::home_dir().unwrap_or_default();
    let candidates = [
        home.join(".local/bin/claude"),            // npm global (default)
        home.join(".npm/bin/claude"),              // npm alternate
        home.join(".nvm/current/bin/claude"),      // nvm
        PathBuf::from("/usr/local/bin/claude"),    // Homebrew / manual
        PathBuf::from("/opt/homebrew/bin/claude"), // Homebrew on Apple Silicon
    ];

    for candidate in &candidates {
        if candidate.is_file() {
            log::info!("Resolved claude binary at: {}", candidate.display());
            return Some(candidate.clone());
        }
    }

    log::warn!("Claude binary not found in PATH or common install locations");
    None
}

/// Default timeout for AI enrichment phase (5 minutes)
pub const DEFAULT_CLAUDE_TIMEOUT_SECS: u64 = 300;
pub const AI_USAGE_DAILY_KEY: &str = "ai_usage_daily";
pub const AI_USAGE_RECENT_KEY: &str = "ai_usage_recent";
pub const BACKGROUND_AI_GUARD_KEY: &str = "background_ai_guard";
pub const ESTIMATED_DAILY_TOKEN_BUDGET: u32 = 50_000;
const RECENT_AI_USAGE_LIMIT: usize = 200;
const BACKGROUND_AI_TOKEN_WINDOW_HOURS: i64 = 4;
const BACKGROUND_AI_PAUSE_MINUTES: i64 = 30;
const BACKGROUND_AI_TIMEOUT_SAMPLE: usize = 20;
const BACKGROUND_AI_TIMEOUT_RATE_THRESHOLD: f64 = 0.25;
const BACKGROUND_AI_CONSECUTIVE_TIMEOUTS: usize = 3;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AiUsageLedger {
    #[serde(default)]
    pub days: std::collections::BTreeMap<String, AiUsageDay>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AiUsageDay {
    #[serde(default)]
    pub call_count: u32,
    #[serde(default)]
    pub estimated_prompt_tokens: u32,
    #[serde(default)]
    pub estimated_output_tokens: u32,
    #[serde(default)]
    pub total_duration_ms: u64,
    #[serde(default)]
    pub call_sites: HashMap<String, u32>,
    #[serde(default)]
    pub operation_counts: HashMap<String, u32>,
    #[serde(default)]
    pub model_counts: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiUsageContext {
    pub subsystem: String,
    pub operation: String,
    pub trigger: String,
    pub tier: String,
    pub background: bool,
}

impl Default for AiUsageContext {
    fn default() -> Self {
        Self {
            subsystem: "claude".to_string(),
            operation: "unspecified".to_string(),
            trigger: "unknown".to_string(),
            tier: "unspecified".to_string(),
            background: false,
        }
    }
}

impl AiUsageContext {
    pub fn new(subsystem: &str, operation: &str) -> Self {
        Self {
            subsystem: subsystem.to_string(),
            operation: operation.to_string(),
            ..Self::default()
        }
    }

    pub fn for_tier(tier: ModelTier) -> Self {
        Self {
            operation: format!("{}_task", tier.as_str()),
            tier: tier.as_str().to_string(),
            ..Self::default()
        }
    }

    pub fn with_trigger(mut self, trigger: &str) -> Self {
        self.trigger = trigger.to_string();
        self
    }

    pub fn with_background(mut self, background: bool) -> Self {
        self.background = background;
        self
    }

    pub fn with_tier(mut self, tier: ModelTier) -> Self {
        self.tier = tier.as_str().to_string();
        self
    }

    pub fn operation_key(&self) -> String {
        format!("{}:{}", self.subsystem, self.operation)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AiRecentUsageLedger {
    #[serde(default)]
    pub calls: Vec<AiRecentUsageCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiRecentUsageCall {
    pub timestamp: String,
    pub subsystem: String,
    pub operation: String,
    pub trigger: String,
    pub tier: String,
    pub model: String,
    pub background: bool,
    pub status: String,
    pub duration_ms: u64,
    pub estimated_prompt_tokens: u32,
    pub estimated_output_tokens: u32,
    pub estimated_total_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundAiGuardState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused_until: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundAiPauseStatus {
    pub paused: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused_until: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub rolling_4h_tokens: u32,
    pub background_calls_4h: u32,
    pub timeout_rate_last_20: f64,
    pub consecutive_background_timeouts: u32,
}

fn estimate_tokens(text: &str) -> u32 {
    text.split_whitespace().count() as u32
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn read_json_kv<T>(db: &crate::db::ActionDb, key: &str) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    db.conn_ref()
        .query_row(
            "SELECT value_json FROM app_state_kv WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str(&json).ok())
}

fn write_json_kv<T>(db: &crate::db::ActionDb, key: &str, value: &T)
where
    T: Serialize,
{
    if let Ok(value_json) = serde_json::to_string(value) {
        let _ = db.conn_ref().execute(
            "INSERT OR REPLACE INTO app_state_kv (key, value_json, updated_at)
             VALUES (?1, ?2, ?3)",
            params![key, value_json, Utc::now().to_rfc3339()],
        );
    }
}

fn build_background_pause_status(
    recent: &AiRecentUsageLedger,
    guard: &BackgroundAiGuardState,
) -> BackgroundAiPauseStatus {
    let now = Utc::now();
    let window_start = now - ChronoDuration::hours(BACKGROUND_AI_TOKEN_WINDOW_HOURS);
    let background_calls: Vec<&AiRecentUsageCall> = recent
        .calls
        .iter()
        .filter(|call| {
            call.background
                && parse_timestamp(&call.timestamp)
                    .map(|timestamp| timestamp >= window_start)
                    .unwrap_or(false)
        })
        .collect();

    let rolling_4h_tokens = background_calls
        .iter()
        .map(|call| call.estimated_total_tokens)
        .sum();

    let last_background_calls: Vec<&AiRecentUsageCall> = recent
        .calls
        .iter()
        .rev()
        .filter(|call| call.background)
        .take(BACKGROUND_AI_TIMEOUT_SAMPLE)
        .collect();

    let timeout_count = last_background_calls
        .iter()
        .filter(|call| call.status == "timeout")
        .count();
    let timeout_rate_last_20 = if last_background_calls.is_empty() {
        0.0
    } else {
        timeout_count as f64 / last_background_calls.len() as f64
    };

    let consecutive_background_timeouts = recent
        .calls
        .iter()
        .rev()
        .filter(|call| call.background)
        .take_while(|call| call.status == "timeout")
        .count() as u32;

    let paused_until_dt = guard
        .paused_until
        .as_deref()
        .and_then(parse_timestamp)
        .filter(|paused_until| *paused_until > now);

    BackgroundAiPauseStatus {
        paused: paused_until_dt.is_some(),
        paused_until: paused_until_dt.map(|dt| dt.to_rfc3339()),
        reason: if paused_until_dt.is_some() {
            guard.reason.clone()
        } else {
            None
        },
        rolling_4h_tokens,
        background_calls_4h: background_calls.len() as u32,
        timeout_rate_last_20,
        consecutive_background_timeouts,
    }
}

fn background_pause_reason(status: &BackgroundAiPauseStatus) -> Option<String> {
    if status.rolling_4h_tokens >= ESTIMATED_DAILY_TOKEN_BUDGET {
        Some(format!(
            "Paused background AI after {} estimated tokens in the last {} hours",
            status.rolling_4h_tokens, BACKGROUND_AI_TOKEN_WINDOW_HOURS
        ))
    } else if status.timeout_rate_last_20 >= BACKGROUND_AI_TIMEOUT_RATE_THRESHOLD {
        Some(format!(
            "Paused background AI after {:.0}% timeout rate across the last {} background calls",
            status.timeout_rate_last_20 * 100.0,
            BACKGROUND_AI_TIMEOUT_SAMPLE
        ))
    } else if status.consecutive_background_timeouts
        >= BACKGROUND_AI_CONSECUTIVE_TIMEOUTS as u32
    {
        Some(format!(
            "Paused background AI after {} consecutive background timeouts",
            status.consecutive_background_timeouts
        ))
    } else {
        None
    }
}

fn maybe_refresh_background_ai_guard(db: &crate::db::ActionDb) -> BackgroundAiPauseStatus {
    let recent: AiRecentUsageLedger = read_json_kv(db, AI_USAGE_RECENT_KEY).unwrap_or_default();
    let mut guard: BackgroundAiGuardState =
        read_json_kv(db, BACKGROUND_AI_GUARD_KEY).unwrap_or_default();

    let status = build_background_pause_status(&recent, &guard);
    if status.paused {
        return status;
    }

    let reason = background_pause_reason(&status);

    if let Some(reason) = reason {
        guard.paused_until = Some(
            (Utc::now() + ChronoDuration::minutes(BACKGROUND_AI_PAUSE_MINUTES)).to_rfc3339(),
        );
        guard.reason = Some(reason);
        write_json_kv(db, BACKGROUND_AI_GUARD_KEY, &guard);
        build_background_pause_status(&recent, &guard)
    } else if guard.paused_until.is_some() || guard.reason.is_some() {
        guard = BackgroundAiGuardState::default();
        write_json_kv(db, BACKGROUND_AI_GUARD_KEY, &guard);
        build_background_pause_status(&recent, &guard)
    } else {
        status
    }
}

pub fn current_background_ai_pause_status() -> BackgroundAiPauseStatus {
    let Ok(db) = crate::db::ActionDb::open() else {
        return BackgroundAiPauseStatus {
            paused: false,
            paused_until: None,
            reason: None,
            rolling_4h_tokens: 0,
            background_calls_4h: 0,
            timeout_rate_last_20: 0.0,
            consecutive_background_timeouts: 0,
        };
    };
    maybe_refresh_background_ai_guard(&db)
}

pub fn background_ai_paused() -> bool {
    current_background_ai_pause_status().paused
}

fn record_ai_usage(
    context: &AiUsageContext,
    model: Option<&str>,
    command: &str,
    output: &str,
    duration: Duration,
    status: &str,
    error: Option<&str>,
) {
    let prompt_tokens = estimate_tokens(command);
    let output_tokens = estimate_tokens(output);
    let total_tokens = prompt_tokens + output_tokens;
    let duration_ms = duration.as_millis() as u64;
    let call_site = context.operation_key();
    let model_name = model.unwrap_or("default").to_string();

    let mut audit = crate::audit_log::AuditLogger::new(crate::audit_log::default_audit_log_path());
    let _ = audit.append(
        "ai",
        "ai_call_completed",
        serde_json::json!({
            "subsystem": context.subsystem.clone(),
            "operation": context.operation.clone(),
            "trigger": context.trigger.clone(),
            "tier": context.tier.clone(),
            "background": context.background,
            "callSite": call_site.clone(),
            "model": model_name.clone(),
            "status": status,
            "durationMs": duration_ms,
            "estimatedPromptTokens": prompt_tokens,
            "estimatedOutputTokens": output_tokens,
            "estimatedTotalTokens": total_tokens,
            "error": error,
        }),
    );

    let Ok(db) = crate::db::ActionDb::open() else {
        return;
    };
    let mut ledger: AiUsageLedger = read_json_kv(&db, AI_USAGE_DAILY_KEY).unwrap_or_default();

    let today = Utc::now().format("%Y-%m-%d").to_string();
    let day = ledger.days.entry(today).or_default();
    day.call_count += 1;
    day.estimated_prompt_tokens += prompt_tokens;
    day.estimated_output_tokens += output_tokens;
    day.total_duration_ms += duration_ms;
    *day.call_sites.entry(call_site.clone()).or_insert(0) += 1;
    *day.operation_counts.entry(call_site).or_insert(0) += 1;
    *day.model_counts.entry(model_name.clone()).or_insert(0) += 1;

    let cutoff = Utc::now().date_naive() - ChronoDuration::days(6);
    ledger
        .days
        .retain(|date, _| parse_usage_day(date).is_some_and(|parsed| parsed >= cutoff));

    write_json_kv(&db, AI_USAGE_DAILY_KEY, &ledger);

    let mut recent: AiRecentUsageLedger = read_json_kv(&db, AI_USAGE_RECENT_KEY).unwrap_or_default();
    recent.calls.push(AiRecentUsageCall {
        timestamp: Utc::now().to_rfc3339(),
        subsystem: context.subsystem.clone(),
        operation: context.operation.clone(),
        trigger: context.trigger.clone(),
        tier: context.tier.clone(),
        model: model_name,
        background: context.background,
        status: status.to_string(),
        duration_ms,
        estimated_prompt_tokens: prompt_tokens,
        estimated_output_tokens: output_tokens,
        estimated_total_tokens: total_tokens,
        error: error.map(|value| value.to_string()),
    });
    if recent.calls.len() > RECENT_AI_USAGE_LIMIT {
        let drain = recent.calls.len() - RECENT_AI_USAGE_LIMIT;
        recent.calls.drain(0..drain);
    }
    write_json_kv(&db, AI_USAGE_RECENT_KEY, &recent);

    let _ = maybe_refresh_background_ai_guard(&db);
}

fn parse_usage_day(value: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

/// Model tier for AI operations (I174).
///
/// Maps to configured model names via `AiModelConfig`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    /// Intelligence, briefing, week narrative — needs synthesis
    Synthesis,
    /// Emails, preps — structured extraction
    Extraction,
    /// Automatic background maintenance should stay cheap.
    Background,
    /// Inbox classification, file summaries — mechanical tasks
    Mechanical,
}

impl ModelTier {
    pub fn as_str(self) -> &'static str {
        match self {
            ModelTier::Synthesis => "synthesis",
            ModelTier::Extraction => "extraction",
            ModelTier::Background => "background",
            ModelTier::Mechanical => "mechanical",
        }
    }
}

/// PTY Manager for spawning Claude Code
pub struct PtyManager {
    timeout_secs: u64,
    model: Option<String>,
    nice_priority: Option<i32>,
    usage_context: AiUsageContext,
}

/// Strip ANSI escape sequences from PTY output.
///
/// Even with TERM=dumb, some programs emit minimal escape codes. This is a
/// defensive safety net applied to all Claude output before parsing.
fn strip_ansi(input: &str) -> String {
    // Matches CSI sequences (\x1b[...X), OSC sequences (\x1b]...BEL/ST), and simple escapes (\x1b[X)
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek() {
                Some('[') => {
                    // CSI sequence: consume until a letter
                    chars.next();
                    while let Some(&next) = chars.peek() {
                        chars.next();
                        if next.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                Some(']') => {
                    // OSC sequence: consume until BEL (\x07) or ST (\x1b\\)
                    chars.next();
                    while let Some(&next) = chars.peek() {
                        if next == '\x07' {
                            chars.next();
                            break;
                        }
                        if next == '\x1b' {
                            chars.next();
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                        chars.next();
                    }
                }
                _ => {
                    // Simple escape: skip next char
                    chars.next();
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn is_model_unavailable_output(output: &str) -> bool {
    let lower = output.to_lowercase();
    (lower.contains("model") && lower.contains("not available"))
        || (lower.contains("unknown model"))
        || (lower.contains("invalid model"))
        || (lower.contains("model") && lower.contains("not found"))
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            timeout_secs: DEFAULT_CLAUDE_TIMEOUT_SECS,
            model: None,
            nice_priority: None,
            usage_context: AiUsageContext::default(),
        }
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set CPU priority via `nice` for the subprocess (I173).
    /// Lower values = higher priority. 10 is a reasonable default for background work.
    pub fn with_nice_priority(mut self, priority: i32) -> Self {
        self.nice_priority = Some(priority);
        self
    }

    pub fn with_usage_context(mut self, usage_context: AiUsageContext) -> Self {
        self.usage_context = usage_context;
        self
    }

    /// Create a PtyManager configured for a specific model tier.
    pub fn for_tier(tier: ModelTier, config: &AiModelConfig) -> Self {
        let model = match tier {
            ModelTier::Synthesis => &config.synthesis,
            ModelTier::Extraction => &config.extraction,
            ModelTier::Background => &config.background,
            ModelTier::Mechanical => &config.mechanical,
        };
        Self::new()
            .with_model(model.clone())
            .with_usage_context(AiUsageContext::for_tier(tier))
    }

    /// Check if Claude Code CLI is available
    pub fn is_claude_available() -> bool {
        resolve_claude_binary().is_some()
    }

    /// Return the absolute path to the claude binary, if found.
    pub fn resolve_binary_path() -> Option<std::path::PathBuf> {
        resolve_claude_binary()
    }

    /// Check if Claude Code is authenticated.
    ///
    /// Checks the macOS Keychain for the "Claude Code-credentials" entry that
    /// Claude Code writes when OAuth completes. This is faster and more reliable
    /// than running `claude --print hello`, which makes an actual LLM API call
    /// and times out even when the user is authenticated.
    pub fn is_claude_authenticated() -> Result<bool, ExecutionError> {
        use std::process::Stdio;

        let output = Command::new("security")
            .args(["find-generic-password", "-s", "Claude Code-credentials"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .map_err(|e| ExecutionError::IoError(format!("Keychain check failed: {}", e)))?;

        Ok(output.status.success())
    }

    /// Spawn Claude Code with a command in the given workspace
    ///
    /// Uses PTY to handle Claude's interactive terminal requirements.
    /// Returns the captured output on success.
    pub fn spawn_claude(
        &self,
        workspace: &Path,
        command: &str,
    ) -> Result<ClaudeOutput, ExecutionError> {
        let claude_path = resolve_claude_binary().ok_or(ExecutionError::ClaudeCodeNotFound)?;
        let claude_str = claude_path.to_string_lossy();
        let started = std::time::Instant::now();

        let pty_system = NativePtySystem::default();

        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 4096, // Wide enough to prevent hard line wrapping of structured output
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| ExecutionError::IoError(format!("Failed to open PTY: {}", e)))?;

        // Build the command, optionally wrapped in `nice` for CPU priority
        let mut cmd = if let Some(priority) = self.nice_priority {
            let mut c = CommandBuilder::new("nice");
            let prio_str = priority.to_string();
            c.args(["-n", &prio_str, &*claude_str]);
            if let Some(ref model) = self.model {
                c.args(["--model", model, "--print", command]);
            } else {
                c.args(["--print", command]);
            }
            c
        } else {
            let mut c = CommandBuilder::new(claude_path.as_os_str());
            if let Some(ref model) = self.model {
                c.args(["--model", model, "--print", command]);
            } else {
                c.args(["--print", command]);
            }
            c
        };
        cmd.cwd(workspace);

        // Suppress ANSI escape codes and terminal control sequences
        cmd.env("TERM", "dumb");

        // Remove Claude Code session env vars so the child process doesn't
        // detect itself as a nested session and refuse to run.
        for key in [
            "CLAUDECODE",
            "CLAUDE_CODE_SSE_PORT",
            "CLAUDE_CODE_ENTRYPOINT",
        ] {
            cmd.env_remove(key);
        }

        // Spawn the child process
        let _child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| ExecutionError::IoError(format!("Failed to spawn claude: {}", e)))?;

        // Drop the slave to avoid blocking
        drop(pair.slave);

        // Read output with timeout
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| ExecutionError::IoError(format!("Failed to clone PTY reader: {}", e)))?;

        // Use a channel to handle timeout
        let (tx, rx) = mpsc::channel();
        let timeout = Duration::from_secs(self.timeout_secs);

        // Spawn reader thread
        thread::spawn(move || {
            let mut output = String::new();
            let mut buf = [0u8; 1024];

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if let Ok(s) = std::str::from_utf8(&buf[..n]) {
                            output.push_str(s);
                        }
                    }
                    Err(_) => break,
                }
            }

            let _ = tx.send(output);
        });

        // Wait for output with timeout
        let raw_output = match rx.recv_timeout(timeout) {
            Ok(output) => output,
            Err(_) => {
                record_ai_usage(
                    &self.usage_context,
                    self.model.as_deref(),
                    command,
                    "",
                    started.elapsed(),
                    "timeout",
                    Some("pty_timeout"),
                );
                return Err(ExecutionError::Timeout(self.timeout_secs));
            }
        };

        // Strip any ANSI escape codes that leaked through despite TERM=dumb
        let output = strip_ansi(&raw_output);

        log::debug!(
            "Claude output ({} bytes, {} after strip): {}",
            raw_output.len(),
            output.len(),
            &output[..output.len().min(500)]
        );

        // Check for known error patterns in output
        if output.contains("not authenticated")
            || output.contains("please login")
            || output.contains("login required")
        {
            record_ai_usage(
                &self.usage_context,
                self.model.as_deref(),
                command,
                &output,
                started.elapsed(),
                "auth_required",
                Some("not_authenticated"),
            );
            return Err(ExecutionError::ClaudeCodeNotAuthenticated);
        }

        if is_model_unavailable_output(&output) {
            let first_line = output.lines().next().unwrap_or("Model unavailable");
            record_ai_usage(
                &self.usage_context,
                self.model.as_deref(),
                command,
                &output,
                started.elapsed(),
                "model_unavailable",
                Some(first_line),
            );
            return Err(ExecutionError::ConfigurationError(format!(
                "model_unavailable: {}",
                first_line
            )));
        }

        if output.contains("rate limit") || output.contains("too many requests") {
            record_ai_usage(
                &self.usage_context,
                self.model.as_deref(),
                command,
                &output,
                started.elapsed(),
                "rate_limited",
                Some("rate_limit"),
            );
            return Err(ExecutionError::ApiRateLimit);
        }

        if output.contains("subscription") && output.contains("limit") {
            record_ai_usage(
                &self.usage_context,
                self.model.as_deref(),
                command,
                &output,
                started.elapsed(),
                "subscription_limit",
                Some("subscription_limit"),
            );
            return Err(ExecutionError::ClaudeSubscriptionLimit);
        }

        if output.contains("cannot be launched inside another Claude Code session") {
            record_ai_usage(
                &self.usage_context,
                self.model.as_deref(),
                command,
                &output,
                started.elapsed(),
                "nested_session",
                Some("nested_session"),
            );
            return Err(ExecutionError::ConfigurationError(
                "Nested Claude Code session detected. CLAUDECODE env var leaked to subprocess."
                    .to_string(),
            ));
        }

        record_ai_usage(
            &self.usage_context,
            self.model.as_deref(),
            command,
            &output,
            started.elapsed(),
            "success",
            None,
        );

        Ok(ClaudeOutput {
            stdout: output,
            exit_code: 0, // Assume success if we got here
        })
    }
}

/// Output from Claude Code execution
#[derive(Debug)]
pub struct ClaudeOutput {
    pub stdout: String,
    pub exit_code: i32,
}

#[cfg(test)]
mod tests {
    use super::{
        background_pause_reason, build_background_pause_status, is_model_unavailable_output,
        strip_ansi, AiRecentUsageCall, AiRecentUsageLedger, BackgroundAiGuardState,
        ESTIMATED_DAILY_TOKEN_BUDGET,
    };
    use chrono::{Duration as ChronoDuration, Utc};

    #[test]
    fn detects_model_unavailable_output() {
        assert!(is_model_unavailable_output(
            "Error: model sonnet-4 not available for this account"
        ));
        assert!(is_model_unavailable_output(
            "unknown model: custom-model-name"
        ));
        assert!(!is_model_unavailable_output("rate limit exceeded"));
    }

    #[test]
    fn strip_ansi_removes_csi_sequences() {
        assert_eq!(strip_ansi("\x1b[1mENRICHMENT:e1\x1b[0m"), "ENRICHMENT:e1");
        assert_eq!(
            strip_ansi("\x1b[32mSUMMARY: hello world\x1b[0m"),
            "SUMMARY: hello world"
        );
    }

    #[test]
    fn strip_ansi_removes_osc_sequences() {
        assert_eq!(
            strip_ansi("\x1b]0;Claude Code\x07ENRICHMENT:e1"),
            "ENRICHMENT:e1"
        );
    }

    #[test]
    fn strip_ansi_preserves_clean_text() {
        let clean = "ENRICHMENT:e1\nSUMMARY: test\nEND_ENRICHMENT";
        assert_eq!(strip_ansi(clean), clean);
    }

    #[test]
    fn strip_ansi_handles_empty_input() {
        assert_eq!(strip_ansi(""), "");
    }

    #[test]
    fn strip_ansi_handles_complex_sequences() {
        // Bold + color + reset around content
        assert_eq!(
            strip_ansi("\x1b[1;33mWARNING\x1b[0m: check this"),
            "WARNING: check this"
        );
    }

    #[test]
    fn background_pause_reason_triggers_on_token_threshold() {
        let recent = AiRecentUsageLedger {
            calls: vec![AiRecentUsageCall {
                timestamp: Utc::now().to_rfc3339(),
                subsystem: "intel_queue".to_string(),
                operation: "background_entity_enrichment".to_string(),
                trigger: "calendar_change".to_string(),
                tier: "background".to_string(),
                model: "haiku".to_string(),
                background: true,
                status: "success".to_string(),
                duration_ms: 1000,
                estimated_prompt_tokens: ESTIMATED_DAILY_TOKEN_BUDGET,
                estimated_output_tokens: 0,
                estimated_total_tokens: ESTIMATED_DAILY_TOKEN_BUDGET,
                error: None,
            }],
        };
        let status = build_background_pause_status(&recent, &BackgroundAiGuardState::default());
        assert!(background_pause_reason(&status).is_some());
    }

    #[test]
    fn background_pause_status_ignores_old_calls() {
        let recent = AiRecentUsageLedger {
            calls: vec![AiRecentUsageCall {
                timestamp: (Utc::now() - ChronoDuration::hours(5)).to_rfc3339(),
                subsystem: "intel_queue".to_string(),
                operation: "background_entity_enrichment".to_string(),
                trigger: "calendar_change".to_string(),
                tier: "background".to_string(),
                model: "haiku".to_string(),
                background: true,
                status: "timeout".to_string(),
                duration_ms: 1000,
                estimated_prompt_tokens: 1000,
                estimated_output_tokens: 0,
                estimated_total_tokens: 1000,
                error: None,
            }],
        };
        let status = build_background_pause_status(&recent, &BackgroundAiGuardState::default());
        assert_eq!(status.rolling_4h_tokens, 0);
    }
}
