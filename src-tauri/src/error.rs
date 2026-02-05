//! Error types for workflow execution
//!
//! Errors are classified by recoverability:
//! - Retryable: Network issues, timeouts, rate limits
//! - NonRetryable: Configuration errors, missing files
//! - RequiresUserAction: Missing Claude CLI, auth issues

use std::path::PathBuf;
use thiserror::Error;

/// Error types for workflow execution
#[derive(Debug, Error)]
pub enum ExecutionError {
    // Retryable errors
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Operation timed out after {0} seconds")]
    Timeout(u64),

    #[error("API rate limit exceeded")]
    ApiRateLimit,

    // Non-retryable errors
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(PathBuf),

    #[error("Script failed with exit code {code}: {stderr}")]
    ScriptFailed { code: i32, stderr: String },

    #[error("Script not found: {0}")]
    ScriptNotFound(PathBuf),

    #[error("Failed to parse script output: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(String),

    // Requires user action
    #[error("Claude Code CLI not found. Install from https://claude.ai/code")]
    ClaudeCodeNotFound,

    #[error("Claude Code not authenticated. Run 'claude login'")]
    ClaudeCodeNotAuthenticated,

    #[error("Claude subscription limit reached. Try again later")]
    ClaudeSubscriptionLimit,

    #[error("Python not found. Install Python 3.8+")]
    PythonNotFound,
}

impl ExecutionError {
    /// Returns true if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ExecutionError::NetworkError(_)
                | ExecutionError::Timeout(_)
                | ExecutionError::ApiRateLimit
        )
    }

    /// Returns true if this error requires user action to resolve
    pub fn requires_user_action(&self) -> bool {
        matches!(
            self,
            ExecutionError::ClaudeCodeNotFound
                | ExecutionError::ClaudeCodeNotAuthenticated
                | ExecutionError::ClaudeSubscriptionLimit
                | ExecutionError::PythonNotFound
        )
    }

    /// Get a user-friendly recovery suggestion
    pub fn recovery_suggestion(&self) -> &'static str {
        match self {
            ExecutionError::NetworkError(_) => "Check your internet connection and try again.",
            ExecutionError::Timeout(_) => "The operation took too long. Try again.",
            ExecutionError::ApiRateLimit => "Wait a few minutes and try again.",
            ExecutionError::ConfigurationError(_) => {
                "Check your configuration in ~/.daybreak/config.json"
            }
            ExecutionError::WorkspaceNotFound(_) => {
                "Verify your workspace path in ~/.daybreak/config.json"
            }
            ExecutionError::ScriptFailed { .. } => "Check the script logs for details.",
            ExecutionError::ScriptNotFound(_) => "Ensure the required scripts are installed.",
            ExecutionError::ParseError(_) => "Check the file format is correct.",
            ExecutionError::IoError(_) => "Check file permissions and disk space.",
            ExecutionError::ClaudeCodeNotFound => {
                "Install Claude Code from https://claude.ai/code"
            }
            ExecutionError::ClaudeCodeNotAuthenticated => {
                "Run 'claude login' in your terminal to authenticate."
            }
            ExecutionError::ClaudeSubscriptionLimit => {
                "Your Claude subscription limit was reached. Wait or upgrade your plan."
            }
            ExecutionError::PythonNotFound => "Install Python 3.8+ from https://python.org",
        }
    }
}

impl From<std::io::Error> for ExecutionError {
    fn from(err: std::io::Error) -> Self {
        ExecutionError::IoError(err.to_string())
    }
}

/// Serializable error representation for IPC
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowError {
    pub message: String,
    pub error_type: ErrorType,
    pub can_retry: bool,
    pub recovery_suggestion: String,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorType {
    Retryable,
    NonRetryable,
    RequiresUserAction,
}

impl From<&ExecutionError> for WorkflowError {
    fn from(err: &ExecutionError) -> Self {
        let error_type = if err.requires_user_action() {
            ErrorType::RequiresUserAction
        } else if err.is_retryable() {
            ErrorType::Retryable
        } else {
            ErrorType::NonRetryable
        };

        WorkflowError {
            message: err.to_string(),
            error_type,
            can_retry: err.is_retryable(),
            recovery_suggestion: err.recovery_suggestion().to_string(),
        }
    }
}
