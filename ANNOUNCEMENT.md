# Announcement / Post Drafts

Show HN / Reddit draft

Title: xsql — convert SQL schema DDL between MySQL/Postgres/SQLite (TUI + CLI)

Body:
I've been working on xsql, a small Rust tool to convert SQL schema DDL between MySQL, PostgreSQL and SQLite.

Why:
- Schema DDL is often the hardest part when moving or syncing simple schemas between databases. xsql parses dialect-specific CREATE TABLE statements into a small IR and emits target-dialect SQL.

What you get:
- CLI + interactive TUI for file/folder conversion
- Batch folder conversion preserving relative paths
- One-line installer and packaged binaries

Limitations:
- Focused on CREATE TABLE and core types — not a full migration tool yet.

Repo: https://github.com/Dawaman43/xsql

—

Reddit title (r/rust or r/opensource): xsql: IR-based SQL schema converter (MySQL ⇄ Postgres ⇄ SQLite) — TUI + CLI
