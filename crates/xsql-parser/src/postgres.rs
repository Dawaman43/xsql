use sqlparser::ast::DataType as SqlType;
use sqlparser::ast::*;
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;

use xsql_ir::{Column, DataType, Schema, Table};

fn is_identity_column(options: &[ColumnOptionDef]) -> bool {
    options.iter().any(|opt| {
        matches!(
            opt.option,
            ColumnOption::Generated {
                generated_as: GeneratedAs::Always | GeneratedAs::ByDefault,
                generation_expr: None,
                ..
            }
        )
    })
}

pub fn parse_postgres_schema(sql: &str) -> Result<Schema, sqlparser::parser::ParserError> {
    let dialect = PostgreSqlDialect {};
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
                let mut nullable = true;
                let mut default = None;
                let mut col_is_primary_key = false;

                for opt in &col.options {
                    match &opt.option {
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

                let mut auto_increment = is_identity_column(&col.options);

                let data_type = match &col.data_type {
                    SqlType::Int(_) | SqlType::Integer(_) | SqlType::Int4(_) => DataType::Int,
                    SqlType::BigInt(_) | SqlType::Int8(_) => DataType::BigInt,
                    SqlType::SmallInt(_) | SqlType::Int2(_) => DataType::Int,
                    SqlType::Boolean | SqlType::Bool => DataType::Boolean,
                    SqlType::Real | SqlType::Float4 | SqlType::Float(_) => DataType::Float,
                    SqlType::Double | SqlType::DoublePrecision | SqlType::Float8 => {
                        DataType::Double
                    }
                    SqlType::Varchar(len)
                    | SqlType::CharacterVarying(len)
                    | SqlType::CharVarying(len) => {
                        let size = len.and_then(|l| match l {
                            CharacterLength::IntegerLength { length, .. } => Some(length as u32),
                            CharacterLength::Max => None,
                        });
                        DataType::Varchar(size)
                    }
                    SqlType::Text => DataType::Text,
                    SqlType::Timestamp(_, _) | SqlType::Datetime(_) => DataType::Timestamp,
                    SqlType::Custom(name, _) => {
                        let ty = name.to_string();
                        if ty.eq_ignore_ascii_case("serial") {
                            auto_increment = true;
                            DataType::Int
                        } else if ty.eq_ignore_ascii_case("bigserial") {
                            auto_increment = true;
                            DataType::BigInt
                        } else {
                            DataType::Text
                        }
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
