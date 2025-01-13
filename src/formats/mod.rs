//! Bioinformatics file format support module
//! Handles common file formats used in bioinformatics

pub mod fasta;
pub mod fastq;
pub mod bed;
pub mod format_detector;

use crate::error::ExtractorError;
use crate::Result;
use std::path::Path;

/// Supported file formats
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileFormat {
    CSV,
    FASTA,
    FASTQ,
    BED,
    Unknown,
}

/// Trait for bioinformatics file records
pub trait BioRecord: Send + Sync {
    /// Get the record identifier
    fn id(&self) -> &str;
    
    /// Get the sequence if available
    fn sequence(&self) -> Option<&[u8]>;
    
    /// Get the quality scores if available
    fn quality(&self) -> Option<&[u8]>;
    
    /// Get additional metadata
    fn metadata(&self) -> &[(String, String)];
    
    /// Convert record to string
    fn to_string(&self) -> String;
}

/// File format detection and validation
pub trait FormatDetector {
    /// Detect file format from content
    fn detect_format(path: &Path) -> Result<FileFormat>;
    
    /// Validate file format
    fn validate(path: &Path) -> Result<bool>;
}

/// Common biological data filters
pub trait BioFilter {
    /// Filter by GC content
    fn gc_content(&self) -> f64;
    
    /// Filter by sequence length
    fn sequence_length(&self) -> usize;
    
    /// Filter by quality score
    fn min_quality_score(&self) -> Option<u8>;
    
    /// Check if sequence contains pattern
    fn contains_pattern(&self, pattern: &[u8]) -> bool;
}