//! Core types for the template upgrade notifier.
//!
//! This module contains all the core data structures used throughout the library:
//! - [`Migration`] - A single version upgrade path
//! - [`MigrationMetadata`] - Parsed metadata.toml content
//! - [`DiscoveredRepository`] - A repository found with outdated template
//! - [`UpgradeIssue`] and [`UpgradePR`] - GitHub artifacts created
//! - [`ProcessingResult`] and [`RunSummary`] - Processing outcomes

use serde::{Deserialize, Serialize};
use thiserror::Error;

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

/// Errors that can occur during repository discovery.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    /// GitHub API error.
    #[error("GitHub API error: {0}")]
    GitHubError(#[from] octocrab::Error),

    /// Rate limit exceeded.
    #[error("Rate limit exceeded, reset at {reset_at}")]
    RateLimitExceeded { reset_at: u64 },
}

/// Errors that can occur during issue operations.
#[derive(Debug, Error)]
pub enum IssueError {
    /// GitHub API error.
    #[error("GitHub API error: {0}")]
    GitHubError(#[from] octocrab::Error),

    /// Permission denied.
    #[error("Permission denied: no write access to {owner}/{repo}")]
    PermissionDenied { owner: String, repo: String },

    /// Rate limit exceeded.
    #[error("Rate limit exceeded, reset at {reset_at}")]
    RateLimitExceeded { reset_at: u64 },

    /// Template rendering error.
    #[error("Template rendering error: {0}")]
    TemplateError(String),
}

/// Errors that can occur during PR operations.
#[derive(Debug, Error)]
pub enum PrError {
    /// GitHub API error.
    #[error("GitHub API error: {0}")]
    GitHubError(#[from] octocrab::Error),

    /// Clone failed.
    #[error("Failed to clone repository: {message}")]
    CloneFailed { message: String },

    /// OpenCode invocation failed.
    #[error("OpenCode invocation failed: {message}")]
    OpenCodeFailed { message: String },

    /// OpenCode timed out.
    #[error("OpenCode timed out after {timeout_secs} seconds")]
    Timeout { timeout_secs: u64 },

    /// Push failed.
    #[error("Failed to push changes: {message}")]
    PushFailed { message: String },

    /// No changes made by OpenCode.
    #[error("No changes were made by OpenCode")]
    NoChanges,
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

/// A repository discovered to contain an outdated template version.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredRepository {
    /// Repository owner (user or organization).
    pub owner: String,

    /// Repository name.
    pub name: String,

    /// Full repository name in "owner/name" format.
    pub full_name: String,

    /// Path to the file containing the match.
    pub file_path: String,

    /// GitHub URL to the matched file.
    pub file_url: String,

    /// Default branch name (e.g., "main").
    pub default_branch: String,
}

/// Status of an issue creation operation.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum IssueStatus {
    /// Issue not yet created.
    Pending,

    /// Issue successfully created.
    Created {
        /// GitHub issue number.
        number: u64,
        /// GitHub issue URL.
        url: String,
    },

    /// Issue creation skipped.
    Skipped {
        /// Reason for skipping.
        reason: String,
    },

    /// Issue creation failed.
    Failed {
        /// Error message.
        error: String,
    },
}

/// Status of a PR creation operation.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PrStatus {
    /// PR not yet created.
    Pending,

    /// PR successfully created.
    Created {
        /// GitHub PR number.
        number: u64,
        /// GitHub PR URL.
        url: String,
    },

    /// PR creation skipped.
    Skipped {
        /// Reason for skipping.
        reason: String,
    },

    /// PR creation failed.
    Failed {
        /// Error message.
        error: String,
    },

    /// OpenCode timed out during PR generation.
    TimedOut,
}

impl PrStatus {
    /// Returns the status as a string for template rendering.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Created { .. } => "created",
            Self::Skipped { .. } => "skipped",
            Self::Failed { .. } => "failed",
            Self::TimedOut => "failed",
        }
    }

    /// Returns the PR URL if created.
    #[must_use]
    pub fn url(&self) -> Option<&str> {
        match self {
            Self::Created { url, .. } => Some(url),
            _ => None,
        }
    }
}

/// A GitHub issue created in a discovered repository.
#[derive(Debug, Clone)]
pub struct UpgradeIssue {
    /// Target repository.
    pub repository: DiscoveredRepository,

    /// Reference to source migration.
    pub migration_id: String,

    /// Issue title.
    pub title: String,

    /// Rendered issue body.
    pub body: String,

    /// Creation status.
    pub status: IssueStatus,
}

/// A pull request created to apply a migration.
#[derive(Debug, Clone)]
pub struct UpgradePR {
    /// Target repository.
    pub repository: DiscoveredRepository,

    /// Reference to source migration.
    pub migration_id: String,

    /// Feature branch name.
    pub branch_name: String,

    /// PR title.
    pub title: String,

    /// Rendered PR body.
    pub body: String,

    /// Creation status.
    pub status: PrStatus,
}

/// Result of processing a single repository.
#[derive(Debug, Clone)]
pub enum ProcessingResult {
    /// Processing succeeded.
    Success {
        /// Repository full name.
        repository: String,
        /// Issue creation status.
        issue: IssueStatus,
        /// Optional PR creation status.
        pr: Option<PrStatus>,
    },

    /// Processing was skipped.
    Skipped {
        /// Repository full name.
        repository: String,
        /// Reason for skipping.
        reason: String,
    },

    /// Processing failed.
    Failed {
        /// Repository full name.
        repository: String,
        /// Error message.
        error: String,
    },
}

/// Summary of a complete run.
#[derive(Debug, Clone, Default)]
pub struct RunSummary {
    /// Number of migrations processed.
    pub migrations_processed: usize,

    /// Number of repositories discovered.
    pub repositories_discovered: usize,

    /// Number of issues successfully created.
    pub issues_created: usize,

    /// Number of issues skipped (e.g., duplicates).
    pub issues_skipped: usize,

    /// Number of issues that failed to create.
    pub issues_failed: usize,

    /// Number of PRs successfully created.
    pub prs_created: usize,

    /// Number of PRs that failed to create.
    pub prs_failed: usize,

    /// Whether this was a dry run.
    pub dry_run: bool,
}

impl RunSummary {
    /// Creates a new empty summary.
    #[must_use]
    pub fn new(dry_run: bool) -> Self {
        Self {
            dry_run,
            ..Default::default()
        }
    }

    /// Updates the summary with a processing result.
    pub fn record_result(&mut self, result: &ProcessingResult) {
        match result {
            ProcessingResult::Success { issue, pr, .. } => {
                match issue {
                    IssueStatus::Created { .. } => self.issues_created += 1,
                    IssueStatus::Skipped { .. } => self.issues_skipped += 1,
                    IssueStatus::Failed { .. } => self.issues_failed += 1,
                    IssueStatus::Pending => {}
                }
                if let Some(pr_status) = pr {
                    match pr_status {
                        PrStatus::Created { .. } => self.prs_created += 1,
                        PrStatus::Failed { .. } | PrStatus::TimedOut => self.prs_failed += 1,
                        _ => {}
                    }
                }
            }
            ProcessingResult::Skipped { .. } => self.issues_skipped += 1,
            ProcessingResult::Failed { .. } => self.issues_failed += 1,
        }
    }

    /// Returns true if any failures occurred.
    #[must_use]
    pub fn has_failures(&self) -> bool {
        self.issues_failed > 0 || self.prs_failed > 0
    }

    /// Returns true if all operations were successful.
    #[must_use]
    pub fn all_success(&self) -> bool {
        self.issues_failed == 0 && self.prs_failed == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pr_status_as_str() {
        assert_eq!(PrStatus::Pending.as_str(), "pending");
        assert_eq!(
            PrStatus::Created {
                number: 1,
                url: "https://example.com".to_string()
            }
            .as_str(),
            "created"
        );
        assert_eq!(
            PrStatus::Skipped {
                reason: "test".to_string()
            }
            .as_str(),
            "skipped"
        );
        assert_eq!(
            PrStatus::Failed {
                error: "test".to_string()
            }
            .as_str(),
            "failed"
        );
        assert_eq!(PrStatus::TimedOut.as_str(), "failed");
    }

    #[test]
    fn test_run_summary_record_result() {
        let mut summary = RunSummary::new(false);

        summary.record_result(&ProcessingResult::Success {
            repository: "test/repo".to_string(),
            issue: IssueStatus::Created {
                number: 1,
                url: "https://example.com".to_string(),
            },
            pr: Some(PrStatus::Created {
                number: 2,
                url: "https://example.com/pr".to_string(),
            }),
        });

        assert_eq!(summary.issues_created, 1);
        assert_eq!(summary.prs_created, 1);
        assert!(summary.all_success());
    }

    #[test]
    fn test_default_target_file() {
        assert_eq!(default_target_file(), "template-version.txt");
    }
}
