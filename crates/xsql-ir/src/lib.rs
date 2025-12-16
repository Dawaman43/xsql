#[derive(Debug, Clone)]
pub struct Schema {
    pub tables: Vec<Table>,
}

#[derive(Debug, Clone)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub primary_key: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub auto_increment: bool,
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataType {
    Int,
    BigInt,
    Boolean,
    Float,
    Double,
    Varchar(Option<u32>),
    Text,
    Timestamp,
}

// New IR v2 is provided in a separate module to preserve backwards compatibility
// with existing parsers and emitters while enabling a richer schema representation.
pub mod v2;
pub mod alter;
