# xsql — User Manual

This manual helps you get started quickly with xsql, explains the TUI workflow, and shows common examples.

## Clone the repo

```bash
git clone https://github.com/Dawaman43/xsql
cd xsql
```

## Build

```bash
cargo build --release
```

The binary is at `target/release/xsql`.

## Quick usage

- Start the interactive TUI (recommended):

```bash
./target/release/xsql
```

TUI hotkeys:
- `i` — open input picker (choose `.sql` file or folder)
- `o` — open output-folder picker
- `x` — swap `from` / `to` dialects
- `d` — toggle **dry-run** (validate & show what would be converted)
- `r` — run conversion
- `Tab` / `Shift+Tab` — move focus
- `Esc` — cancel picker / exit

Workflow example:
1. Press `i` and select a folder of `.sql` files (or a single file).
2. Press `o` and choose an output folder (create if needed).
3. Press `d` to toggle dry-run and validate. If OK, press `d` again to turn it off.
4. Press `r` (or focus Run and press Enter) to perform conversion.

## CLI usage (non-interactive)

Convert one file:

```bash
./target/release/xsql --from mysql --to postgres --input schema.sql --output ./out/schema.pg.sql
```

Convert a folder recursively:

```bash
./target/release/xsql --from postgres --to sqlite --input ./schemas --output ./out
```

## Notes & limitations

- xsql focuses on a small, well-scoped portion of SQL: `CREATE TABLE` DDL mapping.
- Unknown types and dialect-specific features are mapped to conservative defaults.
- The project is designed to be extended — see `CONTRIBUTING.md`.

## Want to help?
- Add new dialect parsers/emitters
- Expand `xsql-ir` to cover indexes, constraints, and more types
- Improve TUI UX and add tests

Thank you for trying xsql — welcome contributions!
