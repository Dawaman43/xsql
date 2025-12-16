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

# xsql

Convert SQL *schema DDL* between dialects (focused on `CREATE TABLE`) via a small intermediate representation (IR).

## Quickstart

Install and run quickly (recommended):

```bash
# one-line installer (clones, builds, installs xsql)
curl -fsSL https://raw.githubusercontent.com/Dawaman43/xsql/main/install.sh | sh

# then run the tool
xsql --help
```

## Design

### Architecture

`xsql` uses a small intermediate representation (IR) to separate parsing from emitting:

- Parser crates (in `crates/xsql-parser`) read dialect-specific SQL into the IR.
- The IR crate (`crates/xsql-ir`) contains a minimal typed model for tables, columns and basic constraints.
- Emitter crates (in `crates/xsql-emitter`) produce dialect-specific SQL from the IR.

This design makes it straightforward to add new dialects and to centralize mapping logic between types and constraints.

### Why an IR?

- Bidirectional conversion: parse → IR → emit avoids ad-hoc text transforms.
- Easier testing: round-trip tests (parse → IR → emit → parse) validate correctness.
- Incremental growth: start with CREATE TABLE and expand IR for indexes, constraints and more.

## Build from source

If you prefer to build locally:

```bash
cargo build --release --manifest-path crates/xsql-cli/Cargo.toml
```

The development binary will be at `crates/xsql-cli/target/release/xsql` (or in the workspace `target/release`).

There is also an installer script at `install.sh` in this repo; the curl one-liner above runs it.

## Installer notes

- The installer runs a quiet build and shows a spinner animation while compiling.
- It will try to move the installed `xsql` binary into `/usr/local/bin` (this may prompt for `sudo`) so the tool is immediately available system-wide.
- If moving to `/usr/local/bin` is not possible, the installer adds `$HOME/.cargo/bin` to your shell RC (prefers `~/.zshrc`, then `~/.bashrc`, then `~/.profile`) so `xsql` will be available in future sessions.
- If the installer cannot move the binary, you can make it available immediately in your current shell by running:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
# or source the updated rc, e.g.:
source ~/.zshrc
```

**Do not run the curl+sh command with `sudo`** — the installer handles any required privilege escalation when moving the binary to `/usr/local/bin`.

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
- `i` opens input picker, `o` opens output picker
- `x` swaps from/to dialects, `r` runs conversion
- `Enter` runs conversion when focused on Run
- If the output folder doesn't exist, the TUI will ask to create it (`y/n`).
- `Esc` quits

TUI workflow is designed to be “no typing”:

1. Pick input file or folder
2. Pick output folder
3. Press Enter to run

## Examples

See the `examples/` folder for small sample schemas (`mysql.sql`, `postgres.sql`, `sqlite.sql`).

## Important limitations

This project currently focuses on a *small subset* of SQL needed for schema conversion:

- Only `CREATE TABLE` is converted.
- The intermediate representation is intentionally minimal (`xsql-ir`).
- Many dialect-specific features (indexes, constraints beyond primary keys, foreign keys, check constraints, extensions, etc.) are not yet mapped.
- Type mapping is best-effort (unknown types become `TEXT`).

## Roadmap (realistic)

- Expand IR to cover more schema features (constraints, indexes)
- Improve type mapping and defaults across dialects
- Add more dialects (e.g. MSSQL, BigQuery, Snowflake) as the IR grows

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT. See [LICENSE](LICENSE).
