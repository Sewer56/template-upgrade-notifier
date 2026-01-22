//! Orchestrates template upgrade scans and notifications.

use crate::config::{scan_migrations, ConfigError, Migration};
use crate::discovery::discover_repositories;
use crate::issues::{create_issue, update_issue_with_pr, IssueStatus};
use crate::pull_requests::{create_pr, PrStatus};
use crate::summary::{ProcessingResult, RunSummary};
use crate::templates::TemplateRenderer;
use futures::stream::{self, StreamExt};
use octocrab::Octocrab;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

/// Configuration for running the template upgrade notifier.
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Path to the migrations directory.
    migrations_path: PathBuf,
    /// GitHub token used for API calls and PR pushes.
    token: String,
    /// Whether to preview changes without creating issues/PRs.
    dry_run: bool,
    /// Maximum concurrent API requests.
    concurrency: usize,
    /// Whether auto-PR generation is enabled.
    auto_pr: bool,
    /// Path to the LLM config file.
    llm_config_path: PathBuf,
}

impl RunnerConfig {
    /// Creates a new configuration for a run.
    pub fn new(
        migrations_path: PathBuf,
        token: String,
        dry_run: bool,
        concurrency: usize,
        auto_pr: bool,
    ) -> Self {
        let llm_config_path = migrations_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("config.toml");
        Self {
            migrations_path,
            token,
            dry_run,
            concurrency,
            auto_pr,
            llm_config_path,
        }
    }

    /// Sets a custom LLM config path.
    pub fn with_llm_config_path(mut self, llm_config_path: PathBuf) -> Self {
        self.llm_config_path = llm_config_path;
        self
    }

    /// Returns the migrations directory path.
    pub fn migrations_path(&self) -> &Path {
        &self.migrations_path
    }

    /// Returns the configured GitHub token.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Returns whether dry-run mode is enabled.
    pub fn dry_run(&self) -> bool {
        self.dry_run
    }

    /// Returns the max concurrent API requests.
    pub fn concurrency(&self) -> usize {
        self.concurrency
    }

    /// Returns whether auto-PR generation is enabled.
    pub fn auto_pr(&self) -> bool {
        self.auto_pr
    }

    /// Returns the LLM config file path.
    pub fn llm_config_path(&self) -> &Path {
        &self.llm_config_path
    }
}

/// Errors that can occur while running the notifier.
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    /// Configuration and migration loading errors.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// GitHub API client initialization errors.
    #[error(transparent)]
    Octocrab(#[from] octocrab::Error),
}

/// Orchestrates a full template upgrade scan and notification run.
pub struct Runner {
    config: RunnerConfig,
    octocrab: Octocrab,
    renderer: TemplateRenderer,
}

impl Runner {
    /// Builds a runner from the provided configuration.
    pub fn new(config: RunnerConfig) -> Result<Self, RunnerError> {
        let octocrab = Octocrab::builder()
            .personal_token(config.token.clone())
            .build()?;
        Ok(Self {
            config,
            octocrab,
            renderer: TemplateRenderer::new(),
        })
    }

    /// Executes the full orchestration flow.
    pub async fn run(&self) -> Result<RunSummary, RunnerError> {
        let mut summary = RunSummary::new(self.config.dry_run);
        info!(path = %self.config.migrations_path.display(), "Loading migrations");
        let migrations = scan_migrations(&self.config.migrations_path)?;

        if migrations.is_empty() {
            warn!("No migrations found");
            return Ok(summary);
        }

        info!(count = migrations.len(), "Found migrations");
        summary.migrations_processed = migrations.len();

        for migration in &migrations {
            process_migration(
                &self.octocrab,
                migration,
                &self.renderer,
                &self.config,
                &mut summary,
            )
            .await?;
        }

        Ok(summary)
    }
}

async fn process_migration(
    octocrab: &Octocrab,
    migration: &Migration,
    renderer: &TemplateRenderer,
    config: &RunnerConfig,
    summary: &mut RunSummary,
) -> Result<(), RunnerError> {
    info!(
        migration_id = %migration.id,
        old_string = %migration.old_string,
        new_string = %migration.new_string,
        "Processing migration"
    );

    let repositories = match discover_repositories(octocrab, migration).await {
        Ok(repos) => repos,
        Err(e) => {
            error!(
                migration_id = %migration.id,
                error = %e,
                "Failed to discover repositories"
            );
            return Ok(());
        }
    };

    if repositories.is_empty() {
        info!(migration_id = %migration.id, "No repositories found");
        return Ok(());
    }

    info!(
        migration_id = %migration.id,
        count = repositories.len(),
        "Found repositories"
    );
    summary.repositories_discovered += repositories.len();

    if config.dry_run {
        print_dry_run_preview(migration, &repositories, renderer);
        return Ok(());
    }

    let llm_config_path = config.llm_config_path().to_path_buf();
    let results: Vec<ProcessingResult> = stream::iter(repositories)
        .map(|repo| {
            let octocrab = octocrab.clone();
            let migration = migration.clone();
            let renderer_ref = renderer;
            let token = config.token.clone();
            let auto_pr = config.auto_pr;
            let llm_config_path = llm_config_path.clone();

            async move {
                process_repository(
                    &octocrab,
                    &repo,
                    &migration,
                    renderer_ref,
                    &token,
                    auto_pr,
                    &llm_config_path,
                )
                .await
            }
        })
        .buffer_unordered(config.concurrency)
        .collect()
        .await;

    for result in &results {
        summary.record_result(result);
    }

    Ok(())
}

async fn process_repository(
    octocrab: &Octocrab,
    repository: &crate::discovery::DiscoveredRepository,
    migration: &Migration,
    renderer: &TemplateRenderer,
    token: &str,
    auto_pr: bool,
    llm_config_path: &Path,
) -> ProcessingResult {
    info!(repo = %repository.full_name, "Processing repository");

    let issue_result =
        match create_issue(octocrab, repository, migration, renderer, None, None).await {
            Ok(issue) => issue,
            Err(e) => {
                error!(
                    repo = %repository.full_name,
                    error = %e,
                    "Failed to create issue"
                );
                return ProcessingResult::Failed {
                    repository: repository.full_name.clone(),
                    error: e.to_string(),
                };
            }
        };

    let issue_status = issue_result.status.clone();
    let mut pr_status: Option<PrStatus> = None;

    if auto_pr {
        if let IssueStatus::Created { number, .. } = &issue_status {
            match create_pr(
                octocrab,
                repository,
                migration,
                renderer,
                token,
                llm_config_path,
            )
            .await
            {
                Ok(pr) => {
                    pr_status = Some(pr.status.clone());
                    if let PrStatus::Created { url, .. } = &pr.status {
                        if let Err(e) = update_issue_with_pr(
                            octocrab,
                            repository,
                            *number,
                            migration,
                            renderer,
                            &pr.status,
                            Some(url),
                        )
                        .await
                        {
                            warn!(
                                repo = %repository.full_name,
                                error = %e,
                                "Failed to update issue with PR info"
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        repo = %repository.full_name,
                        error = %e,
                        "Failed to create PR"
                    );
                    pr_status = Some(PrStatus::Failed {
                        error: e.to_string(),
                    });
                }
            }
        }
    }

    ProcessingResult::Success {
        repository: repository.full_name.clone(),
        issue: issue_status,
        pr: pr_status,
    }
}

fn print_dry_run_preview(
    migration: &Migration,
    repositories: &[crate::discovery::DiscoveredRepository],
    renderer: &TemplateRenderer,
) {
    println!("\n[DRY RUN] Migration: {}", migration.id);
    println!(
        "  Would upgrade: {} -> {}",
        migration.old_string, migration.new_string
    );
    println!("  Found {} repositories:\n", repositories.len());

    for (i, repo) in repositories.iter().enumerate() {
        println!("  [{}/{}] {}", i + 1, repositories.len(), repo.full_name);
        println!(
            "    Would create issue: \"Template Upgrade Available: {} -> {}\"",
            migration.old_string, migration.new_string
        );
        println!(
            "    Would create PR on branch: template-upgrade/{}",
            migration.id
        );
    }

    if let Some(_first_repo) = repositories.first() {
        println!("\n  Sample issue body:");
        if let Ok(body) =
            renderer.render_issue_template(&migration.issue_template, migration, None, None)
        {
            for line in body.lines().take(10) {
                println!("    {line}");
            }
            if body.lines().count() > 10 {
                println!("    ...");
            }
        }
    }

    println!();
}
