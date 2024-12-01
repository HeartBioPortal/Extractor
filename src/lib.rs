//! Extractor: High-performance biological CSV file filtering library
//! 
//! This library provides efficient filtering and processing capabilities for large CSV files,
//! particularly focused on biological data. It supports both streaming and indexed access modes
//! for optimal performance with different use cases.

#![deny(missing_docs)]
#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(rustdoc::broken_intra_doc_links)]

use std::path::PathBuf;

pub mod core;
pub mod error;
pub mod filters;
pub mod index;
pub mod utils;

// Re-export commonly used items
pub use crate::core::BioFilter;
pub use crate::error::ExtractorError;
pub use crate::filters::{Filter, FilterCondition};
pub use crate::index::FileIndex;

/// Configuration options for the Extractor
#[derive(Debug, Clone)]
pub struct Config {
    /// CSV delimiter character (default: ',')
    pub delimiter: u8,
    /// Whether the CSV file has headers (default: true)
    pub has_headers: bool,
    /// Size of processing chunks in bytes (default: 1MB)
    pub chunk_size: usize,
    /// Enable parallel processing (default: true)
    pub parallel: bool,
    /// Use indexed mode for faster access (default: false)
    pub use_index: bool,
    /// Number of worker threads for parallel processing (default: num_cpus)
    pub num_threads: Option<usize>,
    /// Progress bar configuration
    pub progress: ProgressConfig,
}

/// Configuration for progress reporting
#[derive(Debug, Clone)]
pub struct ProgressConfig {
    /// Enable progress bar (default: true if feature enabled)
    pub enabled: bool,
    /// Refresh rate in milliseconds (default: 100)
    pub refresh_rate: u64,
    /// Show ETA (default: true)
    pub show_eta: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            delimiter: b',',
            has_headers: true,
            chunk_size: 1024 * 1024, // 1MB
            parallel: true,
            use_index: false,
            num_threads: None,
            progress: ProgressConfig::default(),
        }
    }
}

impl Default for ProgressConfig {
    fn default() -> Self {
        Self {
            enabled: cfg!(feature = "progress-bars"),
            refresh_rate: 100,
            show_eta: true,
        }
    }
}

/// Result type for Extractor operations
pub type Result<T> = std::result::Result<T, ExtractorError>;

/// Statistics about the processing operation
#[derive(Debug, Clone)]
pub struct ProcessingStats {
    /// Number of rows processed
    pub rows_processed: u64,
    /// Number of rows matched
    pub rows_matched: u64,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Input file size in bytes
    pub input_size: u64,
    /// Output file size in bytes
    pub output_size: u64,
}

/// A builder for configuring and creating a BioFilter instance
#[derive(Debug)]
pub struct ExtractorBuilder {
    config: Config,
    input_path: PathBuf,
    output_path: PathBuf,
    index_path: Option<PathBuf>,
}

impl ExtractorBuilder {
    /// Create a new builder instance
    pub fn new<P: Into<PathBuf>>(input_path: P, output_path: P) -> Self {
        Self {
            config: Config::default(),
            input_path: input_path.into(),
            output_path: output_path.into(),
            index_path: None,
        }
    }

    /// Set the configuration
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    /// Set the index file path
    pub fn with_index<P: Into<PathBuf>>(mut self, index_path: P) -> Self {
        self.index_path = Some(index_path.into());
        self.config.use_index = true;
        self
    }

    /// Build the BioFilter instance
    pub fn build(self) -> Result<BioFilter> {
        BioFilter::new(self.input_path, self.output_path, self.config, self.index_path)
    }
}

/// Convenience function to create a new ExtractorBuilder
pub fn builder<P: Into<PathBuf>>(input_path: P, output_path: P) -> ExtractorBuilder {
    ExtractorBuilder::new(input_path, output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_builder_pattern() {
        let filter = builder("input.csv", "output.csv")
            .with_config(Config::default())
            .with_index("index.json")
            .build();
        
        assert!(filter.is_ok());
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.delimiter, b',');
        assert!(config.has_headers);
        assert!(config.parallel);
        assert!(!config.use_index);
    }
}