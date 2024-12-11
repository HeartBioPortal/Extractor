use extractor::index::{FileIndex, Position};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example usage
    let index = create_index(
        "input.csv",
        "gene_id",  // column to index by
        vec!["chromosome"], // optional secondary indices
    )?;
    
    // Save the index
    index.save("input.csv.index")?;
    
    // Example of using the index
    let position = index.get_position("GENE_123")?;
    println!("GENE_123 is at byte offset: {}", position.offset);
    
    // Read the specific row using the index
    let row = read_row_at_position("input.csv", position)?;
    println!("Row data: {}", String::from_utf8_lossy(&row));

    Ok(())
}

/// Creates an index for a CSV file
fn create_index(
    file_path: &str,
    primary_key: &str,
    secondary_keys: Vec<&str>,
) -> io::Result<FileIndex> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let mut positions = HashMap::new();
    let mut secondary_indices = HashMap::new();
    
    // Initialize secondary indices
    for key in &secondary_keys {
        secondary_indices.insert(key.to_string(), HashMap::new());
    }

    // Read and store header position
    let mut header_line = String::new();
    let header_pos = reader.stream_position()?;
    reader.read_line(&mut header_line)?;
    
    // Parse headers to get column indices
    let headers: Vec<String> = header_line.trim().split(',').map(String::from).collect();
    let primary_idx = headers.iter().position(|h| h == primary_key)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Primary key column not found"))?;
    
    let secondary_indices_pos: Vec<usize> = secondary_keys.iter()
        .map(|&key| headers.iter().position(|h| h == key)
            .expect("Secondary key column not found"))
        .collect();

    // Read each line and build indices
    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        let start_pos = reader.stream_position()? - line.len() as u64;
        let fields: Vec<&str> = line.trim().split(',').collect();
        
        if let Some(key_value) = fields.get(primary_idx) {
            // Store primary index position
            let position = Position {
                offset: start_pos,
                length: line.len() as u32,
                row_number: positions.len() as u64,
            };
            positions.insert(key_value.to_string(), position.clone());
            
            // Store secondary indices
            for (idx, &sec_idx) in secondary_indices_pos.iter().enumerate() {
                if let Some(sec_value) = fields.get(sec_idx) {
                    secondary_indices
                        .get_mut(secondary_keys[idx])
                        .unwrap()
                        .entry(sec_value.to_string())
                        .or_insert_with(Vec::new)
                        .push(position.clone());
                }
            }
        }
        
        line.clear();
    }

    Ok(FileIndex {
        metadata: IndexMetadata {
            source_file: PathBuf::from(file_path),
            file_size: std::fs::metadata(file_path)?.len(),
            modified_time: std::fs::metadata(file_path)?
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            file_checksum: calculate_checksum(file_path)?,
            row_count: positions.len() as u64,
            header_position: Position {
                offset: header_pos,
                length: header_line.len() as u32,
                row_number: 0,
            },
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        },
        columns: headers,
        primary_column: primary_key.to_string(),
        positions,
        secondary_indices,
    })
}

/// Read a specific row using a position from the index
fn read_row_at_position(file_path: &str, position: &Position) -> io::Result<Vec<u8>> {
    let mut file = File::open(file_path)?;
    let mut buffer = vec![0; position.length as usize];
    
    file.seek(SeekFrom::Start(position.offset))?;
    file.read_exact(&mut buffer)?;
    
    Ok(buffer)
}

/// Calculate a checksum for the file
fn calculate_checksum(file_path: &str) -> io::Result<u64> {
    let mut file = File::open(file_path)?;
    let mut buffer = [0; 8192];
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        use std::hash::Hasher;
        hasher.write(&buffer[..bytes_read]);
    }
    
    use std::hash::Hasher;
    Ok(hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_index_creation() -> Result<(), Box<dyn std::error::Error>> {
        // Create test CSV file
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "gene_id,chromosome,value")?;
        writeln!(temp_file, "GENE_1,chr1,100")?;
        writeln!(temp_file, "GENE_2,chr1,200")?;
        writeln!(temp_file, "GENE_3,chr2,300")?;

        // Create index
        let index = create_index(
            temp_file.path().to_str().unwrap(),
            "gene_id",
            vec!["chromosome"],
        )?;

        // Test primary index
        let pos = index.get_position("GENE_2").unwrap();
        let row = read_row_at_position(temp_file.path().to_str().unwrap(), pos)?;
        assert!(String::from_utf8_lossy(&row).contains("GENE_2"));

        // Test secondary index
        let chr1_rows = index.get_secondary_positions("chromosome", "chr1").unwrap();
        assert_eq!(chr1_rows.len(), 2);

        Ok(())
    }
}