//! Pull request status types.

use serde::Serialize;

/// Status of a PR creation operation.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PrStatus {
    /// PR not yet created.
    Pending,

    /// PR successfully created.
    Created {
        /// GitHub PR number.
        number: u64,
        /// GitHub PR URL.
        url: String,
    },

    /// PR creation skipped.
    Skipped {
        /// Reason for skipping.
        reason: String,
    },

    /// PR creation failed.
    Failed {
        /// Error message.
        error: String,
    },

    /// Timed out during PR generation.
    TimedOut,
}

impl PrStatus {
    /// Returns the status as a string for template rendering.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Created { .. } => "created",
            Self::Skipped { .. } => "skipped",
            Self::Failed { .. } => "failed",
            Self::TimedOut => "failed",
        }
    }

    /// Returns the PR URL if created.
    #[must_use]
    pub fn url(&self) -> Option<&str> {
        match self {
            Self::Created { url, .. } => Some(url),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_pr_status_to_string() {
        assert_eq!(PrStatus::Pending.as_str(), "pending");
        assert_eq!(
            PrStatus::Created {
                number: 1,
                url: "https://example.com".to_string()
            }
            .as_str(),
            "created"
        );
        assert_eq!(
            PrStatus::Skipped {
                reason: "test".to_string()
            }
            .as_str(),
            "skipped"
        );
        assert_eq!(
            PrStatus::Failed {
                error: "test".to_string()
            }
            .as_str(),
            "failed"
        );
        assert_eq!(PrStatus::TimedOut.as_str(), "failed");
    }
}
