//! Issue status types.

use serde::Serialize;

/// Status of an issue creation operation.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum IssueStatus {
    /// Issue not yet created.
    Pending,

    /// Issue successfully created.
    Created {
        /// GitHub issue number.
        number: u64,
        /// GitHub issue URL.
        url: String,
    },

    /// Issue creation skipped.
    Skipped {
        /// Reason for skipping.
        reason: String,
    },

    /// Issue creation failed.
    Failed {
        /// Error message.
        error: String,
    },
}
