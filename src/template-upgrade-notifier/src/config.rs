//! Configuration and migration loading.
//!
//! This module handles parsing metadata.toml files and loading migrations
//! from the filesystem.

use serde::Deserialize;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info, warn};
use url::Url;

/// Errors that can occur during configuration parsing.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Failed to read a file.
    #[error("Failed to read file '{path}': {source}")]
    IoError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse TOML content.
    #[error("Failed to parse metadata.toml in '{path}': {source}")]
    TomlError {
        path: String,
        #[source]
        source: toml::de::Error,
    },

    /// Validation error in metadata.
    #[error("Validation error in '{path}': {message}")]
    ValidationError { path: String, message: String },

    /// Missing required file.
    #[error("Missing required file: {path}")]
    MissingFile { path: String },
}

/// Parsed metadata from a `metadata.toml` file.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MigrationMetadata {
    /// The version string to search for in repositories.
    pub old_string: String,

    /// The version string to upgrade to.
    pub new_string: String,

    /// URL to migration documentation.
    pub migration_guide_link: String,

    /// File name to search for (defaults to "template-version.txt").
    #[serde(default = "default_target_file")]
    pub target_file: String,
}

fn default_target_file() -> String {
    "template-version.txt".to_string()
}

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

    /// URL to migration documentation.
    pub migration_guide_link: String,

    /// File name to search for containing the version string.
    pub target_file: String,

    /// Contents of issue-template.md.
    pub issue_template: String,

    /// Contents of pr-template.md.
    pub pr_template: String,
}

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
pub fn load_migration(path: &Path, migration_id: &str) -> Result<Migration, ConfigError> {
    debug!(path = %path.display(), migration_id, "Loading migration");

    // Load and parse metadata.toml
    let metadata_path = path.join("metadata.toml");
    let metadata_content =
        std::fs::read_to_string(&metadata_path).map_err(|e| ConfigError::IoError {
            path: metadata_path.display().to_string(),
            source: e,
        })?;

    let metadata: MigrationMetadata =
        toml::from_str(&metadata_content).map_err(|e| ConfigError::TomlError {
            path: metadata_path.display().to_string(),
            source: e,
        })?;

    // Validate metadata
    validate_metadata(&metadata, path)?;

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

    Ok(Migration {
        id: migration_id.to_string(),
        old_string: metadata.old_string,
        new_string: metadata.new_string,
        migration_guide_link: metadata.migration_guide_link,
        target_file: metadata.target_file,
        issue_template,
        pr_template,
    })
}

/// Validates migration metadata.
fn validate_metadata(metadata: &MigrationMetadata, path: &Path) -> Result<(), ConfigError> {
    let path_str = path.display().to_string();

    // Check old_string != new_string
    if metadata.old_string == metadata.new_string {
        return Err(ConfigError::ValidationError {
            path: path_str,
            message: "old-string and new-string must be different".to_string(),
        });
    }

    // Check old_string is not empty
    if metadata.old_string.trim().is_empty() {
        return Err(ConfigError::ValidationError {
            path: path_str,
            message: "old-string must not be empty".to_string(),
        });
    }

    // Check new_string is not empty
    if metadata.new_string.trim().is_empty() {
        return Err(ConfigError::ValidationError {
            path: path_str,
            message: "new-string must not be empty".to_string(),
        });
    }

    // Validate URL format
    if Url::parse(&metadata.migration_guide_link).is_err() {
        return Err(ConfigError::ValidationError {
            path: path_str,
            message: format!(
                "migration-guide-link is not a valid URL: {}",
                metadata.migration_guide_link
            ),
        });
    }

    // Validate target_file doesn't contain path separators
    if metadata.target_file.contains('/') || metadata.target_file.contains('\\') {
        return Err(ConfigError::ValidationError {
            path: path_str,
            message: "target-file must not contain path separators".to_string(),
        });
    }

    Ok(())
}

/// Scans a migrations directory and loads all valid migrations.
///
/// The directory structure should be:
/// ```text
/// migrations/
/// ├── template-name/
/// │   └── v1.0.0-to-v1.0.1/
/// │       ├── metadata.toml
/// │       ├── issue-template.md
/// │       └── pr-template.md
/// ```
///
/// # Arguments
///
/// * `migrations_path` - Path to the root migrations directory
///
/// # Returns
///
/// A vector of successfully loaded migrations. Failed migrations are logged
/// as warnings but don't cause the entire operation to fail.
///
/// # Errors
///
/// Returns an error if the migrations directory doesn't exist or can't be read.
pub fn scan_migrations(migrations_path: &Path) -> Result<Vec<Migration>, ConfigError> {
    info!(path = %migrations_path.display(), "Scanning migrations directory");

    if !migrations_path.exists() {
        return Err(ConfigError::MissingFile {
            path: migrations_path.display().to_string(),
        });
    }

    let mut migrations = Vec::new();

    // Walk the directory tree looking for metadata.toml files
    scan_directory_recursive(migrations_path, migrations_path, &mut migrations)?;

    info!(count = migrations.len(), "Loaded migrations");
    Ok(migrations)
}

/// Recursively scans a directory for migration folders.
fn scan_directory_recursive(
    base_path: &Path,
    current_path: &Path,
    migrations: &mut Vec<Migration>,
) -> Result<(), ConfigError> {
    let entries = std::fs::read_dir(current_path).map_err(|e| ConfigError::IoError {
        path: current_path.display().to_string(),
        source: e,
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| ConfigError::IoError {
            path: current_path.display().to_string(),
            source: e,
        })?;

        let path = entry.path();

        if path.is_dir() {
            // Check if this directory contains metadata.toml
            let metadata_path = path.join("metadata.toml");
            if metadata_path.exists() {
                // This is a migration directory
                let migration_id = path
                    .strip_prefix(base_path)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();

                match load_migration(&path, &migration_id) {
                    Ok(migration) => {
                        debug!(id = migration_id, "Loaded migration");
                        migrations.push(migration);
                    }
                    Err(e) => {
                        warn!(path = %path.display(), error = %e, "Failed to load migration");
                    }
                }
            } else {
                // Continue scanning subdirectories
                scan_directory_recursive(base_path, &path, migrations)?;
            }
        }
    }

    Ok(())
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
    fn test_load_valid_migration() {
        let temp = TempDir::new().unwrap();
        create_test_migration(temp.path());

        let migration = load_migration(temp.path(), "test/v1").unwrap();

        assert_eq!(migration.id, "test/v1");
        assert_eq!(migration.old_string, "test:1.0.0");
        assert_eq!(migration.new_string, "test:1.0.1");
        assert_eq!(migration.migration_guide_link, "https://example.com/guide");
        assert_eq!(migration.target_file, "version.txt");
    }

    #[test]
    fn test_load_migration_missing_metadata() {
        let temp = TempDir::new().unwrap();

        let result = load_migration(temp.path(), "test/v1");
        assert!(matches!(result, Err(ConfigError::IoError { .. })));
    }

    #[test]
    fn test_validation_same_old_new() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("metadata.toml"),
            r#"
old-string = "same"
new-string = "same"
migration-guide-link = "https://example.com"
"#,
        )
        .unwrap();
        fs::write(temp.path().join("issue-template.md"), "content").unwrap();
        fs::write(temp.path().join("pr-template.md"), "content").unwrap();

        let result = load_migration(temp.path(), "test");
        assert!(matches!(result, Err(ConfigError::ValidationError { .. })));
    }

    #[test]
    fn test_validation_invalid_url() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("metadata.toml"),
            r#"
old-string = "old"
new-string = "new"
migration-guide-link = "not-a-url"
"#,
        )
        .unwrap();
        fs::write(temp.path().join("issue-template.md"), "content").unwrap();
        fs::write(temp.path().join("pr-template.md"), "content").unwrap();

        let result = load_migration(temp.path(), "test");
        assert!(matches!(result, Err(ConfigError::ValidationError { .. })));
    }

    #[test]
    fn test_scan_migrations() {
        let temp = TempDir::new().unwrap();

        // Create nested migration structure
        let migration_dir = temp.path().join("my-template/v1.0.0-to-v1.0.1");
        fs::create_dir_all(&migration_dir).unwrap();
        create_test_migration(&migration_dir);

        let migrations = scan_migrations(temp.path()).unwrap();

        assert_eq!(migrations.len(), 1);
        assert!(migrations[0].id.contains("my-template"));
    }

    #[test]
    fn test_default_target_file() {
        assert_eq!(default_target_file(), "template-version.txt");
    }
}
