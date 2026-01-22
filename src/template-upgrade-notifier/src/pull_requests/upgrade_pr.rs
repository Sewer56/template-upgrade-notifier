//! Upgrade pull request information.

/// A pull request created to apply a migration.
#[derive(Debug, Clone)]
pub struct UpgradePR {
    /// Target repository.
    pub repository: crate::discovery::DiscoveredRepository,

    /// Reference to source migration.
    pub migration_id: String,

    /// Feature branch name.
    pub branch_name: String,

    /// PR title.
    pub title: String,

    /// Rendered PR body.
    pub body: String,

    /// Creation status.
    pub status: super::PrStatus,
}
