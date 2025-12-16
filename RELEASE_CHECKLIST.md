# Release checklist

This file documents the minimal steps to prepare a `v0.1.0` release.

- [ ] Verify `cargo fmt` and `cargo check -p xsql` pass.
- [ ] Run unit/integration tests (if any).
- [ ] Update `Cargo.toml` versions for each crate (xsql, xsql-cli, xsql-ir, xsql-parser, xsql-emitter, xsql-engine) as appropriate.
- [ ] Update `CHANGELOG.md` or `RELEASE_NOTES.md` (draft included).
- [ ] Build release binaries for target platforms (at minimum: `x86_64-unknown-linux-gnu`).
- [ ] Create signed Git tag `v0.1.0`.
- [ ] Create GitHub release `v0.1.0` with release notes and attach binaries.
- [ ] Add GitHub topics: `rust`, `sql`, `database`, `cli`, `tui`, `schema-migration`.
- [ ] Push release and announce (Show HN, Reddit, Twitter, Rust channels).
