//! Example suite for `extractor` demonstrating common bioinformatics filters.
//! - Reuses helpers to reduce boilerplate
//! - Avoids overwriting sample data if present
//! - Creates/uses an index for chromosome queries
//! - Prints consistent, readable summaries

use extractor::{
    BioFilter, ColumnFilter, FileIndex, FilterCondition, NumericCondition, RangeCondition,
};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type DynResult<T> = Result<T, Box<dyn Error>>;

const DATA: &str = "sample_data.csv";
const IDX_PATH: &str = "sample_data.index";

fn main() -> DynResult<()> {
    ensure_sample_data(DATA)?;

    section("1. Basic Filtering — Gene Expression");
    run_and_report(expression_analysis)?;

    section("2. Multiple Conditions — QC Filtering");
    run_and_report(quality_control_filtering)?;

    section("3. Chromosomal Region Analysis");
    run_and_report(chromosome_analysis)?;

    section("4. P-value Based Filtering");
    run_and_report(pvalue_filtering)?;

    section("5. Complex Filtering — DEG Analysis");
    run_and_report(deg_analysis)?;

    Ok(())
}

/* --------------------------------- Helpers -------------------------------- */

fn section(title: &str) {
    println!("\n=== {title} ===");
}

fn run_and_report<F>(f: F) -> DynResult<()>
where
    F: Fn() -> DynResult<usize>,
{
    let matched = f()?;
    println!("→ Rows matched: {matched}");
    Ok(())
}

fn build_filter(input: &str, output: &str, index: Option<&str>) -> DynResult<BioFilter> {
    let mut b = BioFilter::builder(input, output);
    if let Some(idx) = index {
        b = b.with_index(idx);
    }
    Ok(b.build()?)
}

fn add_filters(filter: &mut BioFilter, filters: impl IntoIterator<Item = ColumnFilter>) {
    for f in filters {
        filter.add_filter(Box::new(f));
    }
}

/// Create sample data once; skip if file already exists and is non-empty.
fn ensure_sample_data(path: &str) -> DynResult<()> {
    let p = Path::new(path);
    if p.exists() && fs::metadata(p)?.len() > 0 {
        return Ok(());
    }
    create_sample_data(p)
}

/// Generate a simple CSV with numeric columns commonly used in analyses.
fn create_sample_data(path: &Path) -> DynResult<()> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(path)?;

    // Header
    writeln!(
        file,
        "gene_id,chromosome,start_position,end_position,expression_level,\
         read_count,mapping_quality,duplicate_rate,p_value,adj_p_value,log2fc,base_mean"
    )?;

    // Rows (deterministic mock data)
    for i in 0..1000 {
        let chr = format!("chr{}", (i % 23) + 1);
        let start_pos = i * 1000 + 1_000_000;
        let end_pos = start_pos + 500;
        let expr = (i as f64) / 100.0;
        let reads = 100.0 + (i as f64);
        let mapq = 20.0 + (i % 20) as f64;
        let dup_rate = (i % 100) as f64 / 1000.0;
        let pval = (1000 - i) as f64 / 10000.0;
        let adj_pval = pval * 1.5;
        let log2fc = (i as f64 - 500.0) / 100.0;
        let base_mean = i as f64 / 10.0;

        writeln!(
            file,
            "GENE_{},{},{},{},{:.2},{:.0},{:.1},{:.3},{:.4},{:.4},{:.2},{:.1}",
            i, chr, start_pos, end_pos, expr, reads, mapq, dup_rate, pval, adj_pval, log2fc,
            base_mean
        )?;
    }

    Ok(())
}

/* -------------------------------- Examples -------------------------------- */

/// Example 1: Basic gene expression filtering
fn expression_analysis() -> DynResult<usize> {
    let mut filter = build_filter(DATA, "high_expression.csv", None)?;

    add_filters(
        &mut filter,
        [ColumnFilter::new(
            "expression_level".into(),
            FilterCondition::Numeric(NumericCondition::GreaterThan(5.0)),
        )?],
    );

    let stats = filter.process()?;
    Ok(stats.rows_matched)
}

/// Example 2: Multiple QC filters (AND-composed)
fn quality_control_filtering() -> DynResult<usize> {
    let mut filter = build_filter(DATA, "qc_passed.csv", None)?;

    add_filters(
        &mut filter,
        [
            ColumnFilter::new(
                "read_count".into(),
                FilterCondition::Numeric(NumericCondition::GreaterThan(100.0)),
            )?,
            ColumnFilter::new(
                "mapping_quality".into(),
                FilterCondition::Numeric(NumericCondition::GreaterThan(30.0)),
            )?,
            ColumnFilter::new(
                "duplicate_rate".into(),
                FilterCondition::Numeric(NumericCondition::LessThan(0.10)),
            )?,
        ],
    );

    let stats = filter.process()?;
    Ok(stats.rows_matched)
}

/// Example 3: Chromosome-specific queries using an index
fn chromosome_analysis() -> DynResult<usize> {
    // Build and persist an index (once) for faster lookups on "chromosome"
    if !PathBuf::from(IDX_PATH).exists() {
        let index = FileIndex::builder(DATA, "chromosome").build()?;
        index.save(IDX_PATH)?;
    }

    let mut filter = build_filter(DATA, "chr1_genes.csv", Some(IDX_PATH))?;

    add_filters(
        &mut filter,
        [
            ColumnFilter::new(
                "chromosome".into(),
                FilterCondition::Equals("chr1".into()),
            )?,
            ColumnFilter::new(
                "start_position".into(),
                FilterCondition::Numeric(NumericCondition::GreaterThan(1_000_000.0)),
            )?,
        ],
    );

    let stats = filter.process()?;
    Ok(stats.rows_matched)
}

/// Example 4: Statistical significance filtering
fn pvalue_filtering() -> DynResult<usize> {
    let mut filter = build_filter(DATA, "significant_genes.csv", None)?;

    // Note: This selects rows with p_value < 0.05 and fold_change in (-2, 2).
    // Adjust the range or add additional passes if you want |log2fc| ≥ 1 instead.
    add_filters(
        &mut filter,
        [
            ColumnFilter::new(
                "p_value".into(),
                FilterCondition::Numeric(NumericCondition::LessThan(0.05)),
            )?,
            ColumnFilter::new(
                "fold_change".into(),
                FilterCondition::Range(RangeCondition {
                    min: -2.0,
                    max: 2.0,
                    inclusive: false,
                }),
            )?,
        ],
    );

    let stats = filter.process()?;
    Ok(stats.rows_matched)
}

/// Example 5: Complex DEG analysis (typical thresholds)
fn deg_analysis() -> DynResult<usize> {
    let mut filter = build_filter(DATA, "significant_degs.csv", None)?;

    add_filters(
        &mut filter,
        [
            ColumnFilter::new(
                "p_value".into(),
                FilterCondition::Numeric(NumericCondition::LessThan(0.05)),
            )?,
            ColumnFilter::new(
                "adj_p_value".into(),
                FilterCondition::Numeric(NumericCondition::LessThan(0.10)),
            )?,
            ColumnFilter::new(
                "log2fc".into(),
                FilterCondition::Range(RangeCondition {
                    min: 1.0,
                    max: f64::INFINITY,
                    inclusive: true,
                }),
            )?,
            ColumnFilter::new(
                "base_mean".into(),
                FilterCondition::Numeric(NumericCondition::GreaterThan(10.0)),
            )?,
        ],
    );

    let stats = filter.process()?;
    Ok(stats.rows_matched)
}
