# Contributing to xsql

Thanks for helping improve xsql.

## Scope (what we accept)

- Parsers/emitters for additional dialects
- Better schema coverage (`CREATE TABLE`, constraints, indexes, etc.)
- Bug fixes and correctness improvements
- UX improvements to the CLI/TUI that keep the tool simple

## Development

```bash
cargo fmt
cargo check
```

## Testing

If you add behavior, please add a small test or example if the workspace introduces a test harness later.

## Pull requests

- Keep PRs focused.
- Include before/after examples of SQL when relevant.
- Mention which dialects you tested.
