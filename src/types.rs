use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub paths: PathConfig,
    pub files: FileConfig,
    pub processing: ProcessingConfig,
}

#[derive(Debug, Deserialize)]
pub struct PathConfig {
    pub gwas: String,
    pub trait_data: String,
    pub output: String,
}

#[derive(Debug, Deserialize)]
pub struct FileConfig {
    pub gwas_output: String,
    pub trait_output: String,
    pub sga_output: String,
}

#[derive(Debug, Deserialize)]
pub struct ProcessingConfig {
    pub gwas_delimiter: char,
    pub trait_delimiter: char,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractorArgs {
    pub is_sga: bool,
    pub cvd_names: Vec<String>,
    pub trait_names: Vec<String>,
    pub gene_name: String,
}