use std::io::{BufRead, BufReader};
use std::fs::File;
use std::path::Path;
use super::{BioRecord, BioFilter};
use crate::Result;

/// Represents a FASTA record
#[derive(Debug, Clone)]
pub struct FastaRecord {
    /// Sequence identifier
    id: String,
    /// Optional description
    description: Option<String>,
    /// Sequence data
    sequence: Vec<u8>,
    /// Additional metadata
    metadata: Vec<(String, String)>,
}

impl FastaRecord {
    /// Create a new FASTA record
    pub fn new(id: String, sequence: Vec<u8>) -> Self {
        Self {
            id,
            description: None,
            sequence,
            metadata: Vec::new(),
        }
    }

    /// Add description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.push((key, value));
    }
}

impl BioRecord for FastaRecord {
    fn id(&self) -> &str {
        &self.id
    }

    fn sequence(&self) -> Option<&[u8]> {
        Some(&self.sequence)
    }

    fn quality(&self) -> Option<&[u8]> {
        None
    }

    fn metadata(&self) -> &[(String, String)] {
        &self.metadata
    }

    fn to_string(&self) -> String {
        let mut output = String::with_capacity(self.sequence.len() + 100);
        output.push('>');
        output.push_str(&self.id);
        if let Some(desc) = &self.description {
            output.push(' ');
            output.push_str(desc);
        }
        output.push('\n');
        
        // Format sequence in lines of 60 characters
        for chunk in self.sequence.chunks(60) {
            output.push_str(&String::from_utf8_losix(chunk).unwrap_or_default());
            output.push('\n');
        }
        
        output
    }
}

impl BioFilter for FastaRecord {
    fn gc_content(&self) -> f64 {
        let mut gc_count = 0;
        let total = self.sequence.len();
        
        for &base in &self.sequence {
            match base.to_ascii_uppercase() {
                b'G' | b'C' => gc_count += 1,
                _ => {}
            }
        }
        
        if total > 0 {
            gc_count as f64 / total as f64
        } else {
            0.0
        }
    }

    fn sequence_length(&self) -> usize {
        self.sequence.len()
    }

    fn min_quality_score(&self) -> Option<u8> {
        None
    }

    fn contains_pattern(&self, pattern: &[u8]) -> bool {
        self.sequence.windows(pattern.len())
            .any(|window| window.eq_ignore_ascii_case(pattern))
    }
}

/// FASTA file parser
pub struct FastaReader<R: BufRead> {
    reader: R,
    current_line: String,
}

impl<R: BufRead> FastaReader<R> {
    /// Create a new FASTA reader
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            current_line: String::new(),
        }
    }

    /// Create from file path
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<FastaReader<BufReader<File>>> {
        let file = File::open(path)?;
        Ok(FastaReader::new(BufReader::new(file)))
    }
}

impl<R: BufRead> Iterator for FastaReader<R> {
    type Item = Result<FastaRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut id = String::new();
        let mut description = None;
        let mut sequence = Vec::new();
        
        // Find next header line
        loop {
            self.current_line.clear();
            match self.reader.read_line(&mut self.current_line) {
                Ok(0) => return None, // EOF
                Ok(_) => {
                    let line = self.current_line.trim();
                    if line.starts_with('>') {
                        // Parse header
                        let header = &line[1..];
                        if let Some(space_idx) = header.find(' ') {
                            id = header[..space_idx].to_string();
                            description = Some(header[space_idx + 1..].to_string());
                        } else {
                            id = header.to_string();
                        }
                        break;
                    }
                },
                Err(e) => return Some(Err(e.into())),
            }
        }

        // Read sequence lines until next header or EOF
        loop {
            self.current_line.clear();
            match self.reader.read_line(&mut self.current_line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let line = self.current_line.trim();
                    if line.starts_with('>') {
                        // Next record found
                        break;
                    }
                    sequence.extend(line.bytes());
                },
                Err(e) => return Some(Err(e.into())),
            }
        }

        Some(Ok(FastaRecord::new(id, sequence).with_description(description.unwrap_or_default())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_fasta_parsing() {
        let data = r#">seq1 description
ACGTACGT
GGCCTTAA
>seq2
CCGGAATT
"#;
        let cursor = Cursor::new(data);
        let reader = FastaReader::new(cursor);
        let records: Result<Vec<_>> = reader.collect();
        let records = records.unwrap();
        
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].id(), "seq1");
        assert_eq!(records[0].sequence(), Some("ACGTACGTGGCCTTAA".as_bytes()));
        assert_eq!(records[1].id(), "seq2");
        assert_eq!(records[1].sequence(), Some("CCGGAATT".as_bytes()));
    }

    #[test]
    fn test_gc_content() {
        let record = FastaRecord::new(
            "test".to_string(),
            "GCGCATAT".as_bytes().to_vec()
        );
        assert_eq!(record.gc_content(), 0.5);
    }
}