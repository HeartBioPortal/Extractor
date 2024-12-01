# Extractor

A high-performance Rust library for filtering and processing large CSV files, with support for both streaming and indexed processing modes.

## Features

- 🚀 High-performance parallel processing of CSV files
- 📑 Memory-mapped file handling for efficient I/O
- 🔍 Advanced filtering system with multiple condition types
- 📖 Optional indexed access mode for rapid filtering
- 💻 Multi-threaded processing support
- 🎯 Zero-copy parsing where possible
- 📊 Progress tracking and statistics
- 🛡️ Comprehensive error handling

## Performance

- Processes millions of rows per second on modern hardware
- Memory-efficient streaming mode for large files
- Optional indexing for repeated queries
- Parallel processing with configurable thread count

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
extractor = "0.1.0"
```

## Quick Start

```rust
use extractor::{BioFilter, Config, FilterCondition};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a filter with default configuration
    let mut filter = BioFilter::builder("input.csv", "output.csv")
        .with_config(Config::default())
        .build()?;

    // Add filters
    filter.add_filter(Box::new(ColumnFilter::new(
        "gene_expression",
        FilterCondition::Numeric(NumericCondition::GreaterThan(0.5))
    )?));

    // Process the file
    let stats = filter.process()?;
    println!("Processed {} rows, matched {}", stats.rows_processed, stats.rows_matched);

    Ok(())
}
```

## Advanced Usage

### Indexed Mode

For repeated queries on the same file, use indexed mode for better performance:

```rust
// Build an index
let index = FileIndex::builder("input.csv", "gene_id")
    .add_secondary_index("chromosome")
    .build()?;
index.save("data.index")?;

// Use the index
let mut filter = BioFilter::builder("input.csv", "output.csv")
    .with_index("data.index")
    .build()?;
```

### Custom Filters

Implement the `Filter` trait for custom filtering logic:

```rust
struct CustomFilter {
    column: String,
}

impl Filter for CustomFilter {
    fn apply(&self, row: &[u8], headers: &HashMap<String, usize>) -> Result<bool> {
        // Custom filtering logic here
    }

    fn column_name(&self) -> &str {
        &self.column
    }

    fn description(&self) -> String {
        format!("Custom filter on {}", self.column)
    }
}
```

### Available Filter Conditions

- Exact match (`Equals`)
- Substring (`Contains`)
- Regular expression (`Regex`)
- Numeric comparisons (`GreaterThan`, `LessThan`, `Equal`)
- Range checks (`Between`)
- Multiple values (`OneOf`)
- Null checks (`Empty`, `NotEmpty`)

### Configuration Options

```rust
let config = Config {
    delimiter: b',',
    has_headers: true,
    chunk_size: 1024 * 1024,  // 1MB chunks
    parallel: true,
    use_index: false,
    num_threads: Some(4),
    progress: ProgressConfig::default(),
};
```

## Performance Tips

1. Use indexed mode for repeated queries on the same file
2. Adjust chunk size based on your system's memory
3. Enable parallel processing for multi-core systems
4. Use memory mapping for large files
5. Consider pre-filtering columns when building indices

## Error Handling

The library provides detailed error types for different failure scenarios:

```rust
match result {
    Err(ExtractorError::Io { source, path }) => // Handle I/O errors
    Err(ExtractorError::Csv(e)) => // Handle CSV parsing errors
    Err(ExtractorError::Index { kind, path }) => // Handle index-related errors
    // etc.
}
```

## Contributing

Contributions are welcome! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

This project is licensed under the GNU Affero General Public License - see the [LICENSE](LICENSE) file for details.