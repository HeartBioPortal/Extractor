//! Core processing logic for the Extractor library.
//! Implements the main filtering and processing functionality.

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use rayon::prelude::*;
use csv::{ReaderBuilder, WriterBuilder};
use crossbeam_channel::{bounded, Sender};

use crate::{Config, ProcessingStats};
use crate::error::{ExtractorError, FilterErrorKind};
use crate::filters::Filter;
use crate::index::FileIndex;
use crate::utils::{self, Progress, SafeMmapOptions};
use crate::Result;

/// Chunk of data to be processed
struct Chunk {
    data: Vec<u8>,
    start_offset: u64,
    chunk_index: usize,
}

/// Main processing engine
pub struct BioFilter {
    config: Config,
    filters: Vec<Box<dyn Filter>>,
    input_path: PathBuf,
    output_path: PathBuf,
    index: Option<Arc<FileIndex>>,
    stats: Arc<ProcessingStats>,
}

impl BioFilter {
    /// Create a new BioFilter instance
    pub fn new(
        input_path: PathBuf,
        output_path: PathBuf,
        config: Config,
        index_path: Option<PathBuf>,
    ) -> Result<Self> {
        // Validate input file
        if !input_path.exists() {
            return Err(ExtractorError::io_error(
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Input file does not exist"
                ),
                &input_path
            ));
        }

        // Load index if specified
        let index = if let Some(idx_path) = index_path {
            Some(Arc::new(FileIndex::load(&idx_path)?))
        } else {
            None
        };

        Ok(Self {
            config,
            filters: Vec::new(),
            input_path,
            output_path,
            index,
            stats: Arc::new(ProcessingStats::default()),
        })
    }

    /// Add a filter to the processing pipeline
    pub fn add_filter(&mut self, filter: Box<dyn Filter>) {
        self.filters.push(filter);
    }

    /// Process the input file
    pub fn process(&self) -> Result<ProcessingStats> {
        if self.config.use_index && self.index.is_none() {
            return Err(ExtractorError::Config(
                "Index required but not loaded".to_string()
            ));
        }

        let input_file = File::open(&self.input_path)
            .map_err(|e| ExtractorError::io_error(e, &self.input_path))?;

        let output_file = File::create(&self.output_path)
            .map_err(|e| ExtractorError::io_error(e, &self.output_path))?;

        if self.config.parallel {
            self.process_parallel(input_file, output_file)
        } else {
            self.process_sequential(input_file, output_file)
        }
    }

    /// Process file in parallel using multiple threads
    fn process_parallel(&self, input: File, output: File) -> Result<ProcessingStats> {
        let file_size = input.metadata()?.len();
        let chunk_size = self.config.chunk_size;
        let num_chunks = (file_size + chunk_size as u64 - 1) / chunk_size as u64;

        // Set up progress tracking
        #[cfg(feature = "progress-bars")]
        let progress = Arc::new(Progress::new(
            file_size,
            "Processing file"
        ));

        // Create channels for collecting results
        let (tx, rx) = bounded(self.config.num_threads.unwrap_or_else(num_cpus::get));
        let output = Arc::new(std::sync::Mutex::new(BufWriter::new(output)));

        // Process chunks in parallel
        let processed_rows = Arc::new(AtomicU64::new(0));
        let matched_rows = Arc::new(AtomicU64::new(0));

        let mmap = unsafe {
            utils::create_mmap(&input, &SafeMmapOptions::default())?
        };
        let mmap = Arc::new(mmap);
        // Spawn processing threads
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.config.num_threads.unwrap_or_else(num_cpus::get))
            .build()?;

        pool.scope(|s| {
            // Split file into chunks and process
            for chunk_index in 0..num_chunks {
                let start = chunk_index as u64 * chunk_size as u64;
                let end = std::cmp::min(start + chunk_size as u64, file_size);
                
                let tx = tx.clone();
                let mmap = mmap.clone();
                let filters = &self.filters;
                let processed_rows = processed_rows.clone();
                let matched_rows = matched_rows.clone();
                
                #[cfg(feature = "progress-bars")]
                let progress = progress.clone();

                s.spawn(move |_| {
                    let chunk_data = &mmap[start as usize..end as usize];
                    let chunk = Chunk {
                        data: chunk_data.to_vec(),
                        start_offset: start,
                        chunk_index: chunk_index as usize,
                    };

                    if let Ok(results) = self.process_chunk(&chunk, filters) {
                        processed_rows.fetch_add(results.rows_processed, Ordering::Relaxed);
                        matched_rows.fetch_add(results.rows_matched, Ordering::Relaxed);
                        
                        #[cfg(feature = "progress-bars")]
                        progress.inc(chunk_data.len() as u64);

                        // Send processed results
                        let _ = tx.send((chunk_index, results));
                    }
                });
            }
        });

        // Collect and write results in order
        drop(tx);
        let mut results = vec![];
        while let Ok(result) = rx.recv() {
            results.push(result);
        }
        results.sort_by_key(|(idx, _)| *idx);

        let mut output = output.lock().unwrap();
        for (_, chunk_result) in results {
            output.write_all(&chunk_result.output_data)?;
        }
        output.flush()?;

        #[cfg(feature = "progress-bars")]
        progress.finish();

        Ok(ProcessingStats {
            rows_processed: processed_rows.load(Ordering::Relaxed),
            rows_matched: matched_rows.load(Ordering::Relaxed),
            processing_time_ms: 0, // TODO: Add timing
            input_size: file_size,
            output_size: self.output_path.metadata()?.len(),
        })
    }

    /// Process file sequentially in a single thread
    fn process_sequential(&self, input: File, output: File) -> Result<ProcessingStats> {
        let mut reader = ReaderBuilder::new()
            .delimiter(self.config.delimiter)
            .has_headers(self.config.has_headers)
            .from_reader(input);

        let mut writer = WriterBuilder::new()
            .delimiter(self.config.delimiter)
            .from_writer(output);

        let headers = reader.headers()?.clone();
        writer.write_record(&headers)?;

        let mut stats = ProcessingStats::default();
        stats.input_size = self.input_path.metadata()?.len();

        for result in reader.records() {
            let record = result?;
            stats.rows_processed += 1;

            if self.apply_filters(&record)? {
                writer.write_record(&record)?;
                stats.rows_matched += 1;
            }
        }

        writer.flush()?;
        stats.output_size = self.output_path.metadata()?.len();
        Ok(stats)
    }

    /// Process a single chunk of data
    fn process_chunk(
        &self,
        chunk: &Chunk,
        filters: &[Box<dyn Filter>],
    ) -> Result<ChunkResult> {
        let mut result = ChunkResult {
            rows_processed: 0,
            rows_matched: 0,
            output_data: Vec::with_capacity(chunk.data.len()),
        };

        // Create a writer that writes to our output buffer
        let mut writer = WriterBuilder::new()
            .delimiter(self.config.delimiter)
            .from_writer(&mut result.output_data);

        // Find complete rows in the chunk
        let mut start = 0;
        let mut in_quoted_field = false;
        let mut row_start = 0;
        
        // Skip incomplete row at start if this isn't the first chunk
        if chunk.chunk_index > 0 {
            while start < chunk.data.len() && chunk.data[start] != b'\n' {
                start += 1;
            }
            start += 1;
            row_start = start;
        }

        // Process each row in the chunk
        for (i, &byte) in chunk.data[start..].iter().enumerate() {
            let pos = start + i;

            // Handle quoted fields
            if byte == b'"' {
                in_quoted_field = !in_quoted_field;
                continue;
            }

            // Only process row endings outside of quotes
            if !in_quoted_field && byte == b'\n' {
                let row_data = &chunk.data[row_start..pos];
                result.rows_processed += 1;

                // Skip empty rows
                if row_data.is_empty() {
                    row_start = pos + 1;
                    continue;
                }

                // Parse the row
                if let Ok(should_keep) = self.process_row(row_data, filters) {
                    if should_keep {
                        // Write the row to output
                        writer.write_record(row_data.split(|&b| b == self.config.delimiter))?;
                        result.rows_matched += 1;
                    }
                }

                row_start = pos + 1;
            }
        }

        // Flush the writer to ensure all data is written to our buffer
        writer.flush()?;
        Ok(result)
    }

    /// Process a single row of data
    fn process_row(&self, row_data: &[u8], filters: &[Box<dyn Filter>]) -> Result<bool> {
        // Get cached headers
        let headers = self.get_headers()?;

        // Apply all filters
        for filter in filters {
            if !filter.apply(row_data, &headers)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Helper method to parse a row into fields
    fn parse_row<'a>(&self, row: &'a [u8]) -> Vec<&'a [u8]> {
        let mut fields = Vec::new();
        let mut start = 0;
        let mut in_quotes = false;
        
        for (i, &byte) in row.iter().enumerate() {
            match byte {
                b'"' => in_quotes = !in_quotes,
                b',' if !in_quotes => {
                    fields.push(&row[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }
        
        // Add the last field
        if start < row.len() {
            fields.push(&row[start..]);
        }
        
        fields
    }

    /// Apply filters to a record
    fn apply_filters(&self, record: &csv::StringRecord) -> Result<bool> {
        for filter in &self.filters {
            if !filter.apply(record.as_bytes(), &self.get_headers()?)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Get CSV headers as a map of column names to indices
    fn get_headers(&self) -> Result<std::collections::HashMap<String, usize>> {
        let file = File::open(&self.input_path)?;
        let mut reader = ReaderBuilder::new()
            .delimiter(self.config.delimiter)
            .has_headers(true)
            .from_reader(file);

        let headers = reader.headers()?;
        Ok(headers
            .iter()
            .enumerate()
            .map(|(i, name)| (name.to_string(), i))
            .collect())
    }
}

#[derive(Debug)]
pub(crate) struct ChunkProcessingStats {
    pub rows_processed: u64,
    pub rows_matched: u64,
    pub bytes_processed: u64,
}

/// Helper for managing chunk boundaries
struct ChunkBoundary {
    start: usize,
    end: usize,
    is_complete: bool,
}

impl ChunkBoundary {
    /// Find the actual boundaries of complete rows within a chunk
    fn find_boundaries(data: &[u8], chunk_size: usize) -> Self {
        let mut end = chunk_size;
        if end > data.len() {
            end = data.len();
        }

        // Adjust end to nearest newline
        while end < data.len() && data[end] != b'\n' {
            end += 1;
        }

        // Check if we have a complete chunk
        let is_complete = end < data.len() || data[data.len() - 1] == b'\n';

        Self {
            start: 0,
            end,
            is_complete,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filters::{ColumnFilter, FilterCondition};

    #[test]
    fn test_chunk_processing() -> Result<()> {
        let chunk_data = b"name,value\ntest1,100\ntest2,200\n";
        let chunk = Chunk {
            data: chunk_data.to_vec(),
            start_offset: 0,
            chunk_index: 0,
        };

        let mut filter = BioFilter::new(
            PathBuf::from("test.csv"),
            PathBuf::from("output.csv"),
            Config::default(),
            None,
        )?;

        // Add a test filter
        filter.add_filter(Box::new(ColumnFilter::new(
            "value".to_string(),
            FilterCondition::Numeric(NumericCondition::GreaterThan(150.0)),
        )?));

        let result = filter.process_chunk(&chunk, &filter.filters)?;
        assert_eq!(result.rows_processed, 2);
        assert_eq!(result.rows_matched, 1); // Only test2,200 should match
        
        Ok(())
    }

    #[test]
    fn test_quoted_fields() -> Result<()> {
        let chunk_data = b"name,value\n\"test,1\",100\n\"test,2\",200\n";
        let chunk = Chunk {
            data: chunk_data.to_vec(),
            start_offset: 0,
            chunk_index: 0,
        };

        let filter = BioFilter::new(
            PathBuf::from("test.csv"),
            PathBuf::from("output.csv"),
            Config::default(),
            None,
        )?;

        let result = filter.process_chunk(&chunk, &[])?;
        assert_eq!(result.rows_processed, 2);
        assert_eq!(result.rows_matched, 2); // All rows should match with no filters
        
        Ok(())
    }

    #[test]
    fn test_chunk_boundaries() {
        let data = b"header\nrow1\nrow2\nrow3";
        let boundary = ChunkBoundary::find_boundaries(data, 10);
        assert!(boundary.end > boundary.start);
        assert!(data[boundary.end] == b'\n' || boundary.end == data.len());
    }
}

/// Results from processing a chunk of data
#[derive(Debug)]
struct ChunkResult {
    rows_processed: u64,
    rows_matched: u64,
    output_data: Vec<u8>,
}

impl Default for ProcessingStats {
    fn default() -> Self {
        Self {
            rows_processed: 0,
            rows_matched: 0,
            processing_time_ms: 0,
            input_size: 0,
            output_size: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_basic_filtering() -> Result<()> {
        // Create test data
        let mut input = NamedTempFile::new()?;
        writeln!(input, "name,value")?;
        writeln!(input, "test1,100")?;
        writeln!(input, "test2,200")?;

        let output = NamedTempFile::new()?;
        
        let mut filter = BioFilter::new(
            input.path().to_owned(),
            output.path().to_owned(),
            Config::default(),
            None,
        )?;

        // Add test filter
        filter.add_filter(Box::new(TestFilter));
        
        let stats = filter.process()?;
        assert_eq!(stats.rows_processed, 2);
        Ok(())
    }
}

// Test helper filter implementation
struct TestFilter;

impl Filter for TestFilter {
    fn apply(&self, _row: &[u8], _headers: &std::collections::HashMap<String, usize>) -> Result<bool> {
        Ok(true)
    }

    fn column_name(&self) -> &str {
        "test"
    }

    fn description(&self) -> String {
        "Test filter".to_string()
    }
}