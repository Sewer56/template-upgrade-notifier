//! Migration metadata deserialization and validation.

use crate::config::ConfigError;
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
}

impl MigrationMetadata {
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
                path: path_str,
                message: "target-file must not contain path separators".to_string(),
            });
        }

        Ok(())
    }
}

pub(crate) fn default_target_file() -> String {
    "template-version.txt".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn parse_metadata(toml_content: &str) -> MigrationMetadata {
        toml::from_str(toml_content).unwrap()
    }

    #[test]
    fn test_validation_same_old_new() {
        let temp = TempDir::new().unwrap();
        let metadata = parse_metadata(
            r#"
old-string = "same"
new-string = "same"
"#,
        );

        let result = metadata.validate(temp.path());
        assert!(matches!(result, Err(ConfigError::ValidationError { .. })));
    }

    #[test]
    fn test_validation_invalid_url() {
        let temp = TempDir::new().unwrap();
        let metadata = parse_metadata(
            r#"
old-string = "old"
new-string = "new"
migration-guide-link = "not-a-url"
"#,
        );

        let result = metadata.validate(temp.path());
        assert!(matches!(result, Err(ConfigError::ValidationError { .. })));
    }

    #[test]
    fn test_validation_valid_metadata() {
        let temp = TempDir::new().unwrap();
        let metadata = parse_metadata(
            r#"
old-string = "test:1.0.0"
new-string = "test:1.0.1"
migration-guide-link = "https://example.com/guide"
target-file = "version.txt"
"#,
        );

        let result = metadata.validate(temp.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_target_file() {
        assert_eq!(default_target_file(), "template-version.txt");
    }
}
