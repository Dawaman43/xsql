//! IR v2: richer intermediate representation for schema portability
//!
//! This module introduces more detailed schema constructs without replacing
//! the existing `Schema`/`Table`/`Column` types. The goal is to provide a
//! non-breaking path for tooling that needs constraints, foreign keys, unique
//! constraints, and checks for linting, diffing, and ALTER generation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V2Schema {
    pub tables: Vec<V2Table>,
    /// Optional free-form metadata (key/value) for tools.
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V2Table {
    pub name: String,
    pub columns: Vec<V2Column>,
    pub primary_key: Option<Vec<String>>,
    /// Table-level constraints (unique groups, foreign keys, checks)
    pub constraints: Vec<Constraint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V2Column {
    pub name: String,
    pub data_type: V2DataType,
    /// Column-level constraints (NOT NULL, DEFAULT, AUTO INCREMENT)
    pub constraints: Vec<ColumnConstraint>,
    /// Annotations for things that cannot be represented losslessly
    pub annotations: Vec<String>,
}

/// A small set of portable data types used by V2. Emitters may map these to
/// dialect-specific types; parsers map dialect types into these where possible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum V2DataType {
    Integer {
        bits: Option<u8>,
    },
    Boolean,
    Float,
    Double,
    Varchar(Option<u32>),
    Text,
    Timestamp,
    /// Fallback for unrecognized or vendor-specific types.
    Custom(String),
}

/// Column-level constraints that are common across dialects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColumnConstraint {
    NotNull,
    Default(String),
    AutoIncrement,
    /// A CHECK expression stored as text. Emitters may emit or warn.
    Check(String),
}

/// Table-level constraints and foreign keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constraint {
    Unique {
        name: Option<String>,
        columns: Vec<String>,
    },
    ForeignKey(ForeignKey),
    Check {
        name: Option<String>,
        expr: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKey {
    pub name: Option<String>,
    pub columns: Vec<String>,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    /// Action on update/delete (e.g., CASCADE, SET NULL). Keep as text for portability.
    pub on_update: Option<String>,
    pub on_delete: Option<String>,
}

impl V2Schema {
    pub fn new() -> Self {
        V2Schema {
            tables: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

impl Default for V2Schema {
    fn default() -> Self {
        Self::new()
    }
}
