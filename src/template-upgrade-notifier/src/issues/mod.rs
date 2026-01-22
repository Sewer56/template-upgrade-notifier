//! GitHub issue creation and management.
//!
//! This module handles creating upgrade notification issues in discovered
//! repositories, including duplicate detection and permission handling.

mod error;
mod status;
mod upgrade_issue;

pub use error::IssueError;
pub use status::IssueStatus;
pub use upgrade_issue::UpgradeIssue;

use crate::config::Migration;
use crate::discovery::DiscoveredRepository;
use crate::pull_requests::PrStatus;
use crate::rate_limit::{ensure_core_rate_limit, ensure_search_rate_limit};
use crate::templates::generate_issue_title;
use crate::templates::TemplateRenderer;
use octocrab::Octocrab;
use tracing::{debug, info, info_span, warn, Instrument};

/// Creates an upgrade notification issue in a repository.
///
/// This function:
/// 1. Checks for existing duplicate issues
/// 2. Renders the issue template
/// 3. Creates the issue via GitHub API
///
/// # Arguments
///
/// * `octocrab` - Authenticated GitHub client
/// * `repository` - Target repository
/// * `migration` - Migration to create issue for
/// * `renderer` - Template renderer
/// * `pr_status` - Optional PR status for template rendering
/// * `pr_link` - Optional PR URL for template rendering
///
/// # Returns
///
/// An [`UpgradeIssue`] with the creation status.
///
/// # Errors
///
/// Returns [`IssueError`] if creation fails (except for permission denied,
/// which returns a [`Skipped`][`IssueStatus::Skipped`] status).
pub async fn create_issue(
    octocrab: &Octocrab,
    repository: &DiscoveredRepository,
    migration: &Migration,
    renderer: &TemplateRenderer,
    pr_status: Option<&PrStatus>,
    pr_link: Option<&str>,
) -> Result<UpgradeIssue, IssueError> {
    let span = info_span!(
        "create_issue",
        repo = %repository.full_name,
        migration_id = %migration.id
    );

    async {
        info!("Creating upgrade issue");

        // Generate title
        let title = generate_issue_title(migration);

        // Check for duplicate
        if let Some(existing) = check_duplicate_issue(octocrab, repository, &title).await? {
            info!(issue_number = existing, "Duplicate issue exists, skipping");
            return Ok(UpgradeIssue {
                repository: repository.clone(),
                migration_id: migration.id.clone(),
                title,
                body: String::new(),
                status: IssueStatus::Skipped {
                    reason: format!("duplicate issue exists (#{existing})"),
                },
            });
        }

        // Render template
        let body = renderer
            .render_issue_template(&migration.issue_template, migration, pr_status, pr_link)
            .map_err(|e: crate::templates::TemplateError| {
                IssueError::TemplateError(e.to_string())
            })?;

        // Create issue
        match create_github_issue(octocrab, repository, &title, &body).await {
            Ok((number, url)) => {
                info!(issue_number = number, "Issue created successfully");
                Ok(UpgradeIssue {
                    repository: repository.clone(),
                    migration_id: migration.id.clone(),
                    title,
                    body,
                    status: IssueStatus::Created { number, url },
                })
            }
            Err(e) => {
                if is_permission_denied(&e) {
                    warn!("Permission denied, skipping repository");
                    Ok(UpgradeIssue {
                        repository: repository.clone(),
                        migration_id: migration.id.clone(),
                        title,
                        body,
                        status: IssueStatus::Skipped {
                            reason: "no write access".to_string(),
                        },
                    })
                } else {
                    Err(e)
                }
            }
        }
    }
    .instrument(span)
    .await
}

/// Updates an existing issue with PR information.
///
/// This is called after a PR is created to update the issue body with
/// the PR link and status.
///
/// # Arguments
///
/// * `octocrab` - Authenticated GitHub client
/// * `repository` - Repository containing the issue
/// * `issue_number` - Issue number to update
/// * `migration` - Migration for template rendering
/// * `renderer` - Template renderer
/// * `pr_status` - PR status for template
/// * `pr_link` - PR URL for template
///
/// # Errors
///
/// Returns [`IssueError`] if the update fails.
pub async fn update_issue_with_pr(
    octocrab: &Octocrab,
    repository: &DiscoveredRepository,
    issue_number: u64,
    migration: &Migration,
    renderer: &TemplateRenderer,
    pr_status: &PrStatus,
    pr_link: Option<&str>,
) -> Result<(), IssueError> {
    let span = info_span!(
        "update_issue",
        repo = %repository.full_name,
        issue_number = issue_number
    );

    async {
        info!("Updating issue with PR information");

        // Render updated template
        let body = renderer
            .render_issue_template(
                &migration.issue_template,
                migration,
                Some(pr_status),
                pr_link,
            )
            .map_err(|e: crate::templates::TemplateError| {
                IssueError::TemplateError(e.to_string())
            })?;

        // Ensure rate limit
        ensure_core_rate_limit(octocrab).await?;

        // Update issue
        octocrab
            .issues(&repository.owner, &repository.name)
            .update(issue_number)
            .body(&body)
            .send()
            .await?;

        info!("Issue updated successfully");
        Ok(())
    }
    .instrument(span)
    .await
}

/// Checks if an issue with the given title already exists.
///
/// Returns the issue number if found.
async fn check_duplicate_issue(
    octocrab: &Octocrab,
    repository: &DiscoveredRepository,
    title: &str,
) -> Result<Option<u64>, IssueError> {
    debug!(title = %title, "Checking for duplicate issue");

    // Search for open issues with exact title match
    let query = format!(
        "repo:{} is:issue is:open in:title \"{}\"",
        repository.full_name, title
    );

    // Check rate limit before search API call
    ensure_search_rate_limit(octocrab).await?;

    let results = octocrab
        .search()
        .issues_and_pull_requests(&query)
        .send()
        .await?;

    // Check for exact title match
    for issue in &results.items {
        if issue.title == title {
            return Ok(Some(issue.number));
        }
    }

    Ok(None)
}

/// Creates an issue via GitHub API.
async fn create_github_issue(
    octocrab: &Octocrab,
    repository: &DiscoveredRepository,
    title: &str,
    body: &str,
) -> Result<(u64, String), IssueError> {
    ensure_core_rate_limit(octocrab).await?;
    let issue = octocrab
        .issues(&repository.owner, &repository.name)
        .create(title)
        .body(body)
        .send()
        .await?;

    let url = issue.html_url.to_string();
    Ok((issue.number, url))
}

/// Checks if an error indicates permission denied.
fn is_permission_denied(error: &IssueError) -> bool {
    match error {
        IssueError::GitHubError(e) => {
            let msg = e.to_string().to_lowercase();
            msg.contains("403") || msg.contains("forbidden") || msg.contains("permission")
        }
        IssueError::PermissionDenied { .. } => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_detect_permission_denied() {
        assert!(is_permission_denied(&IssueError::PermissionDenied {
            owner: "test".to_string(),
            repo: "repo".to_string()
        }));

        assert!(!is_permission_denied(&IssueError::TemplateError(
            "error".to_string()
        )));
    }
}
