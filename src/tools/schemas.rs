//! Schema inference utilities
use std::collections::HashMap;
use crate::Result;

/// Schema inference tool
#[derive(Debug)]
pub struct SchemaInference {
    sample_size: usize,
    confidence_threshold: f64,
}

#[derive(Debug)]
pub struct InferredSchema {
    pub columns: HashMap<String, ColumnType>,
    pub constraints: Vec<SchemaConstraint>,
    pub confidence: f64,
}

#[derive(Debug)]
pub enum ColumnType {
    Integer { signed: bool, bits: u8 },
    Float { bits: u8 },
    String { max_length: Option<usize> },
    Date { format: String },
    Boolean,
}

#[derive(Debug)]
pub enum SchemaConstraint {
    PrimaryKey(String),
    ForeignKey { column: String, references: String },
    NotNull(String),
    Unique(String),
    Check { column: String, condition: String },
}