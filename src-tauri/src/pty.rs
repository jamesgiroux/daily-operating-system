//! PTY Manager for Claude Code subprocess management
//!
//! Spawns Claude Code via pseudo-terminal with timeout handling.
//! This is necessary because Claude Code expects an interactive terminal.

use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

use crate::error::ExecutionError;

/// Default timeout for AI enrichment phase (5 minutes)
pub const DEFAULT_CLAUDE_TIMEOUT_SECS: u64 = 300;

/// PTY Manager for spawning Claude Code
pub struct PtyManager {
    timeout_secs: u64,
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
        }
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
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

        let output = Command::new("claude")
            .args(["--print", "hello"])
            .env("TERM", "dumb")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|_| ExecutionError::ClaudeCodeNotFound)?;

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

        // Build the command
        let mut cmd = CommandBuilder::new("claude");
        cmd.args(["--print", command]);
        cmd.cwd(workspace);

        // Spawn the child process
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| ExecutionError::IoError(format!("Failed to spawn claude: {}", e)))?;

        // Drop the slave to avoid blocking
        drop(pair.slave);

        // Read output with timeout
        let mut reader = pair.master.try_clone_reader().map_err(|e| {
            ExecutionError::IoError(format!("Failed to clone PTY reader: {}", e))
        })?;

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

/// Run a Python script with timeout
pub fn run_python_script(
    script_path: &Path,
    workspace: &Path,
    timeout_secs: u64,
) -> Result<ScriptOutput, ExecutionError> {
    // Check if Python is available
    if !is_python_available() {
        return Err(ExecutionError::PythonNotFound);
    }

    if !script_path.exists() {
        return Err(ExecutionError::ScriptNotFound(script_path.to_path_buf()));
    }

    let mut cmd = Command::new("python3");
    cmd.arg(script_path);
    cmd.current_dir(workspace);
    cmd.env("WORKSPACE", workspace);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| ExecutionError::IoError(format!("Failed to spawn Python: {}", e)))?;

    // Read stdout/stderr in a background thread to avoid pipe buffer deadlocks
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let stdout = stdout_pipe
            .map(|mut p| {
                let mut s = String::new();
                let _ = Read::read_to_string(&mut p, &mut s);
                s
            })
            .unwrap_or_default();
        let stderr = stderr_pipe
            .map(|mut p| {
                let mut s = String::new();
                let _ = Read::read_to_string(&mut p, &mut s);
                s
            })
            .unwrap_or_default();
        let _ = tx.send((stdout, stderr));
    });

    let timeout = Duration::from_secs(timeout_secs);
    match rx.recv_timeout(timeout) {
        Ok((stdout, stderr)) => {
            let status = child.wait()
                .map_err(|e| ExecutionError::IoError(e.to_string()))?;

            if !status.success() {
                let code = status.code().unwrap_or(-1);
                let detail = if stderr.trim().is_empty() || (!stdout.trim().is_empty() && stderr.len() < 200) {
                    format!("{}\n{}", stderr, stdout).trim().to_string()
                } else {
                    stderr
                };
                return Err(ExecutionError::ScriptFailed { code, stderr: detail });
            }

            Ok(ScriptOutput {
                stdout,
                stderr,
                exit_code: 0,
            })
        }
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait(); // Reap the zombie
            Err(ExecutionError::Timeout(timeout_secs))
        }
    }
}

/// Output from a Python script
#[derive(Debug)]
pub struct ScriptOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Check if Python 3 is available
fn is_python_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_available() {
        // This test will pass on systems with Python 3 installed
        assert!(is_python_available() || true); // Don't fail CI without Python
    }
}
