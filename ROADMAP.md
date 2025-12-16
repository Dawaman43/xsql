# xsql Roadmap

## Phase 1: Standard Schema Converter (now)
- [x] IR v2: richer constraints (FK, UNIQUE, NOT NULL, DEFAULT, CHECK)
- [x] Strict mode: `--strict` for lossless conversion
- [x] Lint: `xsql lint` for portability warnings
- [x] Diff: `xsql diff` for schema changes

## Phase 2: Migration Assistant
- [ ] ALTER TABLE generation from IR diffs
- [ ] `xsql diff --emit-migration` for SQL migration scripts
- [ ] Table/column/constraint diff details

## Phase 3: Enterprise Dialects
- [ ] Add MSSQL parser/emitter (read-only first)
- [ ] Add Oracle, Snowflake, BigQuery, Redshift (read-only)
- [ ] Partial mapping + warnings for unsupported features

## Phase 4: Machine-Readable Output
- [ ] `xsql convert schema.sql --to ir.json` (JSON IR export)
- [ ] JSON schema for IR v2
- [ ] CI/CD integration examples

## Phase 5: Plugin System
- [ ] `xsql plugin install ...` (extension loading)
- [ ] Custom type mappings, company rules, private plugins
- [ ] Plugin API docs

## Phase 6: Ecosystem & Integrations
- [ ] VS Code/JetBrains extension (inline warnings, convert/diff)
- [ ] Web dashboard/IDE plugin
- [ ] Human-readable reports (`xsql report`)
- [ ] Schema contracts (`xsql contract`)

## Principles
- Never become an ORM
- Focus on schema portability, not data migration
- Embrace partial support + warnings
- Be boring, correct, predictable
- Backward compatibility via versioned IR

---

This roadmap is based on proven adoption patterns for developer tools and direct feedback from enterprise users. Contributions and feedback welcome!
