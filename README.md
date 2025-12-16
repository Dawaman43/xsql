# xsql

Convert SQL *schema DDL* between dialects (currently focused on `CREATE TABLE ...`) via a small intermediate representation.

## What it does (today)

- Parses schema DDL from:
  - MySQL
  - PostgreSQL
  - SQLite
- Emits schema DDL to:
  - MySQL
  - PostgreSQL
  - SQLite
- Converts:
  - a single `.sql` file → another `.sql` file
  - a whole folder tree of `.sql` files → a destination folder (preserves relative paths)
- Includes an interactive TUI (`xsql tui`).

## Quickstart
Install and run quickly (recommended):

```bash
# one-line install (clones, builds, installs to $HOME/.cargo/bin)
curl -fsSL https://raw.githubusercontent.com/Dawaman43/xsql/main/install.sh | sh

# then run the tool
xsql --help
```

## Build from source

If you prefer to build locally:

```bash
cargo build --release
```

The development binary will be at `target/release/xsql`.

There is also an installer script at `install.sh` in this repo; the curl one-liner above runs it.

## CLI usage

### Start the TUI (default)

```bash
xsql
```

Or explicitly:

```bash
xsql tui
```

### Convert one file

```bash
xsql --from mysql --to postgres --input schema.sql --output schema.pg.sql
```

If you installed via the one-liner the binary will be available as `xsql` in your shell.

### Convert a folder (recursive)

```bash
xsql --from postgres --to sqlite --input ./schemas --output ./out
```

Only files ending in `.sql` are converted; others are ignored.

### Use the TUI

```bash
xsql tui
```

- `Tab` / `Shift+Tab` moves between fields
- `↑/↓` or `←/→` changes dialect when focused on From/To
- `i` opens input picker, `o` opens output picker
- `x` swaps from/to dialects, `r` runs conversion
- `Enter` runs conversion when focused on Run
- If the output folder doesn't exist, the TUI will ask to create it (`y/n`).
- `Esc` quits

TUI workflow is designed to be “no typing”:

1. Pick input file or folder
2. Pick output folder
3. Press Enter to run

## Important limitations

This project currently focuses on a *small subset* of SQL needed for schema conversion:

- Only `CREATE TABLE` is converted.
- The intermediate representation is intentionally minimal (`xsql-ir`).
- Many dialect-specific features (indexes, constraints beyond primary keys, foreign keys, check constraints, extensions, etc.) are not yet mapped.
- Type mapping is best-effort (unknown types become `TEXT`).

If you want broader coverage (views, inserts, queries, indexes, constraints), the next step is expanding `xsql-ir` and adding emit/parse support accordingly.

## Roadmap (realistic)

- Expand IR to cover more schema features (constraints, indexes)
- Improve type mapping and defaults across dialects
- Add more dialects (e.g. MSSQL, BigQuery, Snowflake) as the IR grows

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT. See [LICENSE](LICENSE).
