//! Run summary types.

use super::result::ProcessingResult;
use crate::issues::IssueStatus;
use crate::pull_requests::PrStatus;

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
    fn can_record_result() {
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
}
