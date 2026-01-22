//! Pull request error types.

use thiserror::Error;

/// Errors that can occur during PR operations.
#[derive(Debug, Error)]
pub enum PrError {
    /// GitHub API error.
    #[error("GitHub API error: {0}")]
    GitHubError(#[from] octocrab::Error),

    /// Clone failed.
    #[error("Failed to clone repository: {message}")]
    CloneFailed { message: String },

    /// LLM invocation failed.
    #[error("LLM invocation failed: {message}")]
    LlmFailed { message: String },

    /// LLM timed out.
    #[error("LLM timed out after {timeout_secs} seconds")]
    Timeout { timeout_secs: u64 },

    /// Push failed.
    #[error("Failed to push changes: {message}")]
    PushFailed { message: String },

    /// No changes were made.
    #[error("No changes were made")]
    NoChanges,
}
