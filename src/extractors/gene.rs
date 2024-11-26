use std::path::PathBuf;
use glob::glob;
use crate::error::Result;
use crate::types::{Config, ExtractorArgs};
use super::{create_csv_reader, create_csv_writer, GWAS_HEADERS, TRAIT_HEADERS};

pub fn extract_gene_data(config: &Config, args: &ExtractorArgs) -> Result<()> {
    // Process GWAS files
    let gwas_output = PathBuf::from(&config.paths.output)
        .join(&config.files.gwas_output);
    process_gwas_files(
        &config.paths.gwas,
        &gwas_output,
        config.processing.gwas_delimiter,
        &args.cvd_names,
        &args.gene_name,
    )?;

    // Process trait files
    let trait_output = PathBuf::from(&config.paths.output)
        .join(&config.files.trait_output);
    process_trait_files(
        &config.paths.trait_data,
        &trait_output,
        config.processing.trait_delimiter,
        &args.trait_names,
        &args.gene_name,
    )?;

    Ok(())
}

fn process_gwas_files(
    input_path: &str,
    output_path: &PathBuf,
    delimiter: char,
    cvd_names: &[String],
    gene_name: &str,
) -> Result<()> {
    let mut writer = create_csv_writer(output_path)?;
    writer.write_record(&GWAS_HEADERS)?;

    let pattern = format!("{}/*.txt", input_path);
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        if let Ok(path) = entry {
            let file_stem = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");

            if cvd_names.iter().any(|name| file_stem.contains(name)) {
                let mut reader = create_csv_reader(&path, delimiter)?;
                
                for result in reader.records() {
                    let record = result?;
                    // gene_id is at index 21 for GWAS files
                    if record.get(21).map_or(false, |id| id == gene_name) {
                        writer.write_record(&record)?;
                    }
                }
            }
        }
    }
    
    writer.flush()?;
    Ok(())
}

fn process_trait_files(
    input_path: &str,
    output_path: &PathBuf,
    delimiter: char,
    trait_names: &[String],
    gene_name: &str,
) -> Result<()> {
    let mut writer = create_csv_writer(output_path)?;
    writer.write_record(&TRAIT_HEADERS)?;

    let pattern = format!("{}/*.txt", input_path);
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        if let Ok(path) = entry {
            let file_stem = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");

            if trait_names.iter().any(|name| file_stem.contains(name)) {
                let mut reader = create_csv_reader(&path, delimiter)?;
                
                for result in reader.records() {
                    let record = result?;
                    // gene_id is at index 23 for trait files
                    if record.get(23).map_or(false, |id| id == gene_name) {
                        writer.write_record(&record)?;
                    }
                }
            }
        }
    }
    
    writer.flush()?;
    Ok(())
}