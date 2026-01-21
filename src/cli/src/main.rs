//! CLI for the Template Upgrade Notifier.
//!
//! This tool scans repositories for outdated template versions and creates
//! upgrade notification issues with optional auto-fix PRs.

use clap::Parser;
use futures::stream::{self, StreamExt};
use std::path::PathBuf;
use std::process::ExitCode;
use template_upgrade_notifier::{
    create_issue, create_pr, discover_repositories, scan_migrations, IssueStatus, Migration,
    PrStatus, ProcessingResult, RunSummary, TemplateRenderer,
};
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Template Upgrade Notifier - Scan repositories for outdated templates and create upgrade issues.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to migrations folder.
    #[arg(long, default_value = "migrations/")]
    migrations_path: PathBuf,

    /// GitHub Personal Access Token.
    #[arg(long, env = "GITHUB_TOKEN")]
    token: String,

    /// Preview changes without creating issues/PRs.
    #[arg(long)]
    dry_run: bool,

    /// Maximum concurrent API requests.
    #[arg(long, default_value_t = 5)]
    concurrency: usize,

    /// Enable auto-PR generation with OpenCode.
    #[arg(long)]
    auto_pr: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize tracing
    init_tracing();

    // Parse arguments
    let args = Args::parse();

    // Run the main logic
    match run(args).await {
        Ok(summary) => {
            print_summary(&summary);

            if summary.all_success() {
                ExitCode::from(0)
            } else if summary.has_failures() {
                ExitCode::from(1)
            } else {
                ExitCode::from(0)
            }
        }
        Err(e) => {
            error!(error = %e, "Critical failure");
            ExitCode::from(2)
        }
    }
}

/// Initializes tracing with environment filter support.
///
/// Tracing is Rust's structured logging/diagnostics framework. Unlike traditional
/// logging, it's async-aware and captures contextual, structured data rather than
/// just text. The subscriber configured here determines how events (from macros
/// like `info!`, `debug!`, etc.) are collected and displayed.
///
/// Sets up the global tracing subscriber with:
/// - Compact log formatting (single-line output)
/// - Log level filtering via `RUST_LOG` env var (defaults to "info")
fn init_tracing() {
    tracing_subscriber::registry()
        // Use compact formatting without module target paths for cleaner output
        .with(fmt::layer().compact().with_target(false))
        // Allow runtime log filtering via RUST_LOG env var (e.g., RUST_LOG=debug)
        // Falls back to "info" level if RUST_LOG is not set or invalid
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        // Register as the global default subscriber
        .init();
}

/// Main execution logic.
async fn run(args: Args) -> Result<RunSummary, Box<dyn std::error::Error>> {
    let mut summary = RunSummary::new(args.dry_run);

    // Load migrations
    info!(
        path = %args.migrations_path.display(),
        "Loading migrations"
    );
    let migrations = scan_migrations(&args.migrations_path)?;

    if migrations.is_empty() {
        warn!("No migrations found");
        return Ok(summary);
    }

    info!(count = migrations.len(), "Found migrations");
    summary.migrations_processed = migrations.len();

    // Initialize GitHub client
    let octocrab = octocrab::Octocrab::builder()
        .personal_token(args.token.clone())
        .build()?;

    // Initialize template renderer
    let renderer = TemplateRenderer::new();

    // Process each migration
    for migration in &migrations {
        process_migration(&octocrab, migration, &renderer, &args, &mut summary).await?;
    }

    Ok(summary)
}

/// Processes a single migration.
async fn process_migration(
    octocrab: &octocrab::Octocrab,
    migration: &Migration,
    renderer: &TemplateRenderer,
    args: &Args,
    summary: &mut RunSummary,
) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        migration_id = %migration.id,
        old_string = %migration.old_string,
        new_string = %migration.new_string,
        "Processing migration"
    );

    // Discover repositories
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

    if args.dry_run {
        // Dry run mode - just report what would happen
        print_dry_run_preview(migration, &repositories, renderer);
        return Ok(());
    }

    // Process repositories concurrently
    let results: Vec<ProcessingResult> = stream::iter(repositories)
        .map(|repo| {
            let octocrab = octocrab.clone();
            let migration = migration.clone();
            let renderer_ref = renderer;
            let token = args.token.clone();
            let auto_pr = args.auto_pr;

            async move {
                process_repository(&octocrab, &repo, &migration, renderer_ref, &token, auto_pr)
                    .await
            }
        })
        .buffer_unordered(args.concurrency)
        .collect()
        .await;

    // Update summary with results
    for result in &results {
        summary.record_result(result);
    }

    Ok(())
}

/// Processes a single repository.
async fn process_repository(
    octocrab: &octocrab::Octocrab,
    repository: &template_upgrade_notifier::DiscoveredRepository,
    migration: &Migration,
    renderer: &TemplateRenderer,
    token: &str,
    auto_pr: bool,
) -> ProcessingResult {
    info!(repo = %repository.full_name, "Processing repository");

    // Create issue
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

    // Create PR if enabled and issue was created
    if auto_pr {
        if let IssueStatus::Created { number, .. } = &issue_status {
            match create_pr(octocrab, repository, migration, renderer, token).await {
                Ok(pr) => {
                    pr_status = Some(pr.status.clone());

                    // Update issue with PR info
                    if let PrStatus::Created { url, .. } = &pr.status {
                        if let Err(e) = template_upgrade_notifier::update_issue_with_pr(
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

/// Prints a dry run preview.
fn print_dry_run_preview(
    migration: &Migration,
    repositories: &[template_upgrade_notifier::DiscoveredRepository],
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

    // Show sample issue body
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

/// Prints the final run summary.
fn print_summary(summary: &RunSummary) {
    println!("\nSummary:");
    println!(
        "  Mode: {}",
        if summary.dry_run { "Dry Run" } else { "Live" }
    );
    println!("  Migrations processed: {}", summary.migrations_processed);
    println!(
        "  Repositories discovered: {}",
        summary.repositories_discovered
    );

    if !summary.dry_run {
        println!("  Issues created: {}", summary.issues_created);
        println!("  Issues skipped: {}", summary.issues_skipped);
        println!("  Issues failed: {}", summary.issues_failed);
        println!("  PRs created: {}", summary.prs_created);
        println!("  PRs failed: {}", summary.prs_failed);
    }
}
