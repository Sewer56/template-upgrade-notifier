//! Migration metadata deserialization.

use serde::Deserialize;

/// Parsed metadata from a `metadata.toml` file.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MigrationMetadata {
    /// The version string to search for in repositories.
    pub old_string: String,

    /// The version string to upgrade to.
    pub new_string: String,

    /// URL to migration documentation (optional).
    pub migration_guide_link: Option<String>,

    /// File name to search for (defaults to "template-version.txt").
    #[serde(default = "default_target_file")]
    pub target_file: String,
}

pub(crate) fn default_target_file() -> String {
    "template-version.txt".to_string()
}
