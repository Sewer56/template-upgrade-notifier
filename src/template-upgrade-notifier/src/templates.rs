//! Template rendering using Handlebars.
//!
//! This module provides functions to render issue and PR templates with
//! variable substitution and conditional logic.

use handlebars::{
    handlebars_helper, no_escape, Context, Handlebars, Helper, HelperResult, Output, RenderContext,
};
use serde_json::{json, Value};

use crate::types::{Migration, PrStatus};

/// Template rendering error.
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    /// Handlebars rendering error.
    #[error("Template rendering error: {0}")]
    RenderError(#[from] handlebars::RenderError),

    /// Template registration error.
    #[error("Template registration error: {0}")]
    RegistrationError(#[from] handlebars::TemplateError),
}

/// Creates a configured Handlebars registry with custom helpers.
///
/// The registry is configured with:
/// - No HTML escaping (for markdown output)
/// - Strict mode (catches missing variables)
/// - `eq` helper for equality comparisons
#[must_use]
pub fn create_handlebars_registry() -> Handlebars<'static> {
    let mut hbs = Handlebars::new();

    // Disable HTML escaping for markdown output
    hbs.register_escape_fn(no_escape);

    // Enable strict mode to catch missing variables
    hbs.set_strict_mode(true);

    // Register the eq helper for conditionals
    hbs.register_helper("eq", Box::new(eq_helper));

    hbs
}

/// Helper function for equality comparison in templates.
///
/// Usage: `{{#if (eq variable "value")}}...{{/if}}`
fn eq_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let param1 = h.param(0).and_then(|v| v.value().as_str());
    let param2 = h.param(1).and_then(|v| v.value().as_str());

    let result = match (param1, param2) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    };

    out.write(if result { "true" } else { "" })?;
    Ok(())
}

// Alternative: Use handlebars_helper! macro for simpler comparison
handlebars_helper!(str_eq: |a: str, b: str| a == b);

/// Template renderer for issue and PR templates.
pub struct TemplateRenderer {
    handlebars: Handlebars<'static>,
}

impl Default for TemplateRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateRenderer {
    /// Creates a new template renderer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            handlebars: create_handlebars_registry(),
        }
    }

    /// Renders an issue template with the given migration data.
    ///
    /// # Arguments
    ///
    /// * `template` - The issue template content
    /// * `migration` - The migration data
    /// * `pr_status` - Optional PR status for conditional rendering
    /// * `pr_link` - Optional PR URL
    ///
    /// # Errors
    ///
    /// Returns an error if template rendering fails.
    pub fn render_issue_template(
        &self,
        template: &str,
        migration: &Migration,
        pr_status: Option<&PrStatus>,
        pr_link: Option<&str>,
    ) -> Result<String, TemplateError> {
        let data = json!({
            "old_string": migration.old_string,
            "new_string": migration.new_string,
            "migration_guide_link": migration.migration_guide_link,
            "target_file": migration.target_file,
            "pr_status": pr_status.map_or("", |s| s.as_str()),
            "pr_link": pr_link.unwrap_or("")
        });

        self.render_template(template, &data)
    }

    /// Renders a PR template with the given migration data.
    ///
    /// # Arguments
    ///
    /// * `template` - The PR template content
    /// * `migration` - The migration data
    ///
    /// # Errors
    ///
    /// Returns an error if template rendering fails.
    pub fn render_pr_template(
        &self,
        template: &str,
        migration: &Migration,
    ) -> Result<String, TemplateError> {
        let data = json!({
            "old_string": migration.old_string,
            "new_string": migration.new_string,
            "migration_guide_link": migration.migration_guide_link,
            "target_file": migration.target_file
        });

        self.render_template(template, &data)
    }

    /// Renders a template with the given data.
    fn render_template(&self, template: &str, data: &Value) -> Result<String, TemplateError> {
        Ok(self.handlebars.render_template(template, data)?)
    }
}

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

    #[test]
    fn test_render_simple_template() {
        let renderer = TemplateRenderer::new();
        let migration = sample_migration();

        let template = "Upgrade from {{old_string}} to {{new_string}}";
        let result = renderer.render_pr_template(template, &migration).unwrap();

        assert_eq!(
            result,
            "Upgrade from my-template:1.0.0 to my-template:1.0.1"
        );
    }

    #[test]
    fn test_render_issue_with_pr_status() {
        let renderer = TemplateRenderer::new();
        let migration = sample_migration();

        let template = r#"Status: {{pr_status}}
{{#if pr_link}}PR: {{pr_link}}{{/if}}"#;

        let result = renderer
            .render_issue_template(
                template,
                &migration,
                Some(&PrStatus::Created {
                    number: 42,
                    url: "https://github.com/test/repo/pull/42".to_string(),
                }),
                Some("https://github.com/test/repo/pull/42"),
            )
            .unwrap();

        assert!(result.contains("Status: created"));
        assert!(result.contains("PR: https://github.com/test/repo/pull/42"));
    }

    #[test]
    fn test_render_conditional_eq() {
        let renderer = TemplateRenderer::new();
        let migration = sample_migration();

        let template = r#"{{#if (eq pr_status "created")}}PR was created{{else}}No PR{{/if}}"#;

        let result = renderer
            .render_issue_template(
                template,
                &migration,
                Some(&PrStatus::Created {
                    number: 1,
                    url: String::new(),
                }),
                None,
            )
            .unwrap();

        assert_eq!(result, "PR was created");
    }

    #[test]
    fn test_no_html_escaping() {
        let renderer = TemplateRenderer::new();
        let mut migration = sample_migration();
        migration.old_string = "<script>alert('xss')</script>".to_string();

        let template = "{{old_string}}";
        let result = renderer.render_pr_template(template, &migration).unwrap();

        // Should NOT escape HTML entities
        assert_eq!(result, "<script>alert('xss')</script>");
    }
}
