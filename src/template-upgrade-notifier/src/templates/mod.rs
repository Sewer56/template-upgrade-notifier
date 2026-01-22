//! Template rendering using Handlebars.
//!
//! This module provides functions to render issue and PR templates with
//! variable substitution and conditional logic.

mod error;
mod renderer;

pub use error::TemplateError;
pub use renderer::{create_handlebars_registry, TemplateRenderer};

use crate::config::Migration;

/// Generates the issue title for an upgrade notification.
///
/// Format: "Template Upgrade Available: {old_string} -> {new_string}"
#[must_use]
pub fn generate_issue_title(migration: &Migration) -> String {
    format!(
        "Template Upgrade Available: {} -> {}",
        migration.old_string, migration.new_string
    )
}

/// Generates the PR title for an upgrade.
///
/// Format: "Template Upgrade: {old_string} -> {new_string}"
#[must_use]
pub fn generate_pr_title(migration: &Migration) -> String {
    format!(
        "Template Upgrade: {} -> {}",
        migration.old_string, migration.new_string
    )
}

/// Generates the branch name for an upgrade PR.
///
/// Format: "template-upgrade/{migration_id}"
#[must_use]
pub fn generate_branch_name(migration: &Migration) -> String {
    format!("template-upgrade/{}", migration.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_migration() -> Migration {
        Migration {
            id: "my-template/v1.0.0-to-v1.0.1".to_string(),
            old_string: "my-template:1.0.0".to_string(),
            new_string: "my-template:1.0.1".to_string(),
            migration_guide_link: "https://example.com/docs".to_string(),
            target_file: "template-version.txt".to_string(),
            issue_template: String::new(),
            pr_template: String::new(),
        }
    }

    #[test]
    fn test_generate_issue_title() {
        let migration = sample_migration();
        let title = generate_issue_title(&migration);
        assert_eq!(
            title,
            "Template Upgrade Available: my-template:1.0.0 -> my-template:1.0.1"
        );
    }

    #[test]
    fn test_generate_pr_title() {
        let migration = sample_migration();
        let title = generate_pr_title(&migration);
        assert_eq!(
            title,
            "Template Upgrade: my-template:1.0.0 -> my-template:1.0.1"
        );
    }

    #[test]
    fn test_generate_branch_name() {
        let migration = sample_migration();
        let branch = generate_branch_name(&migration);
        assert_eq!(branch, "template-upgrade/my-template/v1.0.0-to-v1.0.1");
    }
}
