use extractor::{
    BioFilter, Config, FilterCondition, NumericCondition,
    ColumnFilter, FileIndex,
};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate sample data for examples
    create_sample_data()?;

    println!("1. Basic Filtering - Gene Expression Analysis");
    expression_analysis()?;

    println!("\n2. Multiple Conditions - QC Filtering");
    quality_control_filtering()?;
    
    println!("\n3. Chromosomal Region Analysis");
    chromosome_analysis()?;

    println!("\n4. P-value Based Filtering");
    pvalue_filtering()?;

    println!("\n5. Complex Filtering - DEG Analysis");
    deg_analysis()?;
    
    Ok(())
}

/// Example 1: Basic gene expression filtering
fn expression_analysis() -> Result<(), Box<dyn std::error::Error>> {
    let mut filter = BioFilter::builder("sample_data.csv", "high_expression.csv")
        .build()?;

    // Filter genes with high expression
    filter.add_filter(Box::new(ColumnFilter::new(
        "expression_level".to_string(),
        FilterCondition::Numeric(NumericCondition::GreaterThan(5.0))
    )?));

    let stats = filter.process()?;
    println!("Found {} highly expressed genes", stats.rows_matched);
    Ok(())
}


/// Example 2: Multiple QC filters
fn quality_control_filtering() -> Result<(), Box<dyn std::error::Error>> {
    let mut filter = BioFilter::builder("sample_data.csv", "qc_passed.csv")
        .build()?;

    // Add multiple QC filters
    let qc_filters = vec![
        ColumnFilter::new(
            "read_count".to_string(),
            FilterCondition::Numeric(NumericCondition::GreaterThan(100.0))
        )?,
        ColumnFilter::new(
            "mapping_quality".to_string(),
            FilterCondition::Numeric(NumericCondition::GreaterThan(30.0))
        )?,
        ColumnFilter::new(
            "duplicate_rate".to_string(),
            FilterCondition::Numeric(NumericCondition::LessThan(0.1))
        )?,
    ];

    for filter_condition in qc_filters {
        filter.add_filter(Box::new(filter_condition));
    }

    let stats = filter.process()?;
    println!("Identified {} high-quality samples", stats.rows_matched);
    Ok(())
}


/// Example 3: Chromosome-specific analysis
fn chromosome_analysis() -> Result<(), Box<dyn std::error::Error>> {
    // Create an index for faster chromosome-based queries
    let index_path = "sample_data.index";
    if !PathBuf::from(index_path).exists() {
        let index = FileIndex::builder("sample_data.csv", "chromosome")
            .build()?;
        index.save(index_path)?;
    }

    let mut filter = BioFilter::builder("sample_data.csv", "chr1_genes.csv")
        .with_index(index_path)
        .build()?;

    // Filter for genes on chromosome 1 with specific conditions
    filter.add_filter(Box::new(ColumnFilter::new(
        "chromosome".to_string(),
        FilterCondition::Equals("chr1".to_string())
    )?));

    filter.add_filter(Box::new(ColumnFilter::new(
        "start_position".to_string(),
        FilterCondition::Numeric(NumericCondition::GreaterThan(1_000_000.0))
    )?));

    let stats = filter.process()?;
    println!("Found {} genes on chromosome 1", stats.rows_matched);
    Ok(())
}

/// Example 4: Statistical significance filtering
fn pvalue_filtering() -> Result<(), Box<dyn std::error::Error>> {
    let mut filter = BioFilter::builder("sample_data.csv", "significant_genes.csv")
        .build()?;

    // Filter for statistically significant results
    filter.add_filter(Box::new(ColumnFilter::new(
        "p_value".to_string(),
        FilterCondition::Numeric(NumericCondition::LessThan(0.05))
    )?));

    filter.add_filter(Box::new(ColumnFilter::new(
        "fold_change".to_string(),
        FilterCondition::Range(RangeCondition {
            min: -2.0,
            max: 2.0,
            inclusive: false,
        })
    )?));

    let stats = filter.process()?;
    println!("Found {} differentially expressed genes", stats.rows_matched);
    Ok(())
}

/// Example 5: Complex DEG analysis

fn deg_analysis() -> Result<(), Box<dyn std::error::Error>> {
    let mut filter = BioFilter::builder("sample_data.csv", "significant_degs.csv")
        .build()?;

    // Multiple conditions for DEG analysis
    filter.add_filter(Box::new(ColumnFilter::new(
        "p_value".to_string(),
        FilterCondition::Numeric(NumericCondition::LessThan(0.05))
    )?));

    filter.add_filter(Box::new(ColumnFilter::new(
        "adj_p_value".to_string(),
        FilterCondition::Numeric(NumericCondition::LessThan(0.1))
    )?));

    filter.add_filter(Box::new(ColumnFilter::new(
        "log2fc".to_string(),
        FilterCondition::Range(RangeCondition {
            min: 1.0,
            max: f64::INFINITY,
            inclusive: true,
        })
    )?));

    filter.add_filter(Box::new(ColumnFilter::new(
        "base_mean".to_string(),
        FilterCondition::Numeric(NumericCondition::GreaterThan(10.0))
    )?));

    let stats = filter.process()?;
    println!("Found {} significant DEGs", stats.rows_matched);
    Ok(())
}

/// Create sample data for examples
fn create_sample_data() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create("sample_data.csv")?;
    
    // Write header
    writeln!(file, "gene_id,chromosome,start_position,end_position,expression_level,\
        read_count,mapping_quality,duplicate_rate,p_value,adj_p_value,log2fc,base_mean")?;

    // Generate sample data
    for i in 0..1000 {
        let chr = format!("chr{}", (i % 23) + 1);
        let start_pos = i * 1000 + 1000000;
        let end_pos = start_pos + 500;
        let expr = (i as f64) / 100.0;
        let reads = 100.0 + (i as f64);
        let mapq = 20.0 + (i % 20) as f64;
        let dup_rate = (i % 100) as f64 / 1000.0;
        let pval = (1000 - i) as f64 / 10000.0;
        let adj_pval = pval * 1.5;
        let log2fc = (i as f64 - 500.0) / 100.0;
        let base_mean = i as f64 / 10.0;

        writeln!(file, "GENE_{},{},{},{},{:.2},{:.0},{:.1},{:.3},{:.4},{:.4},{:.2},{:.1}",
            i, chr, start_pos, end_pos, expr, reads, mapq, dup_rate, pval, adj_pval, log2fc, base_mean)?;
    }

    Ok(())
}