//! Complete migration definition and loading.

use crate::config::{ConfigError, MigrationMetadata};
use std::path::Path;
use tracing::debug;

/// A complete migration definition loaded from a migrations folder.
///
/// Combines [`MigrationMetadata`] with template contents and a derived ID.
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

    /// Handlebars format for issue titles.
    pub issue_title_format: String,

    /// Handlebars format for PR titles.
    pub pr_title_format: String,

    /// Handlebars format for branch names.
    pub branch_name_format: String,

    /// Handlebars format for commit titles.
    pub commit_title_format: String,
}

impl Migration {
    /// Loads a single migration from a directory.
    ///
    /// The directory must contain:
    /// - `metadata.toml` - Migration configuration
    /// - `issue-template.md` - Issue body template
    /// - `pr-template.md` - PR body template
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the migration directory
    /// * `migration_id` - Unique identifier for this migration (typically derived from path)
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if files are missing, invalid, or fail validation.
    pub fn load(path: &Path, migration_id: &str) -> Result<Self, ConfigError> {
        debug!(path = %path.display(), migration_id, "Loading migration");

        // Load and parse metadata.toml
        let metadata = MigrationMetadata::load(path)?;

        // Validate metadata
        metadata.validate(path)?;

        // Load issue template
        let issue_template_path = path.join("issue-template.md");
        let issue_template =
            std::fs::read_to_string(&issue_template_path).map_err(|e| ConfigError::IoError {
                path: issue_template_path.display().to_string(),
                source: e,
            })?;

        if issue_template.trim().is_empty() {
            return Err(ConfigError::ValidationError {
                path: issue_template_path.display().to_string(),
                message: "issue-template.md is empty".to_string(),
            });
        }

        // Load PR template
        let pr_template_path = path.join("pr-template.md");
        let pr_template =
            std::fs::read_to_string(&pr_template_path).map_err(|e| ConfigError::IoError {
                path: pr_template_path.display().to_string(),
                source: e,
            })?;

        if pr_template.trim().is_empty() {
            return Err(ConfigError::ValidationError {
                path: pr_template_path.display().to_string(),
                message: "pr-template.md is empty".to_string(),
            });
        }

        Ok(Self {
            id: migration_id.to_string(),
            old_string: metadata.old_string,
            new_string: metadata.new_string,
            migration_guide_link: metadata.migration_guide_link,
            target_file: metadata.target_file,
            issue_template,
            pr_template,
            issue_title_format: metadata.issue_title_format,
            pr_title_format: metadata.pr_title_format,
            branch_name_format: metadata.branch_name_format,
            commit_title_format: metadata.commit_title_format,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_migration(dir: &Path) {
        fs::write(
            dir.join("metadata.toml"),
            r#"
old-string = "test:1.0.0"
new-string = "test:1.0.1"
migration-guide-link = "https://example.com/guide"
target-file = "version.txt"
"#,
        )
        .unwrap();

        fs::write(
            dir.join("issue-template.md"),
            "Issue: {{old_string}} -> {{new_string}}",
        )
        .unwrap();
        fs::write(
            dir.join("pr-template.md"),
            "PR: {{old_string}} -> {{new_string}}",
        )
        .unwrap();
    }

    #[test]
    fn load_valid_migration() {
        let temp = TempDir::new().unwrap();
        create_test_migration(temp.path());

        let migration = Migration::load(temp.path(), "test/v1").unwrap();

        assert_eq!(migration.id, "test/v1");
        assert_eq!(migration.old_string, "test:1.0.0");
        assert_eq!(migration.new_string, "test:1.0.1");
        assert_eq!(
            migration.migration_guide_link,
            Some("https://example.com/guide".to_string())
        );
        assert_eq!(migration.target_file, "version.txt");
    }

    #[test]
    fn load_migration_missing_metadata() {
        let temp = TempDir::new().unwrap();

        let result = Migration::load(temp.path(), "test/v1");
        assert!(matches!(result, Err(ConfigError::IoError { .. })));
    }

    #[test]
    fn load_migration_without_guide_link() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("metadata.toml"),
            r#"
old-string = "test:1.0.0"
new-string = "test:1.0.1"
"#,
        )
        .unwrap();
        fs::write(temp.path().join("issue-template.md"), "content").unwrap();
        fs::write(temp.path().join("pr-template.md"), "content").unwrap();

        let migration = Migration::load(temp.path(), "test/v1").unwrap();
        assert_eq!(migration.migration_guide_link, None);
    }

    #[test]
    fn load_migration_with_default_formats() {
        let temp = TempDir::new().unwrap();
        create_test_migration(temp.path());

        let migration = Migration::load(temp.path(), "test/v1").unwrap();

        // Should have default format values
        assert_eq!(
            migration.issue_title_format,
            "Template Upgrade Available: {{old_string}} -> {{new_string}}"
        );
        assert_eq!(
            migration.pr_title_format,
            "Template Upgrade: {{old_string}} -> {{new_string}}"
        );
        assert_eq!(migration.branch_name_format, "template-upgrade/{{id}}");
        assert_eq!(
            migration.commit_title_format,
            "chore: upgrade {{old_string}} -> {{new_string}}"
        );
    }

    #[test]
    fn load_migration_with_custom_formats() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("metadata.toml"),
            r#"
old-string = "test:1.0.0"
new-string = "test:1.0.1"
issue-title-format = "Custom: {{id}}"
pr-title-format = "PR: {{old_string}} to {{new_string}}"
branch-name-format = "upgrade/{{id}}"
commit-title-format = "feat: upgrade {{old_string}}"
"#,
        )
        .unwrap();
        fs::write(temp.path().join("issue-template.md"), "content").unwrap();
        fs::write(temp.path().join("pr-template.md"), "content").unwrap();

        let migration = Migration::load(temp.path(), "test/v1").unwrap();

        assert_eq!(migration.issue_title_format, "Custom: {{id}}");
        assert_eq!(
            migration.pr_title_format,
            "PR: {{old_string}} to {{new_string}}"
        );
        assert_eq!(migration.branch_name_format, "upgrade/{{id}}");
        assert_eq!(
            migration.commit_title_format,
            "feat: upgrade {{old_string}}"
        );
    }
}
