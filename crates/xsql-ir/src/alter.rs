use crate::v2::V2Schema;

/// Very small ALTER-generation scaffold.
///
/// This is a placeholder for future ALTER statement generation from v2 diffs.
/// For now it emits simple CREATE TABLE / DROP TABLE statements for added/removed tables.
pub fn diff_to_alter_sql(old: &V2Schema, new: &V2Schema) -> Vec<String> {
    let mut out = Vec::new();

    let old_names: std::collections::BTreeSet<_> = old.tables.iter().map(|t| t.name.clone()).collect();
    let new_names: std::collections::BTreeSet<_> = new.tables.iter().map(|t| t.name.clone()).collect();

    for added in new_names.difference(&old_names) {
        // Placeholder: in future we will render full CREATE TABLE; for now emit simple marker.
        out.push(format!("-- TODO: CREATE TABLE {} (full definition)", added));
    }

    for removed in old_names.difference(&new_names) {
        out.push(format!("DROP TABLE {};", removed));
    }

    out
}
