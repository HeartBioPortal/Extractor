use std::fs;
use std::path::{Path, PathBuf};
use extractor::{
    config,
    types::{Config, ExtractorArgs},
    extractors::{gene, sga},
};

fn setup_test_data() -> PathBuf {
    let test_dir = PathBuf::from("tests/data");
    fs::create_dir_all(&test_dir).unwrap();
    
    // Create test GWAS file
    let gwas_content = "\
MarkerID,pval,Phenotype,Study,PMID,StudyGenomeBuild,snpeff.ann.gene_id
rs123,0.05,CVD1,Study1,12345,hg19,GENE1
rs456,0.01,CVD1,Study1,12345,hg19,GENE2
rs789,0.03,CVD2,Study2,12346,hg19,GENE1";
    
    fs::write(test_dir.join("test_gwas.txt"), gwas_content).unwrap();
    
    // Create test trait file
    let trait_content = "\
MarkerID\tpval\tPhenotype\tStudy\tPMID\tStudyGenomeBuild\tsnpeff.ann.gene_id
rs123\t0.05\tTRAIT1\tStudy1\t12345\thg19\tGENE1
rs456\t0.01\tTRAIT1\tStudy1\t12345\thg19\tGENE2
rs789\t0.03\tTRAIT2\tStudy2\t12346\thg19\tGENE1";
    
    fs::write(test_dir.join("test_trait.txt"), trait_content).unwrap();
    
    test_dir
}

fn cleanup_test_data(test_dir: &Path) {
    fs::remove_dir_all(test_dir).unwrap();
}

fn create_test_config(test_dir: &Path) -> Config {
    Config {
        paths: config::PathConfig {
            gwas: test_dir.to_str().unwrap().to_string(),
            trait_data: test_dir.to_str().unwrap().to_string(),
            output: test_dir.to_str().unwrap().to_string(),
        },
        files: config::FileConfig {
            gwas_output: "test_gwas_output.csv".to_string(),
            trait_output: "test_trait_output.csv".to_string(),
            sga_output: "test_sga_output.csv".to_string(),
        },
        processing: config::ProcessingConfig {
            gwas_delimiter: ',',
            trait_delimiter: '\t',
        },
    }
}

#[test]
fn test_gene_extraction() {
    let test_dir = setup_test_data();
    let config = create_test_config(&test_dir);
    
    let args = ExtractorArgs {
        is_sga: false,
        cvd_names: vec!["CVD1".to_string()],
        trait_names: vec!["TRAIT1".to_string()],
        gene_name: "GENE1".to_string(),
    };
    
    gene::extract_gene_data(&config, &args).unwrap();
    
    // Verify output files exist and contain correct data
    let gwas_output = fs::read_to_string(test_dir.join("test_gwas_output.csv")).unwrap();
    assert!(gwas_output.contains("GENE1"));
    assert!(!gwas_output.contains("GENE2"));
    
    let trait_output = fs::read_to_string(test_dir.join("test_trait_output.csv")).unwrap();
    assert!(trait_output.contains("GENE1"));
    assert!(!trait_output.contains("GENE2"));
    
    cleanup_test_data(&test_dir);
}

#[test]
fn test_sga_extraction() {
    let test_dir = setup_test_data();
    let config = create_test_config(&test_dir);
    
    let args = ExtractorArgs {
        is_sga: true,
        cvd_names: vec![],
        trait_names: vec![],
        gene_name: "GENE1".to_string(),
    };
    
    sga::extract_sga_data(&config, &args).unwrap();
    
    // Verify SGA output file exists and contains correct data
    let sga_output = fs::read_to_string(test_dir.join("test_sga_output.csv")).unwrap();
    assert!(sga_output.contains("GENE1"));
    assert!(!sga_output.contains("GENE2"));
    // Verify Phenotype and Study columns are excluded
    assert!(!sga_output.contains("Study1"));
    assert!(!sga_output.contains("CVD1"));
    
    cleanup_test_data(&test_dir);
}