//! Configuration and migration loading.
//!
//! This module handles parsing metadata.toml files and loading migrations
//! from the filesystem.

mod error;
mod metadata;
mod migration;

pub use error::ConfigError;
pub use metadata::{
    default_branch_name_format, default_commit_title_format, default_issue_title_format,
    default_pr_title_format, MigrationMetadata,
};
pub use migration::Migration;

use std::path::Path;
use tracing::{debug, info, warn};

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

                match Migration::load(&path, &migration_id) {
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
    fn can_scan_migrations() {
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
    fn scan_migrations_missing_directory() {
        let temp = TempDir::new().unwrap();
        let missing_path = temp.path().join("nonexistent");

        let result = scan_migrations(&missing_path);
        assert!(matches!(result, Err(ConfigError::MissingFile { .. })));
    }

    #[test]
    fn scan_migrations_empty_directory() {
        let temp = TempDir::new().unwrap();

        let migrations = scan_migrations(temp.path()).unwrap();
        assert!(migrations.is_empty());
    }

    #[test]
    fn scan_migrations_multiple() {
        let temp = TempDir::new().unwrap();

        // Create two migrations
        let migration1 = temp.path().join("template-a/v1-to-v2");
        let migration2 = temp.path().join("template-b/v2-to-v3");

        fs::create_dir_all(&migration1).unwrap();
        fs::create_dir_all(&migration2).unwrap();

        create_test_migration(&migration1);
        create_test_migration(&migration2);

        let migrations = scan_migrations(temp.path()).unwrap();
        assert_eq!(migrations.len(), 2);
    }
}
