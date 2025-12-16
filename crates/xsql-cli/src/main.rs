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
    let sql = fs::read_to_string(input)
        .map_err(|e| format!("failed to read input {}: {e}", input.display()))?;
    let schema = parse_schema(from, &sql)
        .map_err(|e| format!("failed to parse {}: {e}", input.display()))?;
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

            match convert_file(from, to, &path, &out_path) {
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

            let result = if input.is_dir() {
                convert_dir(from, to, &input, &output)
            } else {
                convert_file(from, to, &input, &output)
            };

            match result {
                Ok(()) => println!("✔ xsql: conversion complete"),
                Err(e) => {
                    eprintln!("✖ xsql: {e}");
                    std::process::exit(1);
                }
            }
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

            let result = if input.is_dir() {
                convert_dir(from, to, &input, &output)
            } else {
                convert_file(from, to, &input, &output)
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
