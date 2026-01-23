# template-upgrade-notifier-cli

[![Crates.io](https://img.shields.io/crates/v/template-upgrade-notifier-cli.svg)](https://crates.io/crates/template-upgrade-notifier-cli)
[![Docs.rs](https://docs.rs/template-upgrade-notifier-cli/badge.svg)](https://docs.rs/template-upgrade-notifier-cli)
[![CI](https://github.com/Sewer56/template-upgrade-notifier/actions/workflows/rust.yml/badge.svg)](https://github.com/Sewer56/template-upgrade-notifier/actions)

Command-line interface for [template-upgrade-notifier](https://github.com/Sewer56/template-upgrade-notifier).

For full documentation, migration setup guides, and project overview, see the [main project README](https://github.com/Sewer56/template-upgrade-notifier#readme).

## Installation

```bash
cargo install template-upgrade-notifier-cli
```

Or build from source:

```bash
cargo build --release -p template-upgrade-notifier-cli
```

## Getting Started

Before running, you need:

1. **A GitHub Personal Access Token** with `repo` scope (pass via `--token` or `GITHUB_TOKEN` env var)
2. **A migrations folder** containing your upgrade definitions

### Migration Folder Structure

```text
migrations/
  my-template/
    v1.0.0-to-v1.1.0/
      metadata.toml      # Defines old/new version strings
      issue-template.md  # Handlebars template for issues
      pr-template.md     # Handlebars template for PRs (optional)
```

### Minimal metadata.toml

```toml
old-string = "my-template:1.0.0"
new-string = "my-template:1.0.1"
```

For complete migration configuration options (custom titles, branch names, migration guide links), see the [library documentation](https://github.com/Sewer56/template-upgrade-notifier/blob/main/src/template-upgrade-notifier/README.MD#migration-folder-structure).

For a ready-to-use starting point, copy the [examples folder](https://github.com/Sewer56/template-upgrade-notifier/tree/main/examples).

## Usage

Run the CLI with `--help` to see all available options:

```bash
template-upgrade-notifier-cli --help
```

## Command-Line Arguments

| Argument                   | Description                                 | Default       | Required                        |
| -------------------------- | ------------------------------------------- | ------------- | ------------------------------- |
| `--migrations-path <PATH>` | Path to migrations folder                   | `migrations/` | No                              |
| `--token <TOKEN>`          | GitHub Personal Access Token                | -             | Yes (or via `GITHUB_TOKEN` env) |
| `--dry-run`                | Preview changes without creating issues/PRs | `false`       | No                              |
| `--concurrency <N>`        | Maximum concurrent API requests             | `5`           | No                              |
| `--auto-pr`                | Enable auto-PR generation via serdes-ai     | `false`       | No                              |
| `--llm-config-path`        | Path to the LLM config file                 | none          | No                              |

## Environment Variables

| Variable                           | Description                                                   |
| ---------------------------------- | ------------------------------------------------------------- |
| `GITHUB_TOKEN`                     | GitHub Personal Access Token (alternative to `--token`)       |
| `RUST_LOG`                         | Logging level filter (e.g., `debug`, `info`, `warn`, `error`) |
| `TEMPLATE_UPGRADE_LLM_MODEL`       | LLM model spec for env-only configuration                     |
| `TEMPLATE_UPGRADE_LLM_TEMPERATURE` | Sampling temperature (0.0-2.0)                                |
| `OPENAI_API_KEY`                   | OpenAI API key                                                |
| `OPENAI_BASE_URL`                  | OpenAI base URL (for proxies/Azure)                           |
| `OPENAI_TIMEOUT_SECS`              | OpenAI request timeout in seconds                             |
| `OPENROUTER_API_KEY`               | OpenRouter API key                                            |
| `OPENROUTER_HTTP_REFERER`          | OpenRouter HTTP Referer header                                |
| `OPENROUTER_APP_TITLE`             | OpenRouter app title header                                   |
| `ANTHROPIC_API_KEY`                | Anthropic API key                                             |
| `ANTHROPIC_BASE_URL`               | Anthropic base URL                                            |
| `ANTHROPIC_TIMEOUT_SECS`           | Anthropic request timeout in seconds                          |
| `GOOGLE_API_KEY`                   | Google/Gemini API key                                         |
| `GEMINI_BASE_URL`                  | Gemini base URL                                               |
| `GEMINI_TIMEOUT_SECS`              | Gemini request timeout in seconds                             |

## LLM Configuration (for Auto-PR)

When using `--auto-pr`, the CLI uses an LLM to generate fix PRs. Configure via `--llm-config-path`:

```toml
[llm]
provider = "openai"  # "openai", "openrouter", "anthropic", or "gemini"
model = "gpt-4o"
# api_key = ""       # Optional: falls back to provider env var
# base-url = ""      # Optional: custom endpoint (proxies, Azure OpenAI, etc.)
# timeout-secs = 60  # Optional: request timeout in seconds
# temperature = 0.2  # Optional: sampling temperature (0.0-2.0)
```

For OpenRouter, additional headers are supported:

```toml
[llm]
provider = "openrouter"
model = "anthropic/claude-3-opus"
# http-referer = "https://example.com"        # Optional: HTTP Referer header
# app-title = "Template Upgrade Notifier"     # Optional: app title header
# temperature = 0.2                           # Optional: sampling temperature (0.0-2.0)
```

Or set `TEMPLATE_UPGRADE_LLM_MODEL` environment variable for simple cases (e.g., `openai:gpt-4o`).

## Examples

### Dry run to preview changes:

```bash
template-upgrade-notifier-cli --token ghp_xxx --dry-run
```

### Live run creating issues:

```bash
template-upgrade-notifier-cli --token ghp_xxx --migrations-path ./my-migrations/
```

### Full run with auto-PR enabled and higher concurrency:

```bash
template-upgrade-notifier-cli --token ghp_xxx --auto-pr --concurrency 10
```

### Using environment variable for token with debug logging:

```bash
export GITHUB_TOKEN=ghp_xxx
export RUST_LOG=debug
template-upgrade-notifier-cli --dry-run
```

## Exit Codes

| Code | Description                                   |
| ---- | --------------------------------------------- |
| `0`  | Success (all operations completed or dry run) |
| `1`  | Partial failure (some operations failed)      |
| `2`  | Critical failure (unable to run)              |

## License

Licensed under LGPL V3
