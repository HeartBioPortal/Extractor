use std::path::PathBuf;
use glob::glob;
use crate::error::Result;
use crate::types::{Config, ExtractorArgs};
use super::{create_csv_reader, create_csv_writer, GWAS_HEADERS, TRAIT_HEADERS};

pub fn extract_sga_data(config: &Config, args: &ExtractorArgs) -> Result<()> {
    let output_path = PathBuf::from(&config.paths.output)
        .join(&config.files.sga_output);
    let mut writer = create_csv_writer(&output_path)?;
    
    // Write headers (excluding Phenotype and Study columns for SGA)
    let filtered_headers: Vec<&str> = GWAS_HEADERS.iter()
        .enumerate()
        .filter(|(i, _)| *i != 2 && *i != 3) // Exclude Phenotype and Study columns
        .map(|(_, &h)| h)
        .collect();
    writer.write_record(&filtered_headers)?;

    // Process GWAS files
    process_sga_gwas_files(
        &config.paths.gwas,
        &mut writer,
        config.processing.gwas_delimiter,
        &args.gene_name,
    )?;

    // Process trait files
    process_sga_trait_files(
        &config.paths.trait_data,
        &mut writer,
        config.processing.trait_delimiter,
        &args.gene_name,
    )?;

    writer.flush()?;
    Ok(())
}

fn process_sga_gwas_files(
    input_path: &str,
    writer: &mut csv::Writer<std::fs::File>,
    delimiter: char,
    gene_name: &str,
) -> Result<()> {
    let pattern = format!("{}/*.txt", input_path);
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        if let Ok(path) = entry {
            let mut reader = create_csv_reader(&path, delimiter)?;
            
            for result in reader.records() {
                let record = result?;
                // gene_id is at index 21 for GWAS files
                if record.get(21).map_or(false, |id| id == gene_name) {
                    // Create new record excluding Phenotype and Study columns
                    let filtered_record: Vec<String> = record.iter()
                        .enumerate()
                        .filter(|(i, _)| *i != 2 && *i != 3)
                        .map(|(_, field)| field.to_string())
                        .collect();
                    writer.write_record(&filtered_record)?;
                }
            }
        }
    }
    Ok(())
}

fn process_sga_trait_files(
    input_path: &str,
    writer: &mut csv::Writer<std::fs::File>,
    delimiter: char,
    gene_name: &str,
) -> Result<()> {
    let pattern = format!("{}/*.txt", input_path);
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        if let Ok(path) = entry {
            let mut reader = create_csv_reader(&path, delimiter)?;
            
            for result in reader.records() {
                let record = result?;
                // gene_id is at index 23 for trait files
                if record.get(23).map_or(false, |id| id == gene_name) {
                    // Create new record excluding Phenotype and Study columns
                    let filtered_record: Vec<String> = record.iter()
                        .enumerate()
                        .filter(|(i, _)| *i != 2 && *i != 3)
                        .map(|(_, field)| field.to_string())
                        .collect();
                    writer.write_record(&filtered_record)?;
                }
            }
        }
    }
    Ok(())
}