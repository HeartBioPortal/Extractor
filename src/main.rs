mod config;
mod error;
mod types;
mod extractors;

use clap::Parser;
use error::{Result, ExtractorError};
use types::{Config, ExtractorArgs};
use std::process;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Whether to run in SGA mode
    #[arg(short, long)]
    sga: bool,

    /// List of CVD names (JSON array)
    #[arg(short, long)]
    cvd_names: String,

    /// List of trait names (JSON array)
    #[arg(short, long)]
    trait_names: String,

    /// Gene name to filter by
    #[arg(short, long)]
    gene: String,
}

fn run() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Load configuration
    let config = config::load_config()
        .map_err(|e| ExtractorError::Config(e))?;

    // Parse command line arguments
    let cli = Args::parse();

    // Parse JSON arrays from command line
    let cvd_names: Vec<String> = serde_json::from_str(&cli.cvd_names)
        .map_err(|e| ExtractorError::Json(e))?;
    
    let trait_names: Vec<String> = serde_json::from_str(&cli.trait_names)
        .map_err(|e| ExtractorError::Json(e))?;

    let args = ExtractorArgs {
        is_sga: cli.sga,
        cvd_names,
        trait_names,
        gene_name: cli.gene,
    };

    // Run appropriate extractor based on mode
    if args.is_sga {
        log::info!("Running in SGA mode for gene: {}", args.gene_name);
        extractors::sga::extract_sga_data(&config, &args)?;
    } else {
        log::info!("Running gene extraction for: {}", args.gene_name);
        extractors::gene::extract_gene_data(&config, &args)?;
    }

    log::info!("Extraction completed successfully");
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        process::exit(1);
    }
}