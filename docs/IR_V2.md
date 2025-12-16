# xsql IR v2 Design

Goal
----
IR v2 provides a richer, more portable intermediate representation for SQL
schemas. It is designed to support linting, strict mode checks, schema diffing,
and eventually ALTER statement generation without breaking existing tooling.

Compatibility
-------------
- The existing `xsql-ir::Schema` (v1) remains unchanged to preserve backwards compatibility.
- IR v2 is exposed as `xsql_ir::v2::V2Schema` and friends. Parsers and emitters
  may optionally populate/consume v2 gradually.

Key Concepts
------------
- V2Schema: top-level container, contains tables and optional metadata.
- V2Table: contains columns, primary key, and table-level `Constraint`s.
- V2Column: richer column description, with `ColumnConstraint`s and `annotations`.
- ColumnConstraint: NotNull, Default, AutoIncrement, Check
- Constraint: Unique, ForeignKey, Check
- ForeignKey: name, columns, referenced_table, referenced_columns, actions

Design Principles
-----------------
- Backwards compatible: do not change v1 types. v2 exists in a separate module.
- Conservative mapping: when parsing dialects, only map well-understood constructs
  into v2; otherwise add `annotations` or `metadata` so lint can warn.
- Portability-first: store action verbs (CASCADE/SET NULL) as text and warn on
  dialect-specific behavior.

Migration Strategy
------------------
1. Add `xsql_ir::v2` module (non-breaking).
2. Update parsers to optionally populate v2 structures (incremental commits).
3. Implement `xsql lint` using v2 metadata to surface portability issues.
4. Implement `xsql diff` using v2 to compute table/column/constraint diffs.
5. Later: emit ALTER statements by mapping v2 diffs to dialects, with
   `--strict` to fail on lossy operations.

Examples
--------
A table with a unique constraint and a foreign key would appear as:

- V2Table.name = `users`
- V2Table.columns = [`id` (AutoIncrement, NotNull), `email` (NotNull)]
- V2Table.constraints contains a Unique on `email` and a ForeignKey to `accounts(id)`.

Notes for Tooling
-----------------
- Emitters must be resilient: unknown `V2DataType::Custom` should map to a
  reasonable dialect type (or emit as-is with a warning).
- Linter should provide suggestions and a `--strict` mode to turn warnings into errors.
