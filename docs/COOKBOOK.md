# Extractor Cookbook

## Common Bioinformatics Tasks

### RNA-seq Analysis

1. Filter Differentially Expressed Genes
```rust
let mut filter = BioFilter::builder("deseq_results.csv", "significant_genes.csv")
    .build()?;

// Add statistical filters
filter.add_filter(Box::new(ColumnFilter::new(
    "padj",
    FilterCondition::Numeric(NumericCondition::LessThan(0.05))
)?));

// Add fold change filter
filter.add_filter(Box::new(ColumnFilter::new(
    "log2FoldChange",
    FilterCondition::Range(RangeCondition {
        min: 1.0,
        max: f64::INFINITY,
        inclusive: true,
    })
)?));
```

### ChIP-seq Analysis

1. Filter Peak Calls
```rust
let mut filter = BioFilter::builder("macs2_peaks.csv", "filtered_peaks.csv")
    .build()?;

// Filter by q-value
filter.add_filter(Box::new(ColumnFilter::new(
    "q_value",
    FilterCondition::Numeric(NumericCondition::LessThan(0.01))
)?));

// Filter by fold enrichment
filter.add_filter(Box::new(ColumnFilter::new(
    "fold_enrichment",
    FilterCondition::Numeric(NumericCondition::GreaterThan(4.0))
)?));
```

### Variant Analysis

1. Filter VCF Data
```rust
let mut filter = BioFilter::builder("variants.csv", "filtered_variants.csv")
    .build()?;

// Quality filters
filter.add_filter(Box::new(ColumnFilter::new(
    "QUAL",
    FilterCondition::Numeric(NumericCondition::GreaterThan(30.0))
)?));

// Depth filter
filter.add_filter(Box::new(ColumnFilter::new(
    "DP",
    FilterCondition::Numeric(NumericCondition::GreaterThan(10.0))
)?));
```

## Advanced Use Cases

### Pipeline Integration

1. Processing Multiple Files
```rust
use rayon::prelude::*;

let files = vec!["sample1.csv", "sample2.csv", "sample3.csv"];
files.par_iter().try_for_each(|file| {
    let mut filter = BioFilter::builder(file, format!("{}.filtered", file))
        .build()?;
    // Add filters
    filter.process()
})?;
```

2. Custom Output Formats
```rust
let mut filter = BioFilter::builder("input.csv", "output.bed")
    .with_config(Config {
        output_format: OutputFormat::BED,
        ..Config::default()
    })
    .build()?;
```

### Complex Filtering

1. Combined Conditions
```rust
// Find genes that are both highly expressed and have low p-values
let mut filter = BioFilter::builder("expression.csv", "candidates.csv")
    .build()?;

filter.add_filter(Box::new(CombinedFilter::new(vec![
    Box::new(ColumnFilter::new(
        "expression",
        FilterCondition::Numeric(NumericCondition::GreaterThan(100.0))
    )?),
    Box::new(ColumnFilter::new(
        "pvalue",
        FilterCondition::Numeric(NumericCondition::LessThan(0.01))
    )?),
])));
```

### Pattern Matching

1. Regular Expressions
```rust
// Find genes matching a pattern
filter.add_filter(Box::new(ColumnFilter::new(
    "gene_name",
    FilterCondition::Regex("HOX[A-D]\\d+".to_string())
)?));
```

## Optimization Recipes

### Memory Optimization

1. Streaming Large Files
```rust
let config = Config {
    chunk_size: 1024 * 1024,  // 1MB chunks
    parallel: false,
    buffer_output: false,
    ..Config::default()
};
```

2. Parallel Processing with Memory Limits
```rust
let config = Config {
    parallel: true,
    num_threads: Some(4),
    max_memory_mb: Some(1024),  // 1GB max
    ..Config::default()
};
```