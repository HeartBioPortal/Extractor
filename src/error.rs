//! Error types for the Extractor library.
//! This module defines all possible errors that can occur during CSV processing.

use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Main error type for the Extractor library
#[derive(Error, Debug)]
pub enum ExtractorError {
    /// I/O errors during file operations
    #[error("I/O error: {source}")]
    Io {
        #[from]
        source: io::Error,
        /// The path where the error occurred, if available
        path: Option<PathBuf>,
    },

    /// CSV parsing or writing errors
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    /// JSON serialization/deserialization errors (for index files)
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Index-related errors
    #[error("Index error: {kind}")]
    Index {
        kind: IndexErrorKind,
        path: Option<PathBuf>,
    },

    /// Filter-related errors
    #[error("Filter error: {kind}")]
    Filter {
        kind: FilterErrorKind,
        column: Option<String>,
    },

    /// Memory mapping errors
    #[error("Memory mapping error: {0}")]
    Mmap(String),

    /// Threading and parallel processing errors
    #[error("Parallel processing error: {0}")]
    Parallel(String),

    /// Column not found in CSV
    #[error("Column '{0}' not found in CSV headers")]
    ColumnNotFound(String),

    /// Invalid data format
    #[error("Invalid data format in column '{column}': {message}")]
    InvalidDataFormat {
        column: String,
        message: String,
        row: Option<u64>,
    },

    /// Resource exhaustion (memory, file handles, etc.)
    #[error("Resource exhaustion: {0}")]
    ResourceExhaustion(String),

    /// Generic error for unexpected situations
    #[error("{0}")]
    Other(String),
}

/// Specific kinds of index-related errors
#[derive(Debug, Error)]
pub enum IndexErrorKind {
    /// Index file not found
    #[error("Index file not found")]
    NotFound,

    /// Invalid index format
    #[error("Invalid index format")]
    InvalidFormat,

    /// Index is out of date
    #[error("Index is outdated")]
    Outdated,

    /// Error building index
    #[error("Failed to build index: {0}")]
    BuildError(String),
}

/// Specific kinds of filter-related errors
#[derive(Debug, Error)]
pub enum FilterErrorKind {
    /// Invalid filter condition
    #[error("Invalid filter condition")]
    InvalidCondition,

    /// Incompatible data type
    #[error("Incompatible data type for filter")]
    IncompatibleType,

    /// Invalid regex pattern
    #[error("Invalid regex pattern")]
    InvalidRegex,
}

impl ExtractorError {
    /// Create a new I/O error with an associated path
    pub fn io_error<P: Into<PathBuf>>(error: io::Error, path: P) -> Self {
        ExtractorError::Io {
            source: error,
            path: Some(path.into()),
        }
    }

    /// Create a new index error
    pub fn index_error<P: Into<PathBuf>>(kind: IndexErrorKind, path: Option<P>) -> Self {
        ExtractorError::Index {
            kind,
            path: path.map(|p| p.into()),
        }
    }

    /// Create a new filter error
    pub fn filter_error(kind: FilterErrorKind, column: Option<String>) -> Self {
        ExtractorError::Filter { kind, column }
    }

    /// Create a new configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        ExtractorError::Config(message.into())
    }

    /// Check if the error is related to I/O
    pub fn is_io_error(&self) -> bool {
        matches!(self, ExtractorError::Io { .. })
    }

    /// Check if the error is related to invalid data
    pub fn is_data_error(&self) -> bool {
        matches!(self, ExtractorError::InvalidDataFormat { .. })
    }

    /// Get the error category for logging/metrics
    pub fn category(&self) -> &'static str {
        match self {
            ExtractorError::Io { .. } => "io",
            ExtractorError::Csv(_) => "csv",
            ExtractorError::Json(_) => "json",
            ExtractorError::Config(_) => "config",
            ExtractorError::Index { .. } => "index",
            ExtractorError::Filter { .. } => "filter",
            ExtractorError::Mmap(_) => "mmap",
            ExtractorError::Parallel(_) => "parallel",
            ExtractorError::ColumnNotFound(_) => "column",
            ExtractorError::InvalidDataFormat { .. } => "data",
            ExtractorError::ResourceExhaustion(_) => "resource",
            ExtractorError::Other(_) => "other",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn test_io_error_creation() {
        let io_err = io::Error::new(ErrorKind::NotFound, "file not found");
        let err = ExtractorError::io_error(io_err, "test.csv");
        assert!(matches!(err, ExtractorError::Io { .. }));
    }

    #[test]
    fn test_error_categories() {
        let io_err = ExtractorError::io_error(
            io::Error::new(ErrorKind::Other, "test"),
            "test.csv",
        );
        assert_eq!(io_err.category(), "io");

        let config_err = ExtractorError::config("invalid config");
        assert_eq!(config_err.category(), "config");
    }

    #[test]
    fn test_filter_error_creation() {
        let err = ExtractorError::filter_error(
            FilterErrorKind::InvalidCondition,
            Some("gene_name".to_string()),
        );
        assert!(matches!(err, ExtractorError::Filter { .. }));
    }
}