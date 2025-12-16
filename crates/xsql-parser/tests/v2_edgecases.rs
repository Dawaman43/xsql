use xsql_parser::v2::*;

#[test]
fn parse_postgres_composite_pk_and_fk() {
    let sql = r#"
    CREATE TABLE parent (id BIGINT PRIMARY KEY);
    CREATE TABLE child (
        a INT,
        b INT,
        PRIMARY KEY (a, b),
        CONSTRAINT fk_parent FOREIGN KEY (a) REFERENCES parent(id) ON DELETE CASCADE
    );
    "#;

    let schema = parse_postgres_schema_v2(sql).expect("parse v2");
    assert_eq!(schema.tables.len(), 2);
    let child = schema
        .tables
        .iter()
        .find(|t| t.name == "child")
        .expect("child");
    assert_eq!(child.primary_key.as_ref().unwrap().len(), 2);
    assert!(child
        .constraints
        .iter()
        .any(|c| matches!(c, xsql_ir::v2::Constraint::ForeignKey(_))));
}

#[test]
fn parse_mysql_check_and_unique_multi() {
    let sql = r#"
    CREATE TABLE t (
        x INT,
        y INT,
        UNIQUE KEY ux_xy (x,y),
        CHECK (x > 0 AND y > 0)
    );
    "#;
    let schema = parse_mysql_schema_v2(sql).expect("parse v2");
    let t = schema.tables.iter().find(|t| t.name == "t").expect("t");
    assert!(t
        .constraints
        .iter()
        .any(|c| matches!(c, xsql_ir::v2::Constraint::Unique { .. })));
    assert!(t
        .constraints
        .iter()
        .any(|c| matches!(c, xsql_ir::v2::Constraint::Check { .. })));
}

#[test]
fn parse_sqlite_autoinc_and_default() {
    let sql = r#"
    CREATE TABLE s (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        created_at TEXT DEFAULT (CURRENT_TIMESTAMP)
    );
    "#;
    let schema = parse_sqlite_schema_v2(sql).expect("parse v2");
    let s = schema.tables.iter().find(|t| t.name == "s").expect("s");
    assert!(s.columns.iter().any(|c| c
        .constraints
        .iter()
        .any(|cc| matches!(cc, xsql_ir::v2::ColumnConstraint::AutoIncrement))));
    assert!(s.columns.iter().any(|c| c
        .constraints
        .iter()
        .any(|cc| matches!(cc, xsql_ir::v2::ColumnConstraint::Default(_)))));
}
