//! Complete migration definition.

/// A complete migration definition loaded from a migrations folder.
///
/// Combines [`crate::config::MigrationMetadata`] with template contents and a derived ID.
#[derive(Debug, Clone)]
pub struct Migration {
    /// Unique identifier derived from folder path (e.g., "my-template/v1.0.0-to-v1.0.1").
    pub id: String,

    /// The version string to search for.
    pub old_string: String,

    /// The version string to upgrade to.
    pub new_string: String,

    /// URL to migration documentation (optional).
    pub migration_guide_link: Option<String>,

    /// File name to search for containing the version string.
    pub target_file: String,

    /// Contents of issue-template.md.
    pub issue_template: String,

    /// Contents of pr-template.md.
    pub pr_template: String,
}
