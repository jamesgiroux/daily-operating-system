//! PTY Manager for Claude Code subprocess management
//!
//! Spawns Claude Code via pseudo-terminal with timeout handling.
//! This is necessary because Claude Code expects an interactive terminal.

use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

use crate::error::ExecutionError;
use crate::types::AiModelConfig;

/// Default timeout for AI enrichment phase (5 minutes)
pub const DEFAULT_CLAUDE_TIMEOUT_SECS: u64 = 300;
/// Timeout for CLI auth checks.
const CLAUDE_AUTH_CHECK_TIMEOUT_SECS: u64 = 8;

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
        Command::new("which")
            .arg("claude")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Check if Claude Code is authenticated
    ///
    /// Uses `--print` which actually exercises auth. If unauthenticated,
    /// Claude Code exits with a non-zero status code.
    pub fn is_claude_authenticated() -> Result<bool, ExecutionError> {
        use std::process::Stdio;

        let mut child = Command::new("claude")
            .args(["--print", "hello"])
            .env("TERM", "dumb")
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
        if !Self::is_claude_available() {
            return Err(ExecutionError::ClaudeCodeNotFound);
        }

        let pty_system = NativePtySystem::default();

        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| ExecutionError::IoError(format!("Failed to open PTY: {}", e)))?;

        // Build the command, optionally wrapped in `nice` for CPU priority (I173)
        let mut cmd = if let Some(priority) = self.nice_priority {
            let mut c = CommandBuilder::new("nice");
            let prio_str = priority.to_string();
            c.args(["-n", &prio_str, "claude"]);
            if let Some(ref model) = self.model {
                c.args(["--model", model, "--print", command]);
            } else {
                c.args(["--print", command]);
            }
            c
        } else {
            let mut c = CommandBuilder::new("claude");
            if let Some(ref model) = self.model {
                c.args(["--model", model, "--print", command]);
            } else {
                c.args(["--print", command]);
            }
            c
        };
        cmd.cwd(workspace);

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
        let output = rx
            .recv_timeout(timeout)
            .map_err(|_| ExecutionError::Timeout(self.timeout_secs))?;

        // Check exit status
        // Note: We can't easily get exit status from PTY child, so we rely on output
        // Claude Code typically exits 0 on success

        // Check for known error patterns in output
        if output.contains("not authenticated")
            || output.contains("please login")
            || output.contains("login required")
        {
            return Err(ExecutionError::ClaudeCodeNotAuthenticated);
        }

        if output.contains("rate limit") || output.contains("too many requests") {
            return Err(ExecutionError::ApiRateLimit);
        }

        if output.contains("subscription") && output.contains("limit") {
            return Err(ExecutionError::ClaudeSubscriptionLimit);
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
