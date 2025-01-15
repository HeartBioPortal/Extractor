# Extractor API Guide

## Table of Contents
1. [Overview](#overview)
2. [Installation](#installation)
3. [Core Concepts](#core-concepts)
4. [Common Use Cases](#common-use-cases)
5. [Performance Tuning](#performance-tuning)
6. [Migration Guide](#migration-guide)

## Overview

Extractor is a high-performance Rust library designed for processing large biological datasets. It provides:
- Fast CSV filtering with indexed access
- Memory-efficient processing
- Parallel execution support
- Comprehensive bioinformatics format support

## Core Concepts

### The BioFilter Builder

```rust
use extractor::{BioFilter, Config};

// Basic usage
let filter = BioFilter::builder("input.csv", "output.csv")
    .build()?;

// With custom configuration
let filter = BioFilter::builder("input.csv", "output.csv")
    .with_config(Config {
        parallel: true,
        chunk_size: 1024 * 1024,  // 1MB chunks
        ..Config::default()
    })
    .build()?;
```

### Filter Conditions

```rust
use extractor::{FilterCondition, NumericCondition};

// Numeric filters
filter.add_filter(Box::new(ColumnFilter::new(
    "expression_level",
    FilterCondition::Numeric(NumericCondition::GreaterThan(5.0))
)?));

// Pattern matching
filter.add_filter(Box::new(ColumnFilter::new(
    "gene_name",
    FilterCondition::Regex("BRCA[12]".to_string())
)?));
```

### Using Indices

```rust
// Create an index
let index = FileIndex::builder("data.csv", "gene_id")
    .add_secondary_index("chromosome")
    .build()?;
index.save("data.index")?;

// Use the index
let filter = BioFilter::builder("data.csv", "output.csv")
    .with_index("data.index")
    .build()?;
```

## Common Use Cases

### 1. RNA-seq Data Processing

```rust
// Filter for significantly expressed genes
let mut filter = BioFilter::builder("expression.csv", "significant.csv")
    .build()?;

filter.add_filter(Box::new(ColumnFilter::new(
    "padj",  // Adjusted p-value
    FilterCondition::Numeric(NumericCondition::LessThan(0.05))
)?));

filter.add_filter(Box::new(ColumnFilter::new(
    "log2FoldChange",
    FilterCondition::Range(RangeCondition {
        min: -2.0,
        max: 2.0,
        inclusive: false,
    })
)?));
```

### 2. ChIP-seq Peak Analysis

```rust
// Filter peaks by signal strength and width
let mut filter = BioFilter::builder("peaks.csv", "strong_peaks.csv")
    .build()?;

filter.add_filter(Box::new(ColumnFilter::new(
    "signal_value",
    FilterCondition::Numeric(NumericCondition::GreaterThan(10.0))
)?));

filter.add_filter(Box::new(ColumnFilter::new(
    "peak_width",
    FilterCondition::Range(RangeCondition {
        min: 100.0,
        max: 1000.0,
        inclusive: true,
    })
)?));
```

## Performance Tuning

### Memory Usage

The chunk size parameter controls the trade-off between memory usage and processing speed:

```rust
let config = Config {
    chunk_size: 1024 * 1024,  // Default: 1MB
    parallel: true,
    num_threads: Some(4),
    ..Config::default()
};
```

Guidelines for chunk size:
- Small files (<100MB): 1MB chunks
- Medium files (100MB-1GB): 4MB chunks
- Large files (>1GB): 16MB chunks

### Parallel Processing

Optimize thread count based on your system:

```rust
let config = Config {
    parallel: true,
    num_threads: Some(num_cpus::get()),  // Use all available cores
    ..Config::default()
};
```
