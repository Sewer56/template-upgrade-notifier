//! Repository discovery error types.

use thiserror::Error;

/// Errors that can occur during repository discovery.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    /// GitHub API error.
    #[error("GitHub API error: {0}")]
    GitHubError(#[from] octocrab::Error),

    /// Rate limit exceeded.
    #[error("Rate limit exceeded, reset at {reset_at}")]
    RateLimitExceeded { reset_at: u64 },
}
