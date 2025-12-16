use xsql_ir::*;

pub fn emit_sqlite(schema: &Schema) -> String {
    let mut out = String::new();

    for table in &schema.tables {
        out.push_str(&format!("CREATE TABLE {} (\n", table.name));

        for (i, col) in table.columns.iter().enumerate() {
            let ty = match col.data_type {
                DataType::Boolean => "INTEGER",
                DataType::Int | DataType::BigInt => "INTEGER",
                _ => "TEXT",
            };

            out.push_str(&format!("  {} {}", col.name, ty));

            if col.auto_increment {
                out.push_str(" PRIMARY KEY AUTOINCREMENT");
            }

            if !col.nullable {
                out.push_str(" NOT NULL");
            }

            if i < table.columns.len() - 1 {
                out.push(',');
            }
            out.push('\n');
        }

        out.push_str(");\n\n");
    }

    out
}
