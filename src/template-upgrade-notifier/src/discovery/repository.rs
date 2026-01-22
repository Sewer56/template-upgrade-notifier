//! Discovered repository information.

use serde::Serialize;

/// A repository discovered to contain an outdated template version.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredRepository {
    /// Repository owner (user or organization).
    pub owner: String,

    /// Repository name.
    pub name: String,

    /// Full repository name in "owner/name" format.
    pub full_name: String,

    /// Path to the file containing the match.
    pub file_path: String,

    /// GitHub URL to the matched file.
    pub file_url: String,

    /// Default branch name (e.g., "main").
    pub default_branch: String,
}
