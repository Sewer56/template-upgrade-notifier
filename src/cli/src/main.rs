//! CLI for the Template Upgrade Notifier.
//!
//! This tool scans repositories for outdated template versions and creates
//! upgrade notification issues with optional auto-fix PRs.

use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;
use template_upgrade_notifier::{RunSummary, Runner, RunnerConfig, RunnerError};
use tracing::error;
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

    /// Enable auto-PR generation via serdes-ai.
    #[arg(long)]
    auto_pr: bool,

    /// Path to the LLM config file.
    #[arg(long)]
    llm_config_path: Option<PathBuf>,
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
async fn run(args: Args) -> Result<RunSummary, RunnerError> {
    let mut config = RunnerConfig::new(
        args.migrations_path,
        args.token,
        args.dry_run,
        args.concurrency,
        args.auto_pr,
    );
    if let Some(path) = args.llm_config_path {
        config = config.with_llm_config_path(path);
    }
    let runner = Runner::new(config)?;
    runner.run().await
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
