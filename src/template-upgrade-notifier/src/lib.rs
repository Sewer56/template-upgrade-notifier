#![doc = include_str!(concat!("../", env!("CARGO_PKG_README")))]

pub mod config;
pub mod discovery;
pub mod issues;
pub(crate) mod llm;
pub mod pull_requests;
pub mod rate_limit;
pub mod runner;
pub mod summary;
pub mod templates;

pub use config::{load_migration, scan_migrations, ConfigError, Migration, MigrationMetadata};
pub use discovery::{
    discover_repositories, enrich_with_default_branches, get_default_branch, DiscoveredRepository,
    DiscoveryError,
};
pub use issues::{create_issue, update_issue_with_pr, IssueError, IssueStatus, UpgradeIssue};
pub use pull_requests::{create_pr, PrError, PrStatus, UpgradePR};
pub use rate_limit::{
    check_core_rate_limit, check_search_rate_limit, ensure_core_rate_limit,
    ensure_search_rate_limit, wait_for_retry_after, wait_if_needed, RateLimitInfo,
};
pub use runner::{Runner, RunnerConfig, RunnerError};
pub use summary::{ProcessingResult, RunSummary};
pub use templates::{
    create_handlebars_registry, generate_branch_name, generate_issue_title, generate_pr_title,
    TemplateError, TemplateRenderer,
};
