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