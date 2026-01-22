//! Issue creation error types.

use thiserror::Error;

/// Errors that can occur during issue operations.
#[derive(Debug, Error)]
pub enum IssueError {
    /// GitHub API error.
    #[error("GitHub API error: {0}")]
    GitHubError(#[from] octocrab::Error),

    /// Permission denied.
    #[error("Permission denied: no write access to {owner}/{repo}")]
    PermissionDenied { owner: String, repo: String },

    /// Rate limit exceeded.
    #[error("Rate limit exceeded, reset at {reset_at}")]
    RateLimitExceeded { reset_at: u64 },

    /// Template rendering error.
    #[error("Template rendering error: {0}")]
    TemplateError(String),
}
