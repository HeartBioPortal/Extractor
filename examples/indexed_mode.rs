use extractor::{
    BioFilter, Config, FileIndex, FilterCondition, 
    ColumnFilter, NumericCondition, RangeCondition
};
use std::path::PathBuf;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create sample data
    create_large_dataset()?;

    // Demonstrate different indexing scenarios
    println!("1. Creating and Using Primary Index");
    primary_index_example()?;

    println!("\n2. Secondary Index Usage");
    secondary_index_example()?;
    
    Ok(())
}

/// Example 1: Basic primary index usage
fn primary_index_example() -> Result<(), Box<dyn std::error::Error>> {
    // Create index on gene_id
    let index = FileIndex::builder("large_dataset.csv", "gene_id")
        .build()?;
    index.save("gene_index.json")?;

    // Use index for filtering
    let mut filter = BioFilter::builder("large_dataset.csv", "output_primary.csv")
        .with_index("gene_index.json")
        .build()?;

    filter.add_filter(Box::new(ColumnFilter::new(
        "gene_id".to_string(),
        FilterCondition::Regex("ENSG.*001".to_string())
    )?));

    let stats = filter.process()?;
    println!("Found {} matching genes using primary index", stats.rows_matched);
    Ok(())
}

// Example 2: Using secondary indices
fn secondary_index_example() -> Result<(), Box<dyn std::error::Error>> {
    // Create index with secondary columns
    let index = FileIndex::builder("large_dataset.csv", "gene_id")
        .add_secondary_index("chromosome")
        .add_secondary_index("gene_type")
        .build()?;
    index.save("multi_index.json")?;

    // Use secondary index for chromosome-based query
    let mut filter = BioFilter::builder("large_dataset.csv", "output_secondary.csv")
        .with_index("multi_index.json")
        .build()?;

    filter.add_filter(Box::new(ColumnFilter::new(
        "chromosome".to_string(),
        FilterCondition::Equals("chr1".to_string())
    )?));

    filter.add_filter(Box::new(ColumnFilter::new(
        "gene_type".to_string(),
        FilterCondition::Equals("protein_coding".to_string())
    )?));

    let stats = filter.process()?;
    println!("Found {} protein-coding genes on chr1", stats.rows_matched);
    Ok(())
}

/// Create a large sample dataset
fn create_large_dataset() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create("large_dataset.csv")?;
    
    // Write header
    writeln!(file, "gene_id,gene_name,chromosome,start,end,strand,gene_type,tpm,fpkm")?;

    // Generate 100,000 sample genes
    for i in 0..100_000 {
        let gene_id = format!("ENSG{:011}", i);
        let gene_name = format!("GENE_{}", i);
        let chr = format!("chr{}", (i % 23) + 1);
        let start = i * 1000 + 1;
        let end = start + 999;
        let strand = if i % 2 == 0 { "+" } else { "-" };
        let gene_type = match i % 5 {
            0 => "protein_coding",
            1 => "lncRNA",
            2 => "miRNA",
            3 => "pseudogene",
            _ => "other",
        };
        let tpm = (i as f64 % 1000.0) + 0.1;
        let fpkm = tpm * 1.2;

        writeln!(file, "{},{},{},{},{},{},{},{:.2},{:.2}",
            gene_id, gene_name, chr, start, end, strand, gene_type, tpm, fpkm)?;
    }

    Ok(())
}

/// Helper function to read a specific row using its position
fn read_row_at_position(file_path: &str, position: &Position) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};

    let mut file = File::open(file_path)?;
    let mut buffer = vec![0; position.length as usize];
    
    file.seek(SeekFrom::Start(position.offset))?;
    file.read_exact(&mut buffer)?;
    
    Ok(buffer)
}