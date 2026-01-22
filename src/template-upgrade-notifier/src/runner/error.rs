//! Runner error types.

/// Errors that can occur while running the notifier.
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    /// Configuration and migration loading errors.
    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),

    /// GitHub API client initialization errors.
    #[error(transparent)]
    Octocrab(#[from] octocrab::Error),
}
