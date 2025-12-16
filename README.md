# xsql

Convert SQL *schema DDL* (currently `CREATE TABLE ...`) between dialects via a small intermediate representation.

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

## Install / build

```bash
cargo build --release
```

Binary will be at `target/release/xsql`.

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
- `Enter` runs conversion when focused on Run
- If the output folder doesn't exist, the TUI will ask to create it (`y/n`).
- `Esc` quits

## Important limitations

This project currently focuses on a *small subset* of SQL needed for schema conversion:

- Only `CREATE TABLE` is converted.
- The intermediate representation is intentionally minimal (`xsql-ir`).
- Many dialect-specific features (indexes, constraints beyond primary keys, foreign keys, check constraints, extensions, etc.) are not yet mapped.
- Type mapping is best-effort (unknown types become `TEXT`).

If you want broader coverage (views, inserts, queries, indexes, constraints), the next step is expanding `xsql-ir` and adding emit/parse support accordingly.

## License

MIT. See [LICENSE](LICENSE).
