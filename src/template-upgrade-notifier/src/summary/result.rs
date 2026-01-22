//! Processing result types.

use crate::issues::IssueStatus;
use crate::pull_requests::PrStatus;

/// Result of processing a single repository.
#[derive(Debug, Clone)]
pub enum ProcessingResult {
    /// Processing succeeded.
    Success {
        /// Repository full name.
        repository: String,
        /// Issue creation status.
        issue: IssueStatus,
        /// Optional PR creation status.
        pr: Option<PrStatus>,
    },

    /// Processing was skipped.
    Skipped {
        /// Repository full name.
        repository: String,
        /// Reason for skipping.
        reason: String,
    },

    /// Processing failed.
    Failed {
        /// Repository full name.
        repository: String,
        /// Error message.
        error: String,
    },
}
