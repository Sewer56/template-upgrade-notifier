//! Runner configuration.

use std::path::{Path, PathBuf};

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
