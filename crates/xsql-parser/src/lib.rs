pub mod mysql;
pub mod postgres;
pub mod sqlite;

/// IR v2: opt-in parse entrypoints (WIP, not all constraints mapped yet)
pub mod v2 {
    use xsql_ir::v2::*;

    use sqlparser::ast::*;
    use sqlparser::dialect::PostgreSqlDialect;
    use sqlparser::parser::Parser;

    pub fn parse_postgres_schema_v2(sql: &str) -> Result<V2Schema, String> {
        let dialect = PostgreSqlDialect {};
        let ast = Parser::parse_sql(&dialect, sql).map_err(|e| e.to_string())?;
        let mut schema = V2Schema::new();

        for stmt in ast {
            if let Statement::CreateTable {
                name,
                columns,
                constraints,
                ..
            } = stmt
            {
                let mut table = V2Table {
                    name: name.to_string(),
                    columns: vec![],
                    primary_key: None,
                    constraints: vec![],
                };

                for col in columns {
                    let mut col_constraints = vec![];
                    let mut annotations = vec![];
                    for opt in &col.options {
                        match &opt.option {
                            ColumnOption::NotNull => {
                                col_constraints.push(ColumnConstraint::NotNull)
                            }
                            ColumnOption::Null => {}
                            ColumnOption::Default(expr) => {
                                col_constraints.push(ColumnConstraint::Default(expr.to_string()))
                            }
                            ColumnOption::Unique {
                                is_primary: true, ..
                            } => {
                                // PK handled below
                            }
                            ColumnOption::Generated { .. } => {
                                col_constraints.push(ColumnConstraint::AutoIncrement)
                            }
                            ColumnOption::Check(expr) => {
                                col_constraints.push(ColumnConstraint::Check(expr.to_string()))
                            }
                            _ => annotations.push(format!("unhandled column option: {opt:?}")),
                        }
                    }

                    let dt = col.data_type.to_string().to_lowercase();
                    let data_type = if dt.contains("tinyint(1)") {
                        V2DataType::Boolean
                    } else if dt.contains("bigint") {
                        V2DataType::Integer { bits: Some(64) }
                    } else if dt.contains("smallint") {
                        V2DataType::Integer { bits: Some(16) }
                    } else if dt.contains("int") {
                        V2DataType::Integer { bits: Some(32) }
                    } else if dt.contains("double")
                        || dt.contains("float8")
                        || dt.contains("double precision")
                    {
                        V2DataType::Double
                    } else if dt.contains("float") || dt.contains("real") {
                        V2DataType::Float
                    } else if dt.contains("varchar")
                        || dt.contains("character varying")
                        || dt.contains("char")
                    {
                        // try to extract size
                        let size = dt
                            .split(|c: char| !c.is_ascii_digit())
                            .filter_map(|s| s.parse::<u32>().ok())
                            .next();
                        V2DataType::Varchar(size)
                    } else if dt.contains("text") {
                        V2DataType::Text
                    } else if dt.contains("timestamp") || dt.contains("datetime") {
                        V2DataType::Timestamp
                    } else {
                        V2DataType::Custom(col.data_type.to_string())
                    };

                    table.columns.push(V2Column {
                        name: col.name.value.clone(),
                        data_type,
                        constraints: col_constraints,
                        annotations,
                    });
                }

                for c in constraints {
                    match c {
                        TableConstraint::Unique {
                            name,
                            is_primary,
                            columns,
                            ..
                        } => {
                            if is_primary {
                                table.primary_key =
                                    Some(columns.iter().map(|c| c.value.clone()).collect());
                            } else {
                                table.constraints.push(Constraint::Unique {
                                    name: name.map(|n| n.value),
                                    columns: columns.iter().map(|c| c.value.clone()).collect(),
                                });
                            }
                        }
                        TableConstraint::ForeignKey {
                            name,
                            columns,
                            foreign_table,
                            referred_columns,
                            ..
                        } => {
                            table.constraints.push(Constraint::ForeignKey(ForeignKey {
                                name: name.map(|n| n.value),
                                columns: columns.iter().map(|c| c.value.clone()).collect(),
                                referenced_table: foreign_table.to_string(),
                                referenced_columns: referred_columns
                                    .iter()
                                    .map(|c| c.value.clone())
                                    .collect(),
                                on_update: None, // TODO: parse actions
                                on_delete: None,
                            }));
                        }
                        TableConstraint::Check { name, expr, .. } => {
                            table.constraints.push(Constraint::Check {
                                name: name.map(|n| n.value),
                                expr: expr.to_string(),
                            });
                        }
                        _ => {}
                    }
                }

                schema.tables.push(table);
            }
        }
        Ok(schema)
    }

    pub fn parse_mysql_schema_v2(sql: &str) -> Result<V2Schema, String> {
        use sqlparser::ast::*;
        use sqlparser::dialect::MySqlDialect;
        use sqlparser::parser::Parser;
        use sqlparser::tokenizer::Token;
        let dialect = MySqlDialect {};
        let ast = Parser::parse_sql(&dialect, sql).map_err(|e| e.to_string())?;
        let mut schema = V2Schema::new();
        fn has_auto_increment(tokens: &[Token]) -> bool {
            tokens.iter().any(
                |t| matches!(t, Token::Word(w) if w.value.eq_ignore_ascii_case("auto_increment")),
            )
        }
        for stmt in ast {
            if let Statement::CreateTable {
                name,
                columns,
                constraints,
                ..
            } = stmt
            {
                let mut table = V2Table {
                    name: name.to_string(),
                    columns: vec![],
                    primary_key: None,
                    constraints: vec![],
                };
                for col in columns {
                    let mut col_constraints = vec![];
                    let mut annotations = vec![];
                    for opt in &col.options {
                        match &opt.option {
                            ColumnOption::NotNull => {
                                col_constraints.push(ColumnConstraint::NotNull)
                            }
                            ColumnOption::Null => {}
                            ColumnOption::Default(expr) => {
                                col_constraints.push(ColumnConstraint::Default(expr.to_string()))
                            }
                            ColumnOption::Unique {
                                is_primary: true, ..
                            } => {}
                            ColumnOption::DialectSpecific(tokens) => {
                                if has_auto_increment(tokens) {
                                    col_constraints.push(ColumnConstraint::AutoIncrement);
                                }
                            }
                            ColumnOption::Check(expr) => {
                                col_constraints.push(ColumnConstraint::Check(expr.to_string()))
                            }
                            _ => annotations.push(format!("unhandled column option: {opt:?}")),
                        }
                    }
                    let dt = col.data_type.to_string().to_lowercase();
                    let data_type = if dt.contains("tinyint(1)") {
                        V2DataType::Boolean
                    } else if dt.contains("bigint") {
                        V2DataType::Integer { bits: Some(64) }
                    } else if dt.contains("int") {
                        V2DataType::Integer { bits: Some(32) }
                    } else if dt.contains("varchar") {
                        let size = dt
                            .split(|c: char| !c.is_ascii_digit())
                            .filter_map(|s| s.parse::<u32>().ok())
                            .next();
                        V2DataType::Varchar(size)
                    } else if dt.contains("text") {
                        V2DataType::Text
                    } else if dt.contains("timestamp") || dt.contains("datetime") {
                        V2DataType::Timestamp
                    } else if dt.contains("double") {
                        V2DataType::Double
                    } else if dt.contains("float") || dt.contains("real") {
                        V2DataType::Float
                    } else if dt.contains("bool") {
                        V2DataType::Boolean
                    } else {
                        V2DataType::Custom(col.data_type.to_string())
                    };
                    table.columns.push(V2Column {
                        name: col.name.value.clone(),
                        data_type,
                        constraints: col_constraints,
                        annotations,
                    });
                }
                for c in constraints {
                    match c {
                        TableConstraint::Unique {
                            name,
                            is_primary,
                            columns,
                            ..
                        } => {
                            if is_primary {
                                table.primary_key =
                                    Some(columns.iter().map(|c| c.value.clone()).collect());
                            } else {
                                table.constraints.push(Constraint::Unique {
                                    name: name.map(|n| n.value),
                                    columns: columns.iter().map(|c| c.value.clone()).collect(),
                                });
                            }
                        }
                        TableConstraint::ForeignKey {
                            name,
                            columns,
                            foreign_table,
                            referred_columns,
                            ..
                        } => {
                            table.constraints.push(Constraint::ForeignKey(ForeignKey {
                                name: name.map(|n| n.value),
                                columns: columns.iter().map(|c| c.value.clone()).collect(),
                                referenced_table: foreign_table.to_string(),
                                referenced_columns: referred_columns
                                    .iter()
                                    .map(|c| c.value.clone())
                                    .collect(),
                                on_update: None,
                                on_delete: None,
                            }));
                        }
                        TableConstraint::Check { name, expr, .. } => {
                            table.constraints.push(Constraint::Check {
                                name: name.map(|n| n.value),
                                expr: expr.to_string(),
                            });
                        }
                        _ => {}
                    }
                }
                schema.tables.push(table);
            }
        }
        Ok(schema)
    }

    // Postgres v2 parser implemented above; do not redefine.

    pub fn parse_sqlite_schema_v2(sql: &str) -> Result<V2Schema, String> {
        use sqlparser::ast::*;
        use sqlparser::dialect::SQLiteDialect;
        use sqlparser::parser::Parser;
        use sqlparser::tokenizer::Token;
        let dialect = SQLiteDialect {};
        let ast = Parser::parse_sql(&dialect, sql).map_err(|e| e.to_string())?;
        let mut schema = V2Schema::new();
        fn has_autoincrement(tokens: &[Token]) -> bool {
            tokens.iter().any(
                |t| matches!(t, Token::Word(w) if w.value.eq_ignore_ascii_case("autoincrement")),
            )
        }
        for stmt in ast {
            if let Statement::CreateTable {
                name,
                columns,
                constraints,
                ..
            } = stmt
            {
                let mut table = V2Table {
                    name: name.to_string(),
                    columns: vec![],
                    primary_key: None,
                    constraints: vec![],
                };
                for col in columns {
                    let mut col_constraints = vec![];
                    let mut annotations = vec![];
                    for opt in &col.options {
                        match &opt.option {
                            ColumnOption::NotNull => {
                                col_constraints.push(ColumnConstraint::NotNull)
                            }
                            ColumnOption::Null => {}
                            ColumnOption::Default(expr) => {
                                col_constraints.push(ColumnConstraint::Default(expr.to_string()))
                            }
                            ColumnOption::Unique {
                                is_primary: true, ..
                            } => {}
                            ColumnOption::DialectSpecific(tokens) => {
                                if has_autoincrement(tokens) {
                                    col_constraints.push(ColumnConstraint::AutoIncrement);
                                }
                            }
                            ColumnOption::Check(expr) => {
                                col_constraints.push(ColumnConstraint::Check(expr.to_string()))
                            }
                            _ => annotations.push(format!("unhandled column option: {opt:?}")),
                        }
                    }
                    let dt = col.data_type.to_string().to_lowercase();
                    let data_type = if dt.contains("bigint") {
                        V2DataType::Integer { bits: Some(64) }
                    } else if dt.contains("int") {
                        V2DataType::Integer { bits: Some(32) }
                    } else if dt.contains("bool") || dt.contains("tinyint(1)") {
                        V2DataType::Boolean
                    } else if dt.contains("double") {
                        V2DataType::Double
                    } else if dt.contains("float") || dt.contains("real") {
                        V2DataType::Float
                    } else if dt.contains("varchar") || dt.contains("character varying") {
                        let size = dt
                            .split(|c: char| !c.is_ascii_digit())
                            .filter_map(|s| s.parse::<u32>().ok())
                            .next();
                        V2DataType::Varchar(size)
                    } else if dt.contains("text") || dt.contains("string") {
                        V2DataType::Text
                    } else if dt.contains("timestamp") || dt.contains("datetime") {
                        V2DataType::Timestamp
                    } else {
                        V2DataType::Custom(col.data_type.to_string())
                    };
                    table.columns.push(V2Column {
                        name: col.name.value.clone(),
                        data_type,
                        constraints: col_constraints,
                        annotations,
                    });
                }
                for c in constraints {
                    match c {
                        TableConstraint::Unique {
                            name,
                            is_primary,
                            columns,
                            ..
                        } => {
                            if is_primary {
                                table.primary_key =
                                    Some(columns.iter().map(|c| c.value.clone()).collect());
                            } else {
                                table.constraints.push(Constraint::Unique {
                                    name: name.map(|n| n.value),
                                    columns: columns.iter().map(|c| c.value.clone()).collect(),
                                });
                            }
                        }
                        TableConstraint::ForeignKey {
                            name,
                            columns,
                            foreign_table,
                            referred_columns,
                            ..
                        } => {
                            table.constraints.push(Constraint::ForeignKey(ForeignKey {
                                name: name.map(|n| n.value),
                                columns: columns.iter().map(|c| c.value.clone()).collect(),
                                referenced_table: foreign_table.to_string(),
                                referenced_columns: referred_columns
                                    .iter()
                                    .map(|c| c.value.clone())
                                    .collect(),
                                on_update: None,
                                on_delete: None,
                            }));
                        }
                        TableConstraint::Check { name, expr, .. } => {
                            table.constraints.push(Constraint::Check {
                                name: name.map(|n| n.value),
                                expr: expr.to_string(),
                            });
                        }
                        _ => {}
                    }
                }
                schema.tables.push(table);
            }
        }
        Ok(schema)
    }
}
