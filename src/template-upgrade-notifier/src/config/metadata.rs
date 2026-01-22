//! Migration metadata deserialization and validation.

use crate::config::ConfigError;
use handlebars::Handlebars;
use serde::Deserialize;
use std::path::Path;

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

    /// Handlebars format for issue titles.
    ///
    /// Available variables: `old_string`, `new_string`, `id`, `target_file`, `migration_guide_link`
    #[serde(default = "default_issue_title_format")]
    pub issue_title_format: String,

    /// Handlebars format for PR titles.
    ///
    /// Available variables: `old_string`, `new_string`, `id`, `target_file`, `migration_guide_link`
    #[serde(default = "default_pr_title_format")]
    pub pr_title_format: String,

    /// Handlebars format for branch names.
    ///
    /// Available variables: `old_string`, `new_string`, `id`, `target_file`, `migration_guide_link`
    ///
    /// The rendered value must be a valid git branch name.
    #[serde(default = "default_branch_name_format")]
    pub branch_name_format: String,

    /// Handlebars format for commit titles.
    ///
    /// Available variables: `old_string`, `new_string`, `id`, `target_file`, `migration_guide_link`
    #[serde(default = "default_commit_title_format")]
    pub commit_title_format: String,
}

impl MigrationMetadata {
    /// Parses metadata from TOML content.
    ///
    /// # Arguments
    ///
    /// * `content` - TOML string to parse
    /// * `path` - Path used for error reporting
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::TomlError`] if parsing fails.
    pub fn parse(content: &str, path: &Path) -> Result<Self, ConfigError> {
        toml::from_str(content).map_err(|e| ConfigError::TomlError {
            path: path.display().to_string(),
            source: e,
        })
    }

    /// Loads metadata from a directory containing `metadata.toml`.
    ///
    /// # Arguments
    ///
    /// * `dir` - Directory containing the `metadata.toml` file
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::IoError`] if the file cannot be read,
    /// or [`ConfigError::TomlError`] if parsing fails.
    pub fn load(dir: &Path) -> Result<Self, ConfigError> {
        let metadata_path = dir.join("metadata.toml");
        let content =
            std::fs::read_to_string(&metadata_path).map_err(|e| ConfigError::IoError {
                path: metadata_path.display().to_string(),
                source: e,
            })?;
        Self::parse(&content, &metadata_path)
    }

    /// Validates the metadata fields.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::ValidationError`] if:
    /// - `old_string` equals `new_string`
    /// - `old_string` is empty
    /// - `new_string` is empty
    /// - `migration_guide_link` is not a valid URL (if present)
    /// - `target_file` contains path separators
    pub fn validate(&self, path: &Path) -> Result<(), ConfigError> {
        let path_str = path.display().to_string();

        // Check old_string != new_string
        if self.old_string == self.new_string {
            return Err(ConfigError::ValidationError {
                path: path_str,
                message: "old-string and new-string must be different".to_string(),
            });
        }

        // Check old_string is not empty
        if self.old_string.trim().is_empty() {
            return Err(ConfigError::ValidationError {
                path: path_str,
                message: "old-string must not be empty".to_string(),
            });
        }

        // Check new_string is not empty
        if self.new_string.trim().is_empty() {
            return Err(ConfigError::ValidationError {
                path: path_str,
                message: "new-string must not be empty".to_string(),
            });
        }

        // Validate URL format if provided
        if let Some(ref link) = self.migration_guide_link {
            if url::Url::parse(link).is_err() {
                return Err(ConfigError::ValidationError {
                    path: path_str,
                    message: format!("migration-guide-link is not a valid URL: {link}"),
                });
            }
        }

        // Validate target_file doesn't contain path separators
        if self.target_file.contains('/') || self.target_file.contains('\\') {
            return Err(ConfigError::ValidationError {
                path: path_str.clone(),
                message: "target-file must not contain path separators".to_string(),
            });
        }

        // Validate format templates are valid Handlebars
        self.validate_format_template(&path_str, "issue-title-format", &self.issue_title_format)?;
        self.validate_format_template(&path_str, "pr-title-format", &self.pr_title_format)?;
        self.validate_format_template(&path_str, "branch-name-format", &self.branch_name_format)?;
        self.validate_format_template(&path_str, "commit-title-format", &self.commit_title_format)?;

        Ok(())
    }

    /// Validates that a format string is a valid Handlebars template.
    fn validate_format_template(
        &self,
        path: &str,
        field_name: &str,
        template: &str,
    ) -> Result<(), ConfigError> {
        let hbs = Handlebars::new();
        hbs.render_template(template, &serde_json::json!({}))
            .map_err(|e| ConfigError::ValidationError {
                path: path.to_string(),
                message: format!("{field_name} is not a valid Handlebars template: {e}"),
            })?;
        Ok(())
    }
}

pub(crate) fn default_target_file() -> String {
    "template-version.txt".to_string()
}

/// Returns the default issue title format.
#[must_use]
pub fn default_issue_title_format() -> String {
    "Template Upgrade Available: {{old_string}} -> {{new_string}}".to_string()
}

/// Returns the default PR title format.
#[must_use]
pub fn default_pr_title_format() -> String {
    "Template Upgrade: {{old_string}} -> {{new_string}}".to_string()
}

/// Returns the default branch name format.
#[must_use]
pub fn default_branch_name_format() -> String {
    "template-upgrade/{{id}}".to_string()
}

/// Returns the default commit title format.
#[must_use]
pub fn default_commit_title_format() -> String {
    "chore: upgrade {{old_string}} -> {{new_string}}".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_same_old_new() {
        let metadata = MigrationMetadata::parse(
            r#"
old-string = "same"
new-string = "same"
"#,
            Path::new("test"),
        )
        .unwrap();

        let result = metadata.validate(Path::new("test"));
        assert!(matches!(result, Err(ConfigError::ValidationError { .. })));
    }

    #[test]
    fn validation_empty_old_string() {
        let metadata = MigrationMetadata::parse(
            r#"
old-string = ""
new-string = "new"
"#,
            Path::new("test"),
        )
        .unwrap();

        let result = metadata.validate(Path::new("test"));
        assert!(matches!(result, Err(ConfigError::ValidationError { .. })));
    }

    #[test]
    fn validation_whitespace_old_string() {
        let metadata = MigrationMetadata::parse(
            r#"
old-string = "   "
new-string = "new"
"#,
            Path::new("test"),
        )
        .unwrap();

        let result = metadata.validate(Path::new("test"));
        assert!(matches!(result, Err(ConfigError::ValidationError { .. })));
    }

    #[test]
    fn validation_empty_new_string() {
        let metadata = MigrationMetadata::parse(
            r#"
old-string = "old"
new-string = ""
"#,
            Path::new("test"),
        )
        .unwrap();

        let result = metadata.validate(Path::new("test"));
        assert!(matches!(result, Err(ConfigError::ValidationError { .. })));
    }

    #[test]
    fn validation_whitespace_new_string() {
        let metadata = MigrationMetadata::parse(
            r#"
old-string = "old"
new-string = "   "
"#,
            Path::new("test"),
        )
        .unwrap();

        let result = metadata.validate(Path::new("test"));
        assert!(matches!(result, Err(ConfigError::ValidationError { .. })));
    }

    #[test]
    fn validation_invalid_url() {
        let metadata = MigrationMetadata::parse(
            r#"
old-string = "old"
new-string = "new"
migration-guide-link = "not-a-url"
"#,
            Path::new("test"),
        )
        .unwrap();

        let result = metadata.validate(Path::new("test"));
        assert!(matches!(result, Err(ConfigError::ValidationError { .. })));
    }

    #[test]
    fn validation_valid_metadata() {
        let metadata = MigrationMetadata::parse(
            r#"
old-string = "test:1.0.0"
new-string = "test:1.0.1"
migration-guide-link = "https://example.com/guide"
target-file = "version.txt"
"#,
            Path::new("test"),
        )
        .unwrap();

        let result = metadata.validate(Path::new("test"));
        assert!(result.is_ok());
    }

    #[test]
    fn default_target_file_returns_correct_value() {
        assert_eq!(default_target_file(), "template-version.txt");
    }

    #[test]
    fn default_format_values() {
        let metadata = MigrationMetadata::parse(
            r#"
old-string = "old"
new-string = "new"
"#,
            Path::new("test"),
        )
        .unwrap();

        assert_eq!(
            metadata.issue_title_format,
            "Template Upgrade Available: {{old_string}} -> {{new_string}}"
        );
        assert_eq!(
            metadata.pr_title_format,
            "Template Upgrade: {{old_string}} -> {{new_string}}"
        );
        assert_eq!(metadata.branch_name_format, "template-upgrade/{{id}}");
        assert_eq!(
            metadata.commit_title_format,
            "chore: upgrade {{old_string}} -> {{new_string}}"
        );
    }

    #[test]
    fn custom_format_values() {
        let metadata = MigrationMetadata::parse(
            r#"
old-string = "old"
new-string = "new"
issue-title-format = "Custom Issue: {{id}}"
pr-title-format = "Custom PR: {{old_string}}"
branch-name-format = "upgrade/{{id}}"
commit-title-format = "feat: {{new_string}}"
"#,
            Path::new("test"),
        )
        .unwrap();

        assert_eq!(metadata.issue_title_format, "Custom Issue: {{id}}");
        assert_eq!(metadata.pr_title_format, "Custom PR: {{old_string}}");
        assert_eq!(metadata.branch_name_format, "upgrade/{{id}}");
        assert_eq!(metadata.commit_title_format, "feat: {{new_string}}");

        // Should validate successfully
        assert!(metadata.validate(Path::new("test")).is_ok());
    }

    #[test]
    fn validation_invalid_issue_title_format() {
        let metadata = MigrationMetadata::parse(
            r#"
old-string = "old"
new-string = "new"
issue-title-format = "Unclosed {{bracket"
"#,
            Path::new("test"),
        )
        .unwrap();

        let result = metadata.validate(Path::new("test"));
        assert!(matches!(result, Err(ConfigError::ValidationError { .. })));
    }

    #[test]
    fn validation_invalid_branch_name_format() {
        let metadata = MigrationMetadata::parse(
            r#"
old-string = "old"
new-string = "new"
branch-name-format = "{{#if unclosed}}"
"#,
            Path::new("test"),
        )
        .unwrap();

        let result = metadata.validate(Path::new("test"));
        assert!(matches!(result, Err(ConfigError::ValidationError { .. })));
    }
}
