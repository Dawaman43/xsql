mod tui;

use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Dialect {
    Mysql,
    Postgres,
    Sqlite,
}

impl Dialect {
    fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "mysql" => Ok(Dialect::Mysql),
            "postgres" | "postgresql" | "pg" => Ok(Dialect::Postgres),
            "sqlite" | "sqlite3" => Ok(Dialect::Sqlite),
            other => Err(format!(
                "unknown dialect '{other}'. Supported: mysql, postgres, sqlite"
            )),
        }
    }
}

#[derive(Debug, Clone, Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    convert: ConvertArgs,
}

#[derive(Debug, Clone, Subcommand)]
enum Command {
    /// Convert SQL between dialects
    Convert(ConvertArgs),
    /// Interactive terminal UI
    Tui,
    /// Lint a schema for portability issues
    Lint(LintArgs),
    /// Diff two schemas (basic table-level diff)
    Diff(DiffArgs),
    /// IR v2: parse a SQL file into V2 JSON
    V2Parse {
        /// Input SQL file to parse
        #[arg(value_name = "FILE")]
        input: PathBuf,

        /// Dialect of the input SQL (mysql|postgres|sqlite). Defaults to postgres.
        #[arg(long)]
        dialect: Option<String>,
    },
    /// IR v2: emit SQL from a V2 JSON file
    V2Emit {
        /// Input V2 JSON file
        #[arg(value_name = "FILE")]
        input: PathBuf,

        /// Dialect to emit (mysql|postgres|sqlite). Defaults to postgres.
        #[arg(long)]
        dialect: Option<String>,
    },
}

#[derive(Debug, Clone, clap::Args)]
struct ConvertArgs {
    /// Input SQL file or a directory to convert recursively
    #[arg(long)]
    input: Option<String>,

    /// Output SQL file or a directory where results will be written
    #[arg(long)]
    output: Option<String>,

    /// Source dialect: mysql | postgres | sqlite
    #[arg(long, default_value = "mysql")]
    from: String,

    /// Destination dialect: mysql | postgres | sqlite
    #[arg(long, default_value = "postgres")]
    to: String,
    /// Fail on any portability warning
    #[arg(long)]
    strict: bool,
}

#[derive(Debug, Clone, clap::Args)]
struct LintArgs {
    /// Input SQL file
    #[arg(value_name = "FILE")]
    input: PathBuf,

    /// Dialect for input SQL: mysql | postgres | sqlite
    #[arg(long, default_value = "mysql")]
    from: String,

    /// Optional target dialect to lint against: mysql | postgres | sqlite
    #[arg(long)]
    to: Option<String>,

    /// Fail on any portability warning
    #[arg(long)]
    strict: bool,
}

#[derive(Debug, Clone, clap::Args)]
struct DiffArgs {
    /// Old schema SQL file
    #[arg(value_name = "OLD")]
    old: PathBuf,

    /// New schema SQL file
    #[arg(value_name = "NEW")]
    new: PathBuf,

    /// Dialect used to parse both inputs: mysql | postgres | sqlite
    #[arg(long, default_value = "postgres")]
    dialect: String,
}

fn parse_schema(from: Dialect, sql: &str) -> Result<xsql_ir::Schema, String> {
    match from {
        Dialect::Mysql => xsql_parser::mysql::parse_mysql_schema(sql).map_err(|e| e.to_string()),
        Dialect::Postgres => {
            xsql_parser::postgres::parse_postgres_schema(sql).map_err(|e| e.to_string())
        }
        Dialect::Sqlite => xsql_parser::sqlite::parse_sqlite_schema(sql).map_err(|e| e.to_string()),
    }
}

fn emit_schema(to: Dialect, schema: &xsql_ir::Schema) -> String {
    match to {
        Dialect::Mysql => xsql_emitter::mysql::emit_mysql(schema),
        Dialect::Postgres => xsql_emitter::postgres::emit_postgres(schema),
        Dialect::Sqlite => xsql_emitter::sqlite::emit_sqlite(schema),
    }
}

pub(crate) fn plan_conversion(
    from: Dialect,
    to: Dialect,
    input: &Path,
    output_dir: &Path,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let mut mappings = Vec::new();

    if input.is_dir() {
        if !input.is_dir() {
            return Err(format!("input is not a directory: {}", input.display()));
        }

        for entry in fs::read_dir(input)
            .map_err(|e| format!("failed to read dir {}: {e}", input.display()))?
        {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                // Recurse into subdirectories
                let sub = plan_conversion(from, to, &path, output_dir)?;
                mappings.extend(sub);
                continue;
            }

            if !is_sql_file(&path) {
                continue;
            }

            // Validate parse
            let sql = fs::read_to_string(&path)
                .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
            parse_schema(from, &sql)
                .map_err(|e| format!("failed to parse {}: {e}", path.display()))?;

            let rel = path.strip_prefix(input).map_err(|e| e.to_string())?;
            let out_path = output_dir.join(rel);
            mappings.push((path, out_path));
        }
    } else {
        if !input.exists() {
            return Err(format!("input does not exist: {}", input.display()));
        }

        if !is_sql_file(input) {
            // still try to parse single file
        }

        let sql = fs::read_to_string(input)
            .map_err(|e| format!("failed to read {}: {e}", input.display()))?;
        parse_schema(from, &sql)
            .map_err(|e| format!("failed to parse {}: {e}", input.display()))?;

        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("schema");
        let out = output_dir.join(format!(
            "{}.{}.sql",
            stem,
            match to {
                Dialect::Mysql => "mysql",
                Dialect::Postgres => "postgres",
                Dialect::Sqlite => "sqlite",
            }
        ));

        mappings.push((input.to_path_buf(), out));
    }

    Ok(mappings)
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create output dir: {e}"))?;
    }
    Ok(())
}

fn is_sql_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("sql"))
}

pub(crate) fn convert_file(
    from: Dialect,
    to: Dialect,
    input: &Path,
    output: &Path,
) -> Result<(), String> {
    convert_file_with_options(from, to, input, output, ConvertOptions { strict: false })
}

#[derive(Debug, Clone, Copy)]
struct ConvertOptions {
    strict: bool,
}

fn conversion_warnings(from: Dialect, to: Dialect, schema: &xsql_ir::Schema) -> Vec<String> {
    let mut warnings = Vec::new();

    // Always run generic checks; target-specific checks are gated below.

    for table in &schema.tables {
        for col in &table.columns {
            if col.auto_increment {
                warnings.push(format!(
                    "{}.{} uses auto-increment; semantics can differ across dialects",
                    table.name, col.name
                ));
            }

            if let Some(def) = &col.default {
                let def_l = def.to_lowercase();
                if def_l.contains("now()") || def_l.contains("current_timestamp") {
                    warnings.push(format!(
                        "{}.{} DEFAULT {} may be dialect-specific",
                        table.name, col.name, def
                    ));
                }
            }

            if matches!(to, Dialect::Sqlite) {
                match col.data_type {
                    xsql_ir::DataType::Timestamp
                    | xsql_ir::DataType::Varchar(_)
                    | xsql_ir::DataType::Text => {
                        warnings.push(format!(
                            "{}.{} type {:?} will be emitted as TEXT in SQLite",
                            table.name, col.name, col.data_type
                        ));
                    }
                    _ => {}
                }
            }
        }
    }

    warnings
}

fn convert_file_with_options(
    from: Dialect,
    to: Dialect,
    input: &Path,
    output: &Path,
    options: ConvertOptions,
) -> Result<(), String> {
    let sql = fs::read_to_string(input)
        .map_err(|e| format!("failed to read input {}: {e}", input.display()))?;
    let schema = parse_schema(from, &sql)
        .map_err(|e| format!("failed to parse {}: {e}", input.display()))?;

    let warnings = conversion_warnings(from, to, &schema);
    if options.strict && !warnings.is_empty() {
        return Err(format!(
            "strict mode: refusing potentially lossy conversion for {}:\n{}",
            input.display(),
            warnings
                .into_iter()
                .map(|w| format!("- {w}"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    let out = emit_schema(to, &schema);
    ensure_parent_dir(output)?;
    fs::write(output, out)
        .map_err(|e| format!("failed to write output {}: {e}", output.display()))?;
    Ok(())
}

pub(crate) fn convert_dir(
    from: Dialect,
    to: Dialect,
    input_dir: &Path,
    output_dir: &Path,
) -> Result<(), String> {
    convert_dir_with_options(from, to, input_dir, output_dir, ConvertOptions { strict: false })
}

fn convert_dir_with_options(
    from: Dialect,
    to: Dialect,
    input_dir: &Path,
    output_dir: &Path,
    options: ConvertOptions,
) -> Result<(), String> {
    if !input_dir.is_dir() {
        return Err(format!("input is not a directory: {}", input_dir.display()));
    }

    let mut stack = vec![input_dir.to_path_buf()];
    let mut converted = 0usize;
    let mut failed = 0usize;

    while let Some(dir) = stack.pop() {
        for entry in
            fs::read_dir(&dir).map_err(|e| format!("failed to read dir {}: {e}", dir.display()))?
        {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if !is_sql_file(&path) {
                continue;
            }

            let rel = path.strip_prefix(input_dir).map_err(|e| e.to_string())?;
            let out_path = output_dir.join(rel);

            match convert_file_with_options(from, to, &path, &out_path, options) {
                Ok(()) => converted += 1,
                Err(err) => {
                    failed += 1;
                    eprintln!("✖ xsql: {err}");
                }
            }
        }
    }

    if failed > 0 {
        return Err(format!("converted {converted} files, {failed} failed"));
    }
    Ok(())
}

fn convert_args_to_paths(args: &ConvertArgs) -> Result<(PathBuf, PathBuf), String> {
    let input = args
        .input
        .as_ref()
        .ok_or_else(|| "--input is required".to_string())?;
    let output = args
        .output
        .as_ref()
        .ok_or_else(|| "--output is required".to_string())?;
    Ok((PathBuf::from(input), PathBuf::from(output)))
}

fn lint_schema_file(from: Dialect, to: Option<Dialect>, input: &Path, strict: bool) -> Result<(), String> {
    let sql = fs::read_to_string(input)
        .map_err(|e| format!("failed to read {}: {e}", input.display()))?;
    let schema = parse_schema(from, &sql).map_err(|e| format!("parse error: {e}"))?;

    // Without a target dialect, we still run generic checks (auto-increment/defaults).
    let warnings = match to {
        Some(target) => conversion_warnings(from, target, &schema),
        None => conversion_warnings(from, from, &schema),
    };

    if warnings.is_empty() {
        println!("✔ xsql: lint OK (no portability warnings)");
        return Ok(());
    }

    eprintln!("xsql: portability warnings:");
    for w in &warnings {
        eprintln!("- {w}");
    }

    if strict {
        return Err("lint failed under --strict".to_string());
    }

    Ok(())
}

fn diff_schemas(dialect: Dialect, old: &Path, new: &Path) -> Result<(), String> {
    let sold = fs::read_to_string(old)
        .map_err(|e| format!("failed to read {}: {e}", old.display()))?;
    let snew = fs::read_to_string(new)
        .map_err(|e| format!("failed to read {}: {e}", new.display()))?;
    let old_schema = parse_schema(dialect, &sold).map_err(|e| format!("parse old: {e}"))?;
    let new_schema = parse_schema(dialect, &snew).map_err(|e| format!("parse new: {e}"))?;

    use std::collections::{BTreeMap, BTreeSet};
    let old_map: BTreeMap<_, _> = old_schema
        .tables
        .iter()
        .map(|t| (t.name.clone(), t.clone()))
        .collect();
    let new_map: BTreeMap<_, _> = new_schema
        .tables
        .iter()
        .map(|t| (t.name.clone(), t.clone()))
        .collect();

    let old_tables: BTreeSet<_> = old_map.keys().cloned().collect();
    let new_tables: BTreeSet<_> = new_map.keys().cloned().collect();

    let added_tables: Vec<_> = new_tables.difference(&old_tables).cloned().collect();
    let removed_tables: Vec<_> = old_tables.difference(&new_tables).cloned().collect();

    if !added_tables.is_empty() {
        println!("Added tables:");
        for t in &added_tables {
            println!("- {t}");
        }
    }
    if !removed_tables.is_empty() {
        println!("Removed tables:");
        for t in &removed_tables {
            println!("- {t}");
        }
    }

    let mut any_changes = !(added_tables.is_empty() && removed_tables.is_empty());

    for tname in old_tables.intersection(&new_tables) {
        let a = old_map.get(tname).expect("table exists");
        let b = new_map.get(tname).expect("table exists");

        let a_cols: BTreeMap<_, _> = a
            .columns
            .iter()
            .map(|c| (c.name.clone(), c.clone()))
            .collect();
        let b_cols: BTreeMap<_, _> = b
            .columns
            .iter()
            .map(|c| (c.name.clone(), c.clone()))
            .collect();

        let a_names: BTreeSet<_> = a_cols.keys().cloned().collect();
        let b_names: BTreeSet<_> = b_cols.keys().cloned().collect();

        let added_cols: Vec<_> = b_names.difference(&a_names).cloned().collect();
        let removed_cols: Vec<_> = a_names.difference(&b_names).cloned().collect();
        let mut changed_cols = Vec::new();

        for cname in a_names.intersection(&b_names) {
            let ca = a_cols.get(cname).expect("col exists");
            let cb = b_cols.get(cname).expect("col exists");
            if ca.data_type != cb.data_type
                || ca.nullable != cb.nullable
                || ca.auto_increment != cb.auto_increment
                || ca.default != cb.default
            {
                changed_cols.push(cname.clone());
            }
        }

        let pk_changed = a.primary_key != b.primary_key;
        if added_cols.is_empty() && removed_cols.is_empty() && changed_cols.is_empty() && !pk_changed {
            continue;
        }

        any_changes = true;
        println!("Table {tname}:");
        if !added_cols.is_empty() {
            println!("  Added columns:");
            for c in added_cols {
                println!("  - {c}");
            }
        }
        if !removed_cols.is_empty() {
            println!("  Removed columns:");
            for c in removed_cols {
                println!("  - {c}");
            }
        }
        if !changed_cols.is_empty() {
            println!("  Changed columns:");
            for c in changed_cols {
                let ca = a_cols.get(&c).expect("col exists");
                let cb = b_cols.get(&c).expect("col exists");
                println!(
                    "  - {c}: {:?} -> {:?}, nullable {} -> {}, default {:?} -> {:?}, autoinc {} -> {}",
                    ca.data_type,
                    cb.data_type,
                    ca.nullable,
                    cb.nullable,
                    ca.default,
                    cb.default,
                    ca.auto_increment,
                    cb.auto_increment
                );
            }
        }
        if pk_changed {
            println!("  Primary key: {:?} -> {:?}", a.primary_key, b.primary_key);
        }
    }

    if !any_changes {
        println!("✔ xsql: schemas are equivalent (within current IR support)");
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Tui) => {
            if let Err(e) = tui::run_tui() {
                eprintln!("✖ xsql: {e}");
                std::process::exit(1);
            }
        }
        Some(Command::Convert(args)) => {
            let from = Dialect::parse(&args.from).unwrap_or_else(|e| panic!("{e}"));
            let to = Dialect::parse(&args.to).unwrap_or_else(|e| panic!("{e}"));
            let (input, output) = match convert_args_to_paths(&args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("✖ xsql: {e}");
                    std::process::exit(2);
                }
            };

            let opts = ConvertOptions { strict: args.strict };

            let result = if input.is_dir() {
                convert_dir_with_options(from, to, &input, &output, opts)
            } else {
                convert_file_with_options(from, to, &input, &output, opts)
            };

            match result {
                Ok(()) => println!("✔ xsql: conversion complete"),
                Err(e) => {
                    eprintln!("✖ xsql: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Command::Lint(args)) => {
            let from = Dialect::parse(&args.from).unwrap_or_else(|e| panic!("{e}"));
            let to = args
                .to
                .as_deref()
                .map(Dialect::parse)
                .transpose()
                .unwrap_or_else(|e| panic!("{e}"));
            match lint_schema_file(from, to, &args.input, args.strict) {
                Ok(()) => println!("✔ xsql: lint completed"),
                Err(e) => {
                    eprintln!("✖ xsql: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Command::Diff(args)) => {
            let dialect = Dialect::parse(&args.dialect).unwrap_or_else(|e| panic!("{e}"));
            match diff_schemas(dialect, &args.old, &args.new) {
                Ok(()) => println!("✔ xsql: diff completed"),
                Err(e) => {
                    eprintln!("✖ xsql: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Command::V2Parse { input, dialect }) => {
            let sql = fs::read_to_string(&input).expect("failed to read input");
            let schema = match dialect
                .as_deref()
                .map(|d| Dialect::parse(d).map_err(|e| e.to_string()))
            {
                Some(Ok(Dialect::Mysql)) => xsql_parser::v2::parse_mysql_schema_v2(&sql),
                Some(Ok(Dialect::Sqlite)) => xsql_parser::v2::parse_sqlite_schema_v2(&sql),
                _ => xsql_parser::v2::parse_postgres_schema_v2(&sql),
            };

            match schema {
                Ok(s) => {
                    let json = serde_json::to_string_pretty(&s).expect("serialize v2");
                    println!("{}", json);
                }
                Err(e) => {
                    eprintln!("✖ xsql: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Command::V2Emit { input, dialect }) => {
            let json = fs::read_to_string(&input).expect("failed to read input");
            let schema: xsql_ir::v2::V2Schema = serde_json::from_str(&json).expect("parse v2 json");
            let sql = match dialect
                .as_deref()
                .map(|d| Dialect::parse(d).map_err(|e| e.to_string()))
            {
                Some(Ok(Dialect::Mysql)) => xsql_emitter::v2::emit_mysql_v2(&schema),
                Some(Ok(Dialect::Sqlite)) => xsql_emitter::v2::emit_sqlite_v2(&schema),
                _ => xsql_emitter::v2::emit_postgres_v2(&schema),
            };
            println!("{}", sql);
        }
        None => {
            // Default UX: `xsql` starts the TUI.
            // Back-compat: if user passed `--input/--output`, run conversion directly.
            let has_paths = cli.convert.input.is_some() || cli.convert.output.is_some();
            if !has_paths {
                if let Err(e) = tui::run_tui() {
                    eprintln!("✖ xsql: {e}");
                    std::process::exit(1);
                }
                return;
            }

            let from = Dialect::parse(&cli.convert.from).unwrap_or_else(|e| panic!("{e}"));
            let to = Dialect::parse(&cli.convert.to).unwrap_or_else(|e| panic!("{e}"));
            let (input, output) = match convert_args_to_paths(&cli.convert) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("✖ xsql: {e}");
                    std::process::exit(2);
                }
            };

            let opts = ConvertOptions {
                strict: cli.convert.strict,
            };

            let result = if input.is_dir() {
                convert_dir_with_options(from, to, &input, &output, opts)
            } else {
                convert_file_with_options(from, to, &input, &output, opts)
            };

            match result {
                Ok(()) => println!("✔ xsql: conversion complete"),
                Err(e) => {
                    eprintln!("✖ xsql: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}
