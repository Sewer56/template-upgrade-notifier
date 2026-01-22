# Examples

This folder contains sample configuration files to help you get started.

## Contents

- `config.toml` - Sample LLM configuration for auto-PR generation
- `reloaded-templates-rust/` - Sample migration based on a real [reloaded-templates-rust](https://github.com/Reloaded-Project/reloaded-templates-rust) upgrade

## Quick Start

1. Copy this folder to your project
2. Edit `reloaded-templates-rust/v1.0.1-to-v1.1.0/metadata.toml` with your version strings
3. Customize the issue and PR templates
4. Set your `GITHUB_TOKEN` environment variable
5. Run:

```bash
template-upgrade-notifier-cli --migrations-path ./examples --dry-run
```

## Creating Your Own Migration

1. Create a folder structure: `<template-name>/<old-version>-to-<new-version>/`
2. Add `metadata.toml` with at minimum:
   ```toml
   old-string = "your-template:1.0.0"
   new-string = "your-template:1.1.0"
   ```
3. Add `issue-template.md` (required) and `pr-template.md` (optional)
4. Use Handlebars variables like `{{old_string}}`, `{{new_string}}`, `{{migration_guide_link}}`

For the full migration format specification, see the [library documentation](https://github.com/Sewer56/template-upgrade-notifier/blob/main/src/template-upgrade-notifier/README.MD#migration-folder-structure).

For general project documentation, see the [main README](../README.MD).
