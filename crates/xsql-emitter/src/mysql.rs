use xsql_ir::*;

pub fn emit_mysql(schema: &Schema) -> String {
    let mut out = String::new();

    for table in &schema.tables {
        out.push_str(&format!("CREATE TABLE {} (\n", table.name));

        for (i, col) in table.columns.iter().enumerate() {
            let ty = match col.data_type {
                DataType::Int => "INT".to_string(),
                DataType::BigInt => "BIGINT".to_string(),
                DataType::Boolean => "TINYINT(1)".to_string(),
                DataType::Float => "FLOAT".to_string(),
                DataType::Double => "DOUBLE".to_string(),
                DataType::Varchar(Some(n)) => format!("VARCHAR({})", n),
                DataType::Varchar(None) => "VARCHAR(255)".to_string(),
                DataType::Text => "TEXT".to_string(),
                DataType::Timestamp => "DATETIME".to_string(),
            };

            out.push_str(&format!("  {} {}", col.name, ty));

            if col.auto_increment {
                out.push_str(" AUTO_INCREMENT");
            }

            if !col.nullable {
                out.push_str(" NOT NULL");
            }

            if let Some(default) = &col.default {
                out.push_str(&format!(" DEFAULT {}", default));
            }

            if i < table.columns.len() - 1 || table.primary_key.is_some() {
                out.push(',');
            }
            out.push('\n');
        }

        if let Some(pk) = &table.primary_key {
            out.push_str(&format!("  PRIMARY KEY ({})\n", pk.join(", ")));
        }

        out.push_str(");\n\n");
    }

    out
}
