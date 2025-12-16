use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::v2::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct V2Diff {
    pub added_tables: Vec<V2Table>,
    pub removed_tables: Vec<String>,
    pub changed_tables: Vec<TableChange>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableChange {
    pub name: String,
    pub added_columns: Vec<V2Column>,
    pub removed_columns: Vec<String>,
    pub changed_columns: Vec<ColumnChange>,
    pub pk_changed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ColumnChange {
    pub name: String,
    pub before: V2Column,
    pub after: V2Column,
}

pub fn diff_v2_schemas(old: &V2Schema, new: &V2Schema) -> V2Diff {
    let old_map: BTreeMap<_, _> = old
        .tables
        .iter()
        .map(|t| (t.name.clone(), t.clone()))
        .collect();
    let new_map: BTreeMap<_, _> = new
        .tables
        .iter()
        .map(|t| (t.name.clone(), t.clone()))
        .collect();

    let old_tables: BTreeSet<_> = old_map.keys().cloned().collect();
    let new_tables: BTreeSet<_> = new_map.keys().cloned().collect();

    let added_tables: Vec<_> = new_tables
        .difference(&old_tables)
        .cloned()
        .map(|n| new_map.get(&n).unwrap().clone())
        .collect();
    let removed_tables: Vec<_> = old_tables.difference(&new_tables).cloned().collect();

    let mut changed_tables = Vec::new();

    for tname in old_tables.intersection(&new_tables) {
        let a = old_map.get(tname).expect("table exists");
        let b = new_map.get(tname).expect("table exists");

        let a_cols: BTreeMap<_, _> = a
            .columns
            .iter()
            .map(|c| (c.name.clone(), c.clone()))
            .collect();
        let b_cols: BTreeMap<_, _> = b
            .columns
            .iter()
            .map(|c| (c.name.clone(), c.clone()))
            .collect();

        let a_names: BTreeSet<_> = a_cols.keys().cloned().collect();
        let b_names: BTreeSet<_> = b_cols.keys().cloned().collect();

        let added_cols: Vec<_> = b_names
            .difference(&a_names)
            .cloned()
            .map(|n| b_cols.get(&n).unwrap().clone())
            .collect();
        let removed_cols: Vec<_> = a_names.difference(&b_names).cloned().collect();

        let mut changed_cols = Vec::new();
        for cname in a_names.intersection(&b_names) {
            let ca = a_cols.get(cname).expect("col exists");
            let cb = b_cols.get(cname).expect("col exists");
            if ca != cb {
                changed_cols.push(ColumnChange {
                    name: cname.clone(),
                    before: ca.clone(),
                    after: cb.clone(),
                });
            }
        }

        let pk_changed = a.primary_key != b.primary_key;

        if !added_cols.is_empty()
            || !removed_cols.is_empty()
            || !changed_cols.is_empty()
            || pk_changed
        {
            changed_tables.push(TableChange {
                name: tname.clone(),
                added_columns: added_cols,
                removed_columns: removed_cols,
                changed_columns: changed_cols,
                pk_changed,
            });
        }
    }

    V2Diff {
        added_tables,
        removed_tables,
        changed_tables,
    }
}
