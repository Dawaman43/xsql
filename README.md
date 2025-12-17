# xsql

Convert SQL *schema DDL* between dialects (currently focused on `CREATE TABLE ...`) via a small intermediate representation.

One-line positioning: a lightweight CLI/TUI to parse, diff and emit SQL schema DDL across MySQL/Postgres/SQLite using an experimental IR v2.

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

## IR v2 (experimental)

IR v2 is an opt-in richer intermediate representation for schema portability. It captures:

- Table-level constraints (`UNIQUE`, `FOREIGN KEY`, `CHECK`)
- Column-level constraints (`NOT NULL`, `DEFAULT`, `AUTO_INCREMENT`/identity)
- Portable data types (`Integer`, `Boolean`, `Float`, `Varchar`, `Text`, `Timestamp`)
- Free-form `metadata` and `annotations` for tools to add hints

Parsers can output V2 JSON and emitters can read V2 JSON to produce dialect SQL.
This is useful for round-tripping, linting, and future ALTER-generation.

CLI examples (v2):

```bash
# Parse a Postgres SQL file into V2 JSON
xsql v2-parse schema.sql --dialect postgres > schema.v2.json

# Emit MySQL SQL from a V2 JSON file
xsql v2-emit schema.v2.json --dialect mysql > schema.mysql.sql

# Diff two V2 JSON files (human output)
xsql v2-diff old.v2.json new.v2.json

# Diff two V2 JSON files (machine JSON output)
xsql v2-diff old.v2.json new.v2.json --json > diff.json
```

Notes and limitations:

- V2 is still experimental: some vendor-specific types and complex CHECK expressions
  may be preserved as `Custom`/`annotations` and not always round-trippable.
- ALTER-statement generation is planned; currently emitters output `CREATE TABLE`.


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

## CI snippet (example)

Use `v2-parse`/`v2-emit` in CI to validate schema round-trips and detect portability issues. Example GitHub Actions step:

```yaml
- name: Validate schema roundtrip
  uses: actions/checkout@v4

- name: Setup Rust
  uses: actions-rs/toolchain@v1
  with:
    toolchain: stable

- name: Build xsql
  run: cargo build --release --manifest-path crates/xsql-cli/Cargo.toml

- name: Parse and emit (roundtrip)
  run: |
    ./target/release/xsql v2-parse examples/postgres.sql --dialect postgres > /tmp/schema.v2.json
    ./target/release/xsql v2-emit /tmp/schema.v2.json --dialect mysql > /tmp/schema.mysql.sql

- name: Compute v2 diff
  run: |
    ./target/release/xsql v2-parse examples/postgres.sql --dialect postgres > /tmp/old.v2.json
    ./target/release/xsql v2-parse examples/mysql.sql --dialect mysql > /tmp/new.v2.json
    ./target/release/xsql v2-diff /tmp/old.v2.json /tmp/new.v2.json --json > /tmp/diff.json
```

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
