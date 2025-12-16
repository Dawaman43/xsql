use sqlparser::ast::DataType as SqlType;
use sqlparser::ast::*;
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;
use sqlparser::tokenizer::Token;

use xsql_ir::{Column, DataType, Schema, Table};

fn has_auto_increment(tokens: &[Token]) -> bool {
    tokens
        .iter()
        .any(|t| matches!(t, Token::Word(w) if w.value.eq_ignore_ascii_case("auto_increment")))
}

pub fn parse_mysql_schema(sql: &str) -> Result<Schema, sqlparser::parser::ParserError> {
    let dialect = MySqlDialect {};
    let ast = Parser::parse_sql(&dialect, sql)?;

    let mut schema = Schema { tables: vec![] };

    for stmt in ast {
        if let Statement::CreateTable {
            name,
            columns,
            constraints,
            ..
        } = stmt
        {
            let mut table = Table {
                name: name.to_string(),
                columns: vec![],
                primary_key: None,
            };

            for col in columns {
                let mut auto_increment = false;
                let mut nullable = true;
                let mut default = None;
                let mut col_is_primary_key = false;

                for opt in &col.options {
                    match &opt.option {
                        ColumnOption::DialectSpecific(tokens) => {
                            if has_auto_increment(tokens) {
                                auto_increment = true;
                            }
                        }
                        ColumnOption::NotNull => nullable = false,
                        ColumnOption::Null => nullable = true,
                        ColumnOption::Default(expr) => default = Some(expr.to_string()),
                        ColumnOption::Unique {
                            is_primary: true, ..
                        } => {
                            col_is_primary_key = true;
                            nullable = false;
                        }
                        _ => {}
                    }
                }

                let data_type = match &col.data_type {
                    SqlType::TinyInt(Some(1)) => DataType::Boolean,
                    SqlType::Int(_) => DataType::Int,
                    SqlType::BigInt(_) => DataType::BigInt,
                    SqlType::Integer(_) => DataType::Int,
                    SqlType::Varchar(len) => {
                        let size = len.and_then(|l| match l {
                            CharacterLength::IntegerLength { length, .. } => Some(length as u32),
                            CharacterLength::Max => None,
                        });
                        DataType::Varchar(size)
                    }
                    SqlType::Text => DataType::Text,
                    SqlType::Datetime(_) => DataType::Timestamp,
                    SqlType::Timestamp(_, _) => DataType::Timestamp,
                    SqlType::Boolean | SqlType::Bool => DataType::Boolean,
                    SqlType::Float(_) | SqlType::Real | SqlType::Float4 => DataType::Float,
                    SqlType::Double | SqlType::DoublePrecision | SqlType::Float8 => {
                        DataType::Double
                    }
                    _ => DataType::Text,
                };

                table.columns.push(Column {
                    name: col.name.value.clone(),
                    data_type,
                    nullable,
                    auto_increment,
                    default,
                });

                if col_is_primary_key && table.primary_key.is_none() {
                    table.primary_key = Some(vec![col.name.value.clone()]);
                }
            }

            for c in constraints {
                if let TableConstraint::Unique {
                    is_primary: true,
                    columns,
                    ..
                } = c
                {
                    table.primary_key = Some(columns.iter().map(|c| c.value.clone()).collect());
                }
            }

            schema.tables.push(table);
        }
    }

    Ok(schema)
}
