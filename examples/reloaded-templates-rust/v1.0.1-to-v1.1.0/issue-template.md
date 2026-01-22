## Template Upgrade Available

A new version of `reloaded-templates-rust` is available for this repository.

| Current | Available |
|---------|-----------|
| `{{old_string}}` | `{{new_string}}` |

{{#if migration_guide_link}}
### What's New in v1.1.0

- Faster CI documentation builds with `--no-deps`
- New verification scripts (`.cargo/verify.sh` and `.cargo/verify.ps1`)
- Simplified `AGENTS.md` post-change verification
- Auto-delete merged branches enabled

See the [migration guide]({{migration_guide_link}}) for step-by-step upgrade instructions.
{{/if}}

{{#if (eq pr_status "created")}}
### Automated Fix

An automated PR has been created to apply this upgrade: {{pr_link}}

Please review the changes before merging.
{{else if (eq pr_status "failed")}}
### Manual Upgrade Required

Automated PR generation was attempted but failed. Please follow the [migration guide]({{migration_guide_link}}) to apply the upgrade manually.
{{/if}}

---

*This issue was created by [template-upgrade-notifier](https://github.com/Sewer56/template-upgrade-notifier).*
