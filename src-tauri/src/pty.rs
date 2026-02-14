//! PTY Manager for Claude Code subprocess management
//!
//! Spawns Claude Code via pseudo-terminal with timeout handling.
//! This is necessary because Claude Code expects an interactive terminal.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{mpsc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

use crate::error::ExecutionError;
use crate::types::AiModelConfig;

/// Cached resolved path to the `claude` binary.
/// Resolved once per process lifetime via `resolve_claude_binary()`.
static CLAUDE_BINARY: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Resolve the absolute path to the `claude` CLI binary.
///
/// macOS apps launched from Finder/DMG don't inherit the user's shell PATH,
/// so `which claude` fails even when claude is installed. This function checks
/// common install locations as a fallback.
fn resolve_claude_binary() -> Option<&'static PathBuf> {
    CLAUDE_BINARY
        .get_or_init(|| {
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
                home.join(".local/bin/claude"),        // npm global (default)
                home.join(".npm/bin/claude"),           // npm alternate
                home.join(".nvm/current/bin/claude"),   // nvm
                PathBuf::from("/usr/local/bin/claude"), // Homebrew / manual
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
        })
        .as_ref()
}

/// Default timeout for AI enrichment phase (5 minutes)
pub const DEFAULT_CLAUDE_TIMEOUT_SECS: u64 = 300;
/// Timeout for Claude CLI auth checks.
const CLAUDE_AUTH_CHECK_TIMEOUT_SECS: u64 = 3;

/// Model tier for AI operations (I174).
///
/// Maps to configured model names via `AiModelConfig`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    /// Intelligence, briefing, week narrative — needs synthesis
    Synthesis,
    /// Emails, preps — structured extraction
    Extraction,
    /// Inbox classification, file summaries — mechanical tasks
    Mechanical,
}

/// PTY Manager for spawning Claude Code
pub struct PtyManager {
    timeout_secs: u64,
    model: Option<String>,
    nice_priority: Option<i32>,
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

    /// Create a PtyManager configured for a specific model tier.
    pub fn for_tier(tier: ModelTier, config: &AiModelConfig) -> Self {
        let model = match tier {
            ModelTier::Synthesis => &config.synthesis,
            ModelTier::Extraction => &config.extraction,
            ModelTier::Mechanical => &config.mechanical,
        };
        Self::new().with_model(model.clone())
    }

    /// Check if Claude Code CLI is available
    pub fn is_claude_available() -> bool {
        resolve_claude_binary().is_some()
    }

    /// Check if Claude Code is authenticated
    ///
    /// Uses `--print` which actually exercises auth. If unauthenticated,
    /// Claude Code exits with a non-zero status code.
    pub fn is_claude_authenticated() -> Result<bool, ExecutionError> {
        use std::process::Stdio;

        let claude_path = resolve_claude_binary()
            .ok_or(ExecutionError::ClaudeCodeNotFound)?;

        let mut child = Command::new(claude_path)
            .args(["--print", "hello"])
            .env("TERM", "dumb")
            .env_remove("CLAUDECODE")
            .env_remove("CLAUDE_CODE_SSE_PORT")
            .env_remove("CLAUDE_CODE_ENTRYPOINT")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| ExecutionError::ClaudeCodeNotFound)?;

        let deadline = Duration::from_secs(CLAUDE_AUTH_CHECK_TIMEOUT_SECS);
        let started_at = Instant::now();

        loop {
            match child.try_wait() {
                Ok(Some(status)) => return Ok(status.success()),
                Ok(None) => {
                    if started_at.elapsed() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        log::warn!(
                            "Claude auth check timed out after {}s",
                            CLAUDE_AUTH_CHECK_TIMEOUT_SECS
                        );
                        return Ok(false);
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    return Err(ExecutionError::IoError(format!(
                        "Claude auth check failed: {}",
                        e
                    )));
                }
            }
        }
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
        let claude_path = resolve_claude_binary()
            .ok_or(ExecutionError::ClaudeCodeNotFound)?;
        let claude_str = claude_path.to_string_lossy();

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
        for key in ["CLAUDECODE", "CLAUDE_CODE_SSE_PORT", "CLAUDE_CODE_ENTRYPOINT"] {
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
        let raw_output = rx
            .recv_timeout(timeout)
            .map_err(|_| ExecutionError::Timeout(self.timeout_secs))?;

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
            return Err(ExecutionError::ClaudeCodeNotAuthenticated);
        }

        if is_model_unavailable_output(&output) {
            let first_line = output.lines().next().unwrap_or("Model unavailable");
            return Err(ExecutionError::ConfigurationError(format!(
                "model_unavailable: {}",
                first_line
            )));
        }

        if output.contains("rate limit") || output.contains("too many requests") {
            return Err(ExecutionError::ApiRateLimit);
        }

        if output.contains("subscription") && output.contains("limit") {
            return Err(ExecutionError::ClaudeSubscriptionLimit);
        }

        if output.contains("cannot be launched inside another Claude Code session") {
            return Err(ExecutionError::ConfigurationError(
                "Nested Claude Code session detected. CLAUDECODE env var leaked to subprocess."
                    .to_string(),
            ));
        }

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
    use super::{is_model_unavailable_output, strip_ansi};

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
        assert_eq!(
            strip_ansi("\x1b[1mENRICHMENT:e1\x1b[0m"),
            "ENRICHMENT:e1"
        );
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
}
