//! Pull request creation for template upgrades.
//!
//! This module handles creating upgrade PRs with LLM-powered code generation
//! using serdes-ai and coding tools.

mod error;
mod status;
mod upgrade_pr;

pub use error::PrError;
pub use status::PrStatus;
pub use upgrade_pr::UpgradePR;

use crate::config::Migration;
use crate::discovery::DiscoveredRepository;
use crate::llm::apply_migration;
use crate::rate_limit::ensure_core_rate_limit;
use crate::templates::generate_branch_name;
use crate::templates::generate_pr_title;
use crate::templates::TemplateRenderer;
use octocrab::Octocrab;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, info, info_span, Instrument};

/// Creates an upgrade PR for template migrations.
///
/// This function:
/// 1. Clones the repository to a temp directory
/// 2. Creates a branch
/// 3. Runs serdes-ai LLM with coding tools to apply the migration
/// 4. Checks for changes and pushes if any exist
/// 5. Creates a PR via GitHub API
///
/// # Arguments
///
/// * `octocrab` - Authenticated GitHub client
/// * `repository` - Target repository
/// * `migration` - Migration to apply
/// * `renderer` - Template renderer
/// * `token` - GitHub token for authentication
/// * `llm_config_path` - Path to LLM config.toml
///
/// # Returns
///
/// An [`UpgradePR`] with the creation status.
pub async fn create_pr(
    octocrab: &Octocrab,
    repository: &DiscoveredRepository,
    migration: &Migration,
    renderer: &TemplateRenderer,
    token: &str,
    llm_config_path: &Path,
) -> Result<UpgradePR, PrError> {
    let span = info_span!(
        "create_pr",
        repo = %repository.full_name,
        migration_id = %migration.id
    );

    async {
        info!("Creating upgrade PR");

        let branch_name = generate_branch_name(migration);
        let title = generate_pr_title(migration);

        // Create temp directory for clone
        let temp_dir = tempfile::tempdir().map_err(|e| PrError::CloneFailed {
            message: format!("Failed to create temp directory: {e}"),
        })?;

        // Clone repository
        clone_repository(repository, temp_dir.path(), token).await?;

        // Create and checkout branch
        create_branch(temp_dir.path(), &branch_name).await?;

        // Invoke serdes-ai with coding tools to apply migration
        match invoke_serdes_ai(temp_dir.path(), llm_config_path, migration).await {
            Ok(()) => {
                debug!("LLM code generation completed");
            }
            Err(PrError::Timeout { .. }) => {
                error!("LLM code generation timed out");
                return Ok(UpgradePR {
                    repository: repository.clone(),
                    migration_id: migration.id.clone(),
                    branch_name,
                    title,
                    body: String::new(),
                    status: PrStatus::TimedOut,
                });
            }
            Err(e) => {
                error!(error = %e, "LLM code generation failed");
                return Ok(UpgradePR {
                    repository: repository.clone(),
                    migration_id: migration.id.clone(),
                    branch_name,
                    title,
                    body: String::new(),
                    status: PrStatus::Failed {
                        error: e.to_string(),
                    },
                });
            }
        }

        // Check if there are changes
        if !has_changes(temp_dir.path()).await? {
            info!("No changes detected");
            return Ok(UpgradePR {
                repository: repository.clone(),
                migration_id: migration.id.clone(),
                branch_name,
                title,
                body: String::new(),
                status: PrStatus::Skipped {
                    reason: "no changes made".to_string(),
                },
            });
        }

        // Commit and push changes
        commit_and_push(temp_dir.path(), &branch_name, migration, token).await?;

        // Render PR body
        let body = renderer
            .render_pr_template(&migration.pr_template, migration)
            .map_err(|e| PrError::LlmFailed {
                message: format!("Template error: {e}"),
            })?;

        // Create PR
        let (number, url) =
            create_github_pr(octocrab, repository, &branch_name, &title, &body).await?;

        info!(pr_number = number, "PR created successfully");

        Ok(UpgradePR {
            repository: repository.clone(),
            migration_id: migration.id.clone(),
            branch_name,
            title,
            body,
            status: PrStatus::Created { number, url },
        })
    }
    .instrument(span)
    .await
}

/// Clones a repository to a local path.
async fn clone_repository(
    repository: &DiscoveredRepository,
    path: &Path,
    token: &str,
) -> Result<(), PrError> {
    debug!(repo = %repository.full_name, "Cloning repository");

    let clone_url = format!(
        "https://x-access-token:{}@github.com/{}.git",
        token, repository.full_name
    );

    let output = Command::new("git")
        .args(["clone", "--depth", "1", &clone_url, "."])
        .current_dir(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| PrError::CloneFailed {
            message: format!("Failed to execute git clone: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PrError::CloneFailed {
            message: format!("git clone failed: {stderr}"),
        });
    }

    Ok(())
}

/// Creates and checks out a new branch.
async fn create_branch(path: &Path, branch_name: &str) -> Result<(), PrError> {
    debug!(branch = %branch_name, "Creating branch");

    let output = Command::new("git")
        .args(["checkout", "-b", branch_name])
        .current_dir(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| PrError::CloneFailed {
            message: format!("Failed to create branch: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PrError::CloneFailed {
            message: format!("git checkout failed: {stderr}"),
        });
    }

    Ok(())
}

/// Invokes serdes-ai with coding tools to apply the migration.
async fn invoke_serdes_ai(
    path: &Path,
    config_path: &Path,
    migration: &Migration,
) -> Result<(), PrError> {
    apply_migration(path, config_path, migration)
        .await
        .map_err(|e| match e {
            crate::llm::LlmError::Timeout(secs) => PrError::Timeout { timeout_secs: secs },
            _ => PrError::LlmFailed {
                message: e.to_string(),
            },
        })
}

/// Checks if there are uncommitted changes.
async fn has_changes(path: &Path) -> Result<bool, PrError> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| PrError::CloneFailed {
            message: format!("Failed to check git status: {e}"),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(!stdout.trim().is_empty())
}

/// Commits and pushes changes.
async fn commit_and_push(
    path: &Path,
    branch_name: &str,
    migration: &Migration,
    token: &str,
) -> Result<(), PrError> {
    debug!("Committing and pushing changes");

    // Configure git user
    run_git_command(
        path,
        &["config", "user.email", "bot@template-upgrade-notifier"],
    )
    .await?;
    run_git_command(path, &["config", "user.name", "Template Upgrade Bot"]).await?;

    // Add all changes
    run_git_command(path, &["add", "-A"]).await?;

    // Commit
    let guide_line = migration
        .migration_guide_link
        .as_ref()
        .map(|g| format!("\n\nMigration guide: {g}"))
        .unwrap_or_default();
    let commit_msg = format!(
        "chore: upgrade {} -> {}{}",
        migration.old_string, migration.new_string, guide_line
    );
    run_git_command(path, &["commit", "-m", &commit_msg]).await?;

    // Push
    let push_url = format!("https://x-access-token:{token}@github.com");
    run_git_command(
        path,
        &["push", "-u", &push_url, &format!("HEAD:{branch_name}")],
    )
    .await
    .map_err(|e| PrError::PushFailed {
        message: e.to_string(),
    })?;

    Ok(())
}

/// Runs a git command.
async fn run_git_command(path: &Path, args: &[&str]) -> Result<(), PrError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| PrError::CloneFailed {
            message: format!("Failed to execute git {}: {e}", args.join(" ")),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PrError::CloneFailed {
            message: format!("git {} failed: {stderr}", args.join(" ")),
        });
    }

    Ok(())
}

/// Creates a PR via GitHub API.
async fn create_github_pr(
    octocrab: &Octocrab,
    repository: &DiscoveredRepository,
    branch_name: &str,
    title: &str,
    body: &str,
) -> Result<(u64, String), PrError> {
    ensure_core_rate_limit(octocrab).await?;
    let pr = octocrab
        .pulls(&repository.owner, &repository.name)
        .create(title, branch_name, &repository.default_branch)
        .body(body)
        .send()
        .await?;

    let url = pr
        .html_url
        .as_ref()
        .map(|u| u.to_string())
        .unwrap_or_else(|| {
            format!(
                "https://github.com/{}/pull/{}",
                repository.full_name, pr.number
            )
        });

    Ok((pr.number, url))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_migration() -> Migration {
        Migration {
            id: "test/v1".to_string(),
            old_string: "test:1.0.0".to_string(),
            new_string: "test:1.0.1".to_string(),
            migration_guide_link: Some("https://example.com".to_string()),
            target_file: "version.txt".to_string(),
            issue_template: String::new(),
            pr_template: String::new(),
        }
    }

    #[test]
    fn generates_branch_name() {
        let migration = sample_migration();
        let branch = generate_branch_name(&migration);
        assert_eq!(branch, "template-upgrade/test/v1");
    }

    #[test]
    fn generates_pr_title() {
        let migration = sample_migration();
        let title = generate_pr_title(&migration);
        assert_eq!(title, "Template Upgrade: test:1.0.0 -> test:1.0.1");
    }
}
