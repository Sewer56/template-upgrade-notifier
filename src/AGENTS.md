# template-upgrade-notifier

A GitHub Action that scans repositories for outdated template versions and creates upgrade notification issues with LLM-powered auto-fix PRs

# Project Structure

- `template-upgrade-notifier/` - Main library crate
  - `src/` - Library source code
- `cli/` - CLI executable wrapper

# Code Guidelines

- Optimize for performance; use zero-cost abstractions, avoid allocations.
- Keep modules under 500 lines (excluding tests); split if larger.
- Place `use` inside functions only for `#[cfg]` conditional compilation.
- Prefer `core` over `std` where possible (`core::mem` over `std::mem`).

# Documentation Standards

- Document public items with `///`
- Add examples in docs where helpful
- Use `//!` for module-level docs
- Focus comments on "why" not "what"
- Use [`TypeName`] rustdoc links, not backticks.

# Post-Change Verification

Always run `.cargo/verify.sh` (or `.cargo/verify.ps1` on Windows) from the workspace root after changing code.