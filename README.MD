# template-upgrade-notifier

[![Crates.io](https://img.shields.io/crates/v/template-upgrade-notifier.svg)](https://crates.io/crates/template-upgrade-notifier)
[![Docs.rs](https://docs.rs/template-upgrade-notifier/badge.svg)](https://docs.rs/template-upgrade-notifier)
[![CI](https://github.com/Sewer56/template-upgrade-notifier/actions/workflows/rust.yml/badge.svg)](https://github.com/Sewer56/template-upgrade-notifier/actions)

A GitHub Action that scans repositories for outdated template versions and creates upgrade notification issues with LLM-powered auto-fix PRs

## Quick Start

Install the CLI:

```bash
cargo install template-upgrade-notifier-cli
```

Run a dry-run scan:

```bash
template-upgrade-notifier-cli \
    --migrations-path ./migrations \
    --token "$GITHUB_TOKEN" \
    --dry-run
```

For library usage, see the [template-upgrade-notifier crate documentation](./src/template-upgrade-notifier/README.MD).

## Migration Structure

Migrations are defined in folders containing:

```text
migrations/
  my-migration/
    metadata.toml     # old_string, new_string, target_file
    issue-template.md # Handlebars template for issues
    pr-template.md    # Handlebars template for PRs
```

See the [library documentation](./src/template-upgrade-notifier/README.MD) for full configuration details.

## Crates

- [template-upgrade-notifier](./src/template-upgrade-notifier/README.MD): Core library for template version discovery and upgrade notification
- [cli](./src/cli/README.MD): Command-line tool for running template upgrade scans

## Developer Manual

For step-by-step development guidance, see the [Developer Manual](https://reloaded-project.github.io/reloaded-templates-rust/manual/).

## Contributing

We welcome contributions! See the [Contributing Guide](https://reloaded-project.github.io/reloaded-templates-rust/manual/#contributing) for details.

## License

Licensed under [LGPL V3](./LICENSE).

[codecov]: https://app.codecov.io
[gh-actions-secrets]: https://github.com/Sewer56/template-upgrade-notifier/settings/secrets/actions
[gh-actions-dependabot]: https://github.com/Sewer56/template-upgrade-notifier/settings/secrets/dependabot
[gh-pages]: https://github.com/Sewer56/template-upgrade-notifier/settings/pages
[crates-io-key]: https://crates.io/settings/tokens
[nuget-key]: https://www.nuget.org/account/apikeys
