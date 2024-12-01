//! Indexing functionality for fast CSV data access.
//! Provides file indexing and efficient row lookup capabilities.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::error::{ExtractorError, IndexErrorKind};
use crate::Result;

/// Represents a position in the CSV file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Byte offset from the start of the file
    pub offset: u64,
    /// Length of the row in bytes
    pub length: u32,
    /// Row number (0-based, excluding header)
    pub row_number: u64,
}

/// Index metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// Original file path
    pub source_file: PathBuf,
    /// File size at index creation
    pub file_size: u64,
    /// File modification time at index creation
    pub modified_time: u64,
    /// Checksum of the first few KB of the file
    pub file_checksum: u64,
    /// Number of indexed rows
    pub row_count: u64,
    /// Header row position
    pub header_position: Position,
    /// Index creation timestamp
    pub created_at: u64,
}

/// Main index structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIndex {
    /// Index metadata
    pub metadata: IndexMetadata,
    /// Column to index mapping
    pub columns: Vec<String>,
    /// Primary index column
    pub primary_column: String,
    /// Row positions by primary key
    pub positions: HashMap<String, Position>,
    /// Secondary indices
    pub secondary_indices: HashMap<String, HashMap<String, Vec<Position>>>,
}

impl FileIndex {
    /// Create a new index builder
    pub fn builder(source_file: PathBuf, primary_column: String) -> IndexBuilder {
        IndexBuilder::new(source_file, primary_column)
    }

    /// Load an existing index from file
    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path).map_err(|e| ExtractorError::io_error(e, path))?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(|e| ExtractorError::index_error(
            IndexErrorKind::InvalidFormat,
            Some(path.to_owned())
        ))
    }

    /// Save index to file
    pub fn save(&self, path: &Path) -> Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .map_err(|e| ExtractorError::io_error(e, path))?;
            
        serde_json::to_writer(file, self).map_err(|e| ExtractorError::index_error(
            IndexErrorKind::BuildError(e.to_string()),
            Some(path.to_owned())
        ))
    }

    /// Verify index against current file state
    pub fn verify(&self, file: &File) -> Result<bool> {
        let metadata = file.metadata()
            .map_err(|e| ExtractorError::io_error(e, &self.metadata.source_file))?;

        // Check file size
        if metadata.len() != self.metadata.file_size {
            return Ok(false);
        }

        // Check modification time
        #[cfg(unix)]
        let mtime = metadata.modified()
            .map_err(|e| ExtractorError::io_error(e, &self.metadata.source_file))?
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        #[cfg(unix)]
        if mtime > self.metadata.modified_time {
            return Ok(false);
        }

        // Verify checksum
        let current_checksum = self.calculate_checksum(file)?;
        Ok(current_checksum == self.metadata.file_checksum)
    }

    /// Get position for a primary key value
    pub fn get_position(&self, key: &str) -> Option<&Position> {
        self.positions.get(key)
    }

    /// Get positions for a secondary index value
    pub fn get_secondary_positions(&self, column: &str, value: &str) -> Option<&Vec<Position>> {
        self.secondary_indices.get(column)?.get(value)
    }

    /// Calculate file checksum
    fn calculate_checksum(&self, file: &File) -> Result<u64> {
        let mut buffer = [0u8; 8192];
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let mut handle = file.try_clone()
            .map_err(|e| ExtractorError::io_error(e, &self.metadata.source_file))?;
            
        handle.seek(SeekFrom::Start(0))
            .map_err(|e| ExtractorError::io_error(e, &self.metadata.source_file))?;
            
        let bytes_read = handle.read(&mut buffer)
            .map_err(|e| ExtractorError::io_error(e, &self.metadata.source_file))?;
            
        use std::hash::Hasher;
        hasher.write(&buffer[..bytes_read]);
        Ok(hasher.finish())
    }
}

/// Builder for creating indices
pub struct IndexBuilder {
    source_file: PathBuf,
    primary_column: String,
    secondary_columns: Vec<String>,
    chunk_size: usize,
}

impl IndexBuilder {
    /// Create a new index builder
    pub fn new(source_file: PathBuf, primary_column: String) -> Self {
        Self {
            source_file,
            primary_column,
            secondary_columns: Vec::new(),
            chunk_size: 1024 * 1024, // 1MB default
        }
    }

    /// Add a secondary index
    pub fn add_secondary_index(mut self, column: String) -> Self {
        self.secondary_columns.push(column);
        self
    }

    /// Set chunk size for building
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    /// Build the index
    pub fn build(self) -> Result<FileIndex> {
        let file = File::open(&self.source_file)
            .map_err(|e| ExtractorError::io_error(e, &self.source_file))?;
            
        let metadata = file.metadata()
            .map_err(|e| ExtractorError::io_error(e, &self.source_file))?;

        let mut builder = IndexBuilderState {
            file,
            primary_column: self.primary_column,
            secondary_columns: self.secondary_columns,
            chunk_size: self.chunk_size,
            positions: HashMap::new(),
            secondary_indices: HashMap::new(),
        };

        builder.build_index()?;

        Ok(FileIndex {
            metadata: IndexMetadata {
                source_file: self.source_file,
                file_size: metadata.len(),
                modified_time: metadata.modified()
                    .map_err(|e| ExtractorError::io_error(e, &self.source_file))?
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                file_checksum: builder.calculate_checksum()?,
                row_count: builder.positions.len() as u64,
                header_position: Position {
                    offset: 0,
                    length: 0,
                    row_number: 0,
                },
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            },
            columns: vec![self.primary_column],
            primary_column: self.primary_column,
            positions: builder.positions,
            secondary_indices: builder.secondary_indices,
        })
    }
}

/// Internal state for index building
struct IndexBuilderState {
    file: File,
    primary_column: String,
    secondary_columns: Vec<String>,
    chunk_size: usize,
    positions: HashMap<String, Position>,
    secondary_indices: HashMap<String, HashMap<String, Vec<Position>>>,
}

impl IndexBuilderState {
    fn build_index(&mut self) -> Result<()> {
        // Implementation details for building the index
        // This would involve:
        // 1. Reading the CSV in chunks
        // 2. Building primary and secondary indices
        // 3. Handling errors and edge cases
        todo!("Implement index building")
    }

    fn calculate_checksum(&self) -> Result<u64> {
        let mut buffer = [0u8; 8192];
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let mut handle = self.file.try_clone()
            .map_err(|e| ExtractorError::io_error(e, "Failed to clone file handle"))?;
            
        handle.seek(SeekFrom::Start(0))
            .map_err(|e| ExtractorError::io_error(e, "Failed to seek to start"))?;
            
        let bytes_read = handle.read(&mut buffer)
            .map_err(|e| ExtractorError::io_error(e, "Failed to read file"))?;
            
        use std::hash::Hasher;
        hasher.write(&buffer[..bytes_read]);
        Ok(hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_index_creation() -> Result<()> {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "id,name,value").unwrap();
        writeln!(temp_file, "1,test1,100").unwrap();
        writeln!(temp_file, "2,test2,200").unwrap();

        let index = FileIndex::builder(
            temp_file.path().to_owned(),
            "id".to_string(),
        )
        .add_secondary_index("name".to_string())
        .build()?;

        assert_eq!(index.metadata.row_count, 2);
        Ok(())
    }

    #[test]
    fn test_index_serialization() -> Result<()> {
        let index = FileIndex::builder(
            PathBuf::from("test.csv"),
            "id".to_string(),
        )
        .build()?;

        let temp_index = NamedTempFile::new().unwrap();
        index.save(temp_index.path())?;

        let loaded = FileIndex::load(temp_index.path())?;
        assert_eq!(loaded.primary_column, "id");
        Ok(())
    }
}