use std::path::PathBuf;

use template_upgrade_notifier::{scan_migrations, ConfigError, Migration};

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/migrations")
}

#[test]
fn load_migration_from_fixture() {
    let migration_dir = fixtures_root().join("acme-template/v1.0.0-to-v1.0.1");
    let migration = Migration::load(&migration_dir, "acme-template/v1.0.0-to-v1.0.1").unwrap();

    assert_eq!(migration.id, "acme-template/v1.0.0-to-v1.0.1");
    assert_eq!(migration.old_string, "acme:1.0.0");
    assert_eq!(migration.new_string, "acme:1.0.1");
    assert_eq!(
        migration.migration_guide_link,
        Some("https://example.com/acme/upgrade".to_string())
    );
    assert_eq!(migration.target_file, "template-version.txt");
    assert_eq!(
        migration.issue_template.trim(),
        "Upgrade {{old_string}} -> {{new_string}}."
    );
    assert_eq!(
        migration.pr_template.trim(),
        "Apply migration for {{old_string}} -> {{new_string}}."
    );
}

#[test]
fn load_migration_rejects_invalid_fixture() {
    let migration_dir = fixtures_root().join("broken-template/v1.0.1-to-v1.0.2");
    let result = Migration::load(&migration_dir, "broken-template/v1.0.1-to-v1.0.2");

    assert!(matches!(result, Err(ConfigError::ValidationError { .. })));
}

#[test]
fn scan_migrations_skips_invalid_fixture() {
    let migrations = scan_migrations(&fixtures_root()).unwrap();

    assert_eq!(migrations.len(), 1);
    assert!(migrations
        .iter()
        .any(|migration| migration.id == "acme-template/v1.0.0-to-v1.0.1"));
}
