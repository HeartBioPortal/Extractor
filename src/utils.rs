//! Utility functions and helpers for the Extractor library.
//! This module provides common functionality used across the library.

use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use memmap2::{Mmap, MmapOptions};
use crate::error::ExtractorError;
use crate::Result;

/// Memory mapping options with safety checks
#[derive(Debug, Clone)]
pub struct SafeMmapOptions {
    /// Maximum file size to memory map (in bytes)
    pub max_size: Option<u64>,
    /// Whether to map the file read-only
    pub read_only: bool,
}

impl Default for SafeMmapOptions {
    fn default() -> Self {
        Self {
            max_size: Some(1024 * 1024 * 1024), // 1GB default limit
            read_only: true,
        }
    }
}

/// Safely create a memory map for a file
pub fn create_mmap(file: &File, options: &SafeMmapOptions) -> Result<Mmap> {
    let file_size = file.metadata()
        .map_err(|e| ExtractorError::io_error(e, "Failed to get file metadata"))?
        .len();

    // Check file size against maximum if specified
    if let Some(max_size) = options.max_size {
        if file_size > max_size {
            return Err(ExtractorError::ResourceExhaustion(
                format!("File size {} exceeds maximum allowed size {}", file_size, max_size)
            ));
        }
    }

    // Create the memory map
    unsafe {
        let mut mmap_options = MmapOptions::new();
        if options.read_only {
            mmap_options.map(file)
        } else {
            mmap_options.map_mut(file)
        }
        .map_err(|e| ExtractorError::Mmap(e.to_string()))
    }
}

/// Find the start of a line given a position in a byte slice
pub fn find_line_start(data: &[u8], mut pos: usize) -> usize {
    while pos > 0 && data[pos - 1] != b'\n' {
        pos -= 1;
    }
    pos
}

/// Find the end of a line given a position in a byte slice
pub fn find_line_end(data: &[u8], mut pos: usize) -> usize {
    while pos < data.len() && data[pos] != b'\n' {
        pos += 1;
    }
    pos
}

/// Check if a file is likely to be CSV based on content
pub fn is_csv_file(path: &Path) -> Result<bool> {
    let file = File::open(path)
        .map_err(|e| ExtractorError::io_error(e, path))?;
    
    let mut reader = BufReader::new(file);
    let mut buffer = [0u8; 1024];
    let bytes_read = reader.read(&mut buffer)
        .map_err(|e| ExtractorError::io_error(e, path))?;

    if bytes_read == 0 {
        return Ok(false);
    }

    // Check for common CSV characteristics
    let has_commas = buffer[..bytes_read].contains(&b',');
    let has_newlines = buffer[..bytes_read].contains(&b'\n');
    let consistent_fields = check_consistent_fields(&buffer[..bytes_read]);

    Ok(has_commas && has_newlines && consistent_fields)
}

/// Calculate file checksums for index validation
pub fn calculate_file_checksum(path: &Path) -> Result<u64> {
    let mut file = File::open(path)
        .map_err(|e| ExtractorError::io_error(e, path))?;
    
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let mut buffer = [0u8; 8192];
    
    loop {
        let bytes_read = file.read(&mut buffer)
            .map_err(|e| ExtractorError::io_error(e, path))?;
        
        if bytes_read == 0 {
            break;
        }
        
        use std::hash::Hasher;
        hasher.write(&buffer[..bytes_read]);
    }
    
    use std::hash::Hasher;
    Ok(hasher.finish())
}

/// Get the field count for a CSV row
fn get_field_count(line: &[u8]) -> usize {
    let mut count = 1;
    let mut in_quotes = false;
    
    for &byte in line {
        match byte {
            b'"' => in_quotes = !in_quotes,
            b',' if !in_quotes => count += 1,
            _ => {}
        }
    }
    
    count
}

/// Check if CSV has consistent number of fields per row
fn check_consistent_fields(data: &[u8]) -> bool {
    let mut lines = data.split(|&b| b == b'\n');
    
    if let Some(first_line) = lines.next() {
        let expected_count = get_field_count(first_line);
        lines.all(|line| get_field_count(line) == expected_count)
    } else {
        false
    }
}

/// Progress tracking helper
#[cfg(feature = "progress-bars")]
pub struct Progress {
    bar: indicatif::ProgressBar,
    total: u64,
}

#[cfg(feature = "progress-bars")]
impl Progress {
    pub fn new(total: u64, message: &str) -> Self {
        let bar = indicatif::ProgressBar::new(total);
        bar.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("{msg} [{bar:40}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("=> ")
        );
        bar.set_message(message.to_string());
        
        Self { bar, total }
    }

    pub fn inc(&self, delta: u64) {
        self.bar.inc(delta);
    }

    pub fn finish(&self) {
        self.bar.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_find_line_boundaries() {
        let data = b"first line\nsecond line\nthird line";
        assert_eq!(find_line_start(data, 15), 11);
        assert_eq!(find_line_end(data, 15), 21);
    }

    #[test]
    fn test_csv_detection() -> Result<()> {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "header1,header2,header3").unwrap();
        writeln!(file, "value1,value2,value3").unwrap();
        
        assert!(is_csv_file(file.path())?);
        Ok(())
    }

    #[test]
    fn test_field_count() {
        let line = b"field1,field2,\"field,3\",field4";
        assert_eq!(get_field_count(line), 4);
    }
}