//! Upgrade issue information.

/// A GitHub issue created in a discovered repository.
#[derive(Debug, Clone)]
pub struct UpgradeIssue {
    /// Target repository.
    pub repository: crate::discovery::DiscoveredRepository,

    /// Reference to source migration.
    pub migration_id: String,

    /// Issue title.
    pub title: String,

    /// Rendered issue body.
    pub body: String,

    /// Creation status.
    pub status: super::IssueStatus,
}
