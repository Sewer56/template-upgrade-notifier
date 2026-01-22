# template-upgrade-notifier

[![Crates.io](https://img.shields.io/crates/v/template-upgrade-notifier.svg)](https://crates.io/crates/template-upgrade-notifier)
[![Docs.rs](https://docs.rs/template-upgrade-notifier/badge.svg)](https://docs.rs/template-upgrade-notifier)
[![CI](https://github.com/Sewer56/template-upgrade-notifier/actions/workflows/rust.yml/badge.svg)](https://github.com/Sewer56/template-upgrade-notifier/actions)

<!-- 
    This README is displayed on crates.io & docs.rs
    Write detailed documentation here for crate users.
-->

A GitHub Action that scans repositories for outdated template versions and creates upgrade notification issues with LLM-powered auto-fix PRs

## Features

- Scan migrations from filesystem with `scan_migrations` and `load_migration`
- Discover repositories via GitHub Code Search API with pagination and deduplication
- Create upgrade notification issues with duplicate detection
- Generate auto-fix PRs using serdes-ai with coding tools
- Render templates using Handlebars with conditional logic support
- Built-in rate limit handling with proactive waiting
- Comprehensive error types for each module

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
template-upgrade-notifier = "0.1.0"
```

## Migration Folder Structure

Migrations are organized in a nested directory structure:

```text
migrations/
├── my-template/
│   ├── v1.0.0-to-v1.0.1/
│   │   ├── metadata.toml
│   │   ├── issue-template.md
│   │   └── pr-template.md
│   └── v1.0.1-to-v1.1.0/
│       ├── metadata.toml
│       ├── issue-template.md
│       └── pr-template.md
└── another-template/
    └── v2.0.0-to-v3.0.0/
        ├── metadata.toml
        ├── issue-template.md
        └── pr-template.md
```

### metadata.toml

Configuration for a single migration:

```toml
old-string = "my-template:1.0.0"
new-string = "my-template:1.0.1"
migration-guide-link = "https://example.com/docs/v1.0.1-migration"
target-file = "template-version.txt"  # Optional, defaults to "template-version.txt"
```

### issue-template.md

Handlebars template for the notification issue body.

### pr-template.md

Handlebars template for auto-fix PR body.

### config.toml

LLM configuration file for auto-PR generation:

```toml
[llm]
# Provider to use: "open_ai", "open_router", "anthropic", or "gemini"
provider = "open_ai"

# Model name (provider-specific)
model = "gpt-4o"

# Optional: API key (falls back to provider env var if not set)
# api_key = ""

# Optional: Custom base URL
# base-url = "https://api.openai.com/v1"

# Optional: Timeout in seconds (default: 600)
# timeout-secs = 600
```

Alternatively, configure via environment variables:

| Variable                       | Description                              |
| ------------------------------ | ---------------------------------------- |
| `TEMPLATE_UPGRADE_LLM_MODEL` | LLM model spec for env-only configuration |
| `OPENAI_API_KEY`             | OpenAI API key                           |
| `OPENROUTER_API_KEY`          | OpenRouter API key                        |
| `ANTHROPIC_API_KEY`          | Anthropic API key                         |
| `GOOGLE_API_KEY`             | Google/Gemini API key                    |

Use `--llm-config-path` to point at a non-default config location.

## Template Variables

Both issue and PR templates support these Handlebars variables:

| Variable                   | Description                            |
| -------------------------- | -------------------------------------- |
| `{{old_string}}`           | Version string being replaced          |
| `{{new_string}}`           | New version string                     |
| `{{migration_guide_link}}` | URL to migration documentation         |
| `{{target_file}}`          | Name of file containing version string |

Issue templates also support:

| Variable        | Description                                             |
| --------------- | ------------------------------------------------------- |
| `{{pr_status}}` | PR status: "pending", "created", "skipped", or "failed" |
| `{{pr_link}}`   | URL to the created PR (empty if not created)            |

### Conditionals

Use the `eq` helper for conditional rendering:

```handlebars
{{#if (eq pr_status "created")}}
A PR has been created: {{pr_link}}
{{else}}
No automated fix is available.
{{/if}}
```

## Usage

### Basic Example

```rust,no_run
use std::path::Path;
use template_upgrade_notifier::{
    create_issue, discover_repositories, scan_migrations, TemplateRenderer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Octocrab with a personal access token
    let octocrab = octocrab::Octocrab::builder()
        .personal_token(std::env::var("GITHUB_TOKEN")?)
        .build()?;

    // Load migrations from a directory
    let migrations = scan_migrations(Path::new("./migrations"))?;
    println!("Loaded {} migrations", migrations.len());

    // Create a template renderer
    let renderer = TemplateRenderer::new();

    // Process each migration
    for migration in &migrations {
        // Discover repositories with outdated versions
        let repositories = discover_repositories(&octocrab, migration).await?;
        println!(
            "Found {} repositories for migration {}",
            repositories.len(),
            migration.id
        );

        // Create issues in discovered repositories
        for repo in &repositories {
            let issue = create_issue(&octocrab, repo, migration, &renderer, None, None).await?;
            println!("Created issue in {}: {:?}", repo.full_name, issue.status);
        }
    }

    Ok(())
}
```

### Advanced Example

```rust,no_run
use std::path::Path;
use template_upgrade_notifier::{
    create_issue, create_pr, discover_repositories, enrich_with_default_branches,
    scan_migrations, IssueStatus, PrStatus, ProcessingResult, RunSummary, TemplateRenderer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("GITHUB_TOKEN")?;
    let octocrab = octocrab::Octocrab::builder()
        .personal_token(token.clone())
        .build()?;

    let migrations = scan_migrations(Path::new("./migrations"))?;
    let renderer = TemplateRenderer::new();
    let mut summary = RunSummary::new(false);

    for migration in &migrations {
        // Discover and enrich repositories with default branch info
        let mut repositories = discover_repositories(&octocrab, &migration).await?;
        enrich_with_default_branches(&octocrab, &mut repositories).await?;
        summary.repositories_discovered += repositories.len();

        for repo in &repositories {
            // First, try to create an auto-fix PR
            let pr_result = create_pr(&octocrab, repo, &migration, &renderer, &token).await?;

            // Then create an issue with PR status
            let issue = create_issue(
                &octocrab,
                repo,
                &migration,
                &renderer,
                Some(&pr_result.status),
                pr_result.status.url(),
            )
            .await?;

            // Record result
            let result = ProcessingResult::Success {
                repository: repo.full_name.clone(),
                issue: issue.status.clone(),
                pr: Some(pr_result.status),
            };
            summary.record_result(&result);
        }
        summary.migrations_processed += 1;
    }

    println!("Summary: {} issues created, {} PRs created",
        summary.issues_created, summary.prs_created);

    Ok(())
}
```

## License

Licensed under LGPL V3
