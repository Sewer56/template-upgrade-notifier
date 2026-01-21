# template-upgrade-notifier-cli

[![Crates.io](https://img.shields.io/crates/v/template-upgrade-notifier-cli.svg)](https://crates.io/crates/template-upgrade-notifier-cli)
[![Docs.rs](https://docs.rs/template-upgrade-notifier-cli/badge.svg)](https://docs.rs/template-upgrade-notifier-cli)
[![CI](https://github.com/Sewer56/template-upgrade-notifier/actions/workflows/rust.yml/badge.svg)](https://github.com/Sewer56/template-upgrade-notifier/actions)

Command-line interface for [template-upgrade-notifier](../template-upgrade-notifier/README.MD).

<!-- 
    This README is displayed on crates.io & docs.rs
    Write detailed documentation here for users.
-->

## Installation

```bash
cargo install template-upgrade-notifier-cli
```

Or build from source:

```bash
cargo build --release -p template-upgrade-notifier-cli
```

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
| `--auto-pr`                | Enable auto-PR generation with OpenCode     | `false`       | No                              |

## Environment Variables

| Variable       | Description                                                   |
| -------------- | ------------------------------------------------------------- |
| `GITHUB_TOKEN` | GitHub Personal Access Token (alternative to `--token`)       |
| `RUST_LOG`     | Logging level filter (e.g., `debug`, `info`, `warn`, `error`) |

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
