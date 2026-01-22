//! Template rendering using Handlebars.
//!
//! This module provides functions to render issue and PR templates with
//! variable substitution and conditional logic.

mod error;
mod renderer;

pub use error::TemplateError;
pub use renderer::{create_handlebars_registry, TemplateRenderer};

use crate::config::Migration;
use bstr::ByteSlice;
use handlebars::Handlebars;
use serde_json::json;

/// Renders a format template with migration data.
///
/// Available variables: `old_string`, `new_string`, `id`, `target_file`, `migration_guide_link`
fn render_format(template: &str, migration: &Migration) -> Result<String, TemplateError> {
    let hbs = Handlebars::new();
    let data = json!({
        "old_string": migration.old_string,
        "new_string": migration.new_string,
        "id": migration.id,
        "target_file": migration.target_file,
        "migration_guide_link": migration.migration_guide_link.as_deref().unwrap_or("")
    });
    Ok(hbs.render_template(template, &data)?)
}

/// Generates the issue title for an upgrade notification.
///
/// Uses the `issue_title_format` from the migration config.
///
/// # Errors
///
/// Returns [`TemplateError::RenderError`] if template rendering fails.
pub fn generate_issue_title(migration: &Migration) -> Result<String, TemplateError> {
    render_format(&migration.issue_title_format, migration)
}

/// Generates the PR title for an upgrade.
///
/// Uses the `pr_title_format` from the migration config.
///
/// # Errors
///
/// Returns [`TemplateError::RenderError`] if template rendering fails.
pub fn generate_pr_title(migration: &Migration) -> Result<String, TemplateError> {
    render_format(&migration.pr_title_format, migration)
}

/// Generates the branch name for an upgrade PR.
///
/// Uses the `branch_name_format` from the migration config.
///
/// # Errors
///
/// Returns [`TemplateError::RenderError`] if template rendering fails,
/// or [`TemplateError::InvalidBranchName`] if the rendered name is invalid.
pub fn generate_branch_name(migration: &Migration) -> Result<String, TemplateError> {
    let branch = render_format(&migration.branch_name_format, migration)?;
    validate_branch_name(&branch)?;
    Ok(branch)
}

/// Generates the commit title for an upgrade.
///
/// Uses the `commit_title_format` from the migration config.
///
/// # Errors
///
/// Returns [`TemplateError::RenderError`] if template rendering fails.
pub fn generate_commit_title(migration: &Migration) -> Result<String, TemplateError> {
    render_format(&migration.commit_title_format, migration)
}

/// Validates that a string is a valid git branch name using [`gix_validate`].
fn validate_branch_name(branch: &str) -> Result<(), TemplateError> {
    gix_validate::reference::name_partial(branch.as_bytes().as_bstr()).map_err(|e| {
        TemplateError::InvalidBranchName {
            branch: branch.to_string(),
            reason: e.to_string(),
        }
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        default_branch_name_format, default_commit_title_format, default_issue_title_format,
        default_pr_title_format,
    };

    fn sample_migration() -> Migration {
        Migration {
            id: "my-template/v1.0.0-to-v1.0.1".to_string(),
            old_string: "my-template:1.0.0".to_string(),
            new_string: "my-template:1.0.1".to_string(),
            migration_guide_link: Some("https://example.com/docs".to_string()),
            target_file: "template-version.txt".to_string(),
            issue_template: String::new(),
            pr_template: String::new(),
            issue_title_format: default_issue_title_format(),
            pr_title_format: default_pr_title_format(),
            branch_name_format: default_branch_name_format(),
            commit_title_format: default_commit_title_format(),
        }
    }

    #[test]
    fn can_generate_issue_title() {
        let migration = sample_migration();
        let title = generate_issue_title(&migration).unwrap();
        assert_eq!(
            title,
            "Template Upgrade Available: my-template:1.0.0 -> my-template:1.0.1"
        );
    }

    #[test]
    fn can_generate_pr_title() {
        let migration = sample_migration();
        let title = generate_pr_title(&migration).unwrap();
        assert_eq!(
            title,
            "Template Upgrade: my-template:1.0.0 -> my-template:1.0.1"
        );
    }

    #[test]
    fn can_generate_branch_name() {
        let migration = sample_migration();
        let branch = generate_branch_name(&migration).unwrap();
        assert_eq!(branch, "template-upgrade/my-template/v1.0.0-to-v1.0.1");
    }

    #[test]
    fn can_generate_commit_title() {
        let migration = sample_migration();
        let title = generate_commit_title(&migration).unwrap();
        assert_eq!(
            title,
            "chore: upgrade my-template:1.0.0 -> my-template:1.0.1"
        );
    }

    #[test]
    fn custom_issue_title_format() {
        let mut migration = sample_migration();
        migration.issue_title_format = "Upgrade: {{id}}".to_string();
        let title = generate_issue_title(&migration).unwrap();
        assert_eq!(title, "Upgrade: my-template/v1.0.0-to-v1.0.1");
    }

    #[test]
    fn custom_branch_name_format() {
        let mut migration = sample_migration();
        migration.branch_name_format = "upgrade/{{id}}".to_string();
        let branch = generate_branch_name(&migration).unwrap();
        assert_eq!(branch, "upgrade/my-template/v1.0.0-to-v1.0.1");
    }

    #[test]
    fn branch_name_rejects_invalid() {
        // Just verify our error wrapping works; gix-validate handles the actual validation
        assert!(matches!(
            validate_branch_name("feature branch"),
            Err(TemplateError::InvalidBranchName { .. })
        ));
    }

    #[test]
    fn branch_name_accepts_valid() {
        assert!(validate_branch_name("template-upgrade/my-template/v1.0.0-to-v1.0.1").is_ok());
    }
}
