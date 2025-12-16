# Changelog

## [v0.2.0] - 2025-12-16

### Added
- IR v2: new richer schema representation (`crates/xsql-ir/src/v2.rs`) with `V2Schema`, `V2Table`, `V2Column`, `V2DataType`, constraints, and FK support.
- `v2` CLI workflow: `v2-parse` and `v2-emit` for parsing SQL -> V2 JSON and emitting SQL from V2 JSON (`crates/xsql-cli`).
- `--strict` mode for `v2-emit` (and conversion/lint), which fails on lossy mappings.
- `v2-diff` library and CLI: basic structural diff of two V2 schemas (`crates/xsql-ir::v2_diff` and `xsql v2-diff`).
- ALTER-generation scaffold (`crates/xsql-ir/src/alter.rs`) for future ALTER SQL output.
- Tests: parser/emitter v2 roundtrip and edge-case tests.
- CI: GitHub Actions workflow to run `fmt`, `clippy`, and tests.

### Fixed
- Formatting and clippy issues across the workspace.
- CLI `v2-emit` strict flag wiring and pattern-match bug.

### Notes
- IR v2 is experimental. The `V2Schema` JSON format is considered unstable and may evolve; use `--strict` to ensure lossless emit where possible.

