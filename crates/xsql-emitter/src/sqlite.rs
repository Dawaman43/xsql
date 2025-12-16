use xsql_ir::*;

pub fn emit_sqlite(schema: &Schema) -> String {
    let mut out = String::new();

    for table in &schema.tables {
        out.push_str(&format!("CREATE TABLE {} (\n", table.name));

        let mut defs: Vec<String> = Vec::new();
        let has_autoinc_pk = table.columns.iter().any(|c| c.auto_increment);

        for col in &table.columns {
            let ty = match col.data_type {
                DataType::Boolean => "INTEGER",
                DataType::Int | DataType::BigInt => "INTEGER",
                DataType::Float | DataType::Double => "REAL",
                _ => "TEXT",
            };

            let mut line = format!("  {} {}", col.name, ty);

            if col.auto_increment {
                line.push_str(" PRIMARY KEY AUTOINCREMENT");
            }

            if !col.nullable {
                line.push_str(" NOT NULL");
            }

            if let Some(default) = &col.default {
                line.push_str(&format!(" DEFAULT {}", default));
            }

            defs.push(line);
        }

        if !has_autoinc_pk
            && let Some(pk) = &table.primary_key
            && !pk.is_empty()
        {
            defs.push(format!("  PRIMARY KEY ({})", pk.join(", ")));
        }

        out.push_str(&defs.join(",\n"));
        out.push_str("\n);\n\n");
    }

    out
}
