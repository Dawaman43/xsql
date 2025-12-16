use xsql_emitter::v2::*;
use xsql_ir::v2::*;
use std::collections::HashMap;

fn sample_fk_schema() -> V2Schema {
    let mut m = HashMap::new();
    V2Schema {
        tables: vec![
            V2Table {
                name: "parent".to_string(),
                columns: vec![V2Column { name: "id".to_string(), data_type: V2DataType::Integer { bits: Some(64) }, constraints: vec![ColumnConstraint::NotNull], annotations: vec![] }],
                primary_key: Some(vec!["id".to_string()]),
                constraints: vec![],
            },
            V2Table {
                name: "child".to_string(),
                columns: vec![
                    V2Column { name: "a".to_string(), data_type: V2DataType::Integer { bits: Some(32) }, constraints: vec![], annotations: vec![] },
                    V2Column { name: "b".to_string(), data_type: V2DataType::Integer { bits: Some(32) }, constraints: vec![], annotations: vec![] },
                ],
                primary_key: Some(vec!["a".to_string(), "b".to_string()]),
                constraints: vec![Constraint::ForeignKey(ForeignKey { name: Some("fk_parent".to_string()), columns: vec!["a".to_string()], referenced_table: "parent".to_string(), referenced_columns: vec!["id".to_string()], on_update: Some("NO ACTION".to_string()), on_delete: Some("CASCADE".to_string()) })],
            },
        ],
        metadata: m,
    }
}

#[test]
fn emit_fk_actions_mysql() {
    let schema = sample_fk_schema();
    let sql = emit_mysql_v2(&schema);
    assert!(sql.contains("ON DELETE CASCADE"));
    assert!(sql.contains("ON UPDATE NO ACTION"));
}

#[test]
fn emit_composite_pk_sqlite() {
    let schema = sample_fk_schema();
    let sql = emit_sqlite_v2(&schema);
    assert!(sql.contains("PRIMARY KEY (a, b)"));
}
