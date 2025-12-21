# template-upgrade-notifier

## Additional Setup Required!!

<!-- DELETE THIS SECTION AFTER COMPLETING SETUP -->

Additional setup might be required.
Once you do the stuff under this text, delete this section.

### Code Coverage
To setup code coverage for this project:

1. Go to [Codecov][codecov], add your project.
2. Go to [Settings -> Secrets and variables -> Actions][gh-actions-secrets] in your repo and add a repository secret named `CODECOV_TOKEN`.
3. Do the same in [Settings -> Secrets and variables -> Dependabot][gh-actions-dependabot].

The instructions on the Codecov page will provide the token to use.


### Setup API Key (crates.io)

You'll need to set up an API key to publish to `crates.io` on tag.

- Generate your API key in [crates.io (Account Settings -> API Tokens)][crates-io-key].
- Go to [Settings -> Secrets and variables -> Actions][gh-actions-secrets] in your repo and add environment secret named `CRATES_IO_TOKEN`.
- Paste your API key.




### Fill in README Documentation

This repository uses a two-level README structure:

| File | Purpose | What to Write |
|------|---------|---------------|
| `README.MD` (this file) | GitHub landing page | Project overview, quick start example, badges |
| [`src/template-upgrade-notifier/README.MD`](./src/template-upgrade-notifier/README.MD) | crates.io page | Detailed features, installation, comprehensive usage examples |

-----------------------

[![Crates.io](https://img.shields.io/crates/v/template-upgrade-notifier.svg)](https://crates.io/crates/template-upgrade-notifier)
[![Docs.rs](https://docs.rs/template-upgrade-notifier/badge.svg)](https://docs.rs/template-upgrade-notifier)
[![CI](https://github.com/Sewer56/template-upgrade-notifier/actions/workflows/rust.yml/badge.svg)](https://github.com/Sewer56/template-upgrade-notifier/actions)

A GitHub Action that scans repositories for outdated template versions and creates upgrade notification issues with LLM-powered auto-fix PRs

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
template-upgrade-notifier = "0.1.0"
```

```rust
// TODO: Add a minimal example showing the most common use case
```
See the [template-upgrade-notifier crate documentation](./src/template-upgrade-notifier/README.MD) for detailed usage.

## Crates

- [template-upgrade-notifier](./src/template-upgrade-notifier/README.MD): Core library <!-- TODO: Update description -->
- [cli](./src/cli/README.MD): Command-line interface <!-- TODO: Update description -->

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
