# Extractor

A high-performance Rust tool for filtering and extracting genetic data from large CSV/TSV files.
## Features

- Fast processing of large CSV and TSV files
- Support for both GWAS and trait data formats
- Flexible filtering based on gene names and CVD/trait identifiers
- Two operation modes:
  - Standard gene-based extraction
  - Shared genetic architecture (SGA) analysis
- Configurable input/output paths and delimiters
- Comprehensive error handling and logging

## Installation

1. Ensure you have Rust installed on your system. If not, install it from [rustup.rs](https://rustup.rs/).

2. Clone the repository:
```bash
git clone https://github.com/HeartBioPortal/Extractor.git
cd extractor
```

3. Build the project:
```bash
cargo build --release
```

The compiled binary will be available at `target/release/extractor`.

## Configuration

The tool can be configured through a TOML file or environment variables.

### Configuration File

Create a `config/default.toml` file:

```toml
[paths]
gwas = "/path/to/gwas/data"
trait = "/path/to/trait/data"
output = "/path/to/output"

[files]
gwas_output = "gwas_consortium_efforts.csv"
trait_output = "traits.csv"
sga_output = "shared_genetic_architecture.csv"

[processing]
gwas_delimiter = ","
trait_delimiter = "\t"
```

### Environment Variables

Alternatively, you can use environment variables:
```bash
export EXTRACTOR_PATHS__GWAS="/path/to/gwas/data"
export EXTRACTOR_PATHS__TRAIT="/path/to/trait/data"
export EXTRACTOR_PATHS__OUTPUT="/path/to/output"
```

## Usage

The tool supports two modes of operation: gene-based extraction and SGA analysis.

### Gene-based Extraction

This mode filters data based on specific CVD/trait names and a gene identifier:

```bash
./target/release/extractor \
    --sga false \
    --cvd-names '["CVD1", "CVD2"]' \
    --trait-names '["TRAIT1", "TRAIT2"]' \
    --gene "GENE1"
```

### SGA Analysis

This mode performs shared genetic architecture analysis:

```bash
./target/release/extractor \
    --sga true \
    --cvd-names '[]' \
    --trait-names '[]' \
    --gene "GENE1"
```

### Command Line Arguments

- `--sga`: Boolean flag to enable SGA mode
- `--cvd-names`: JSON array of CVD names to filter by
- `--trait-names`: JSON array of trait names to filter by
- `--gene`: Gene identifier to filter by

## Input File Format

### GWAS Files
- Format: CSV (comma-separated)
- Required columns:
  - MarkerID
  - pval
  - Phenotype
  - Study
  - snpeff.ann.gene_id (used for gene filtering)
  - [Other standard GWAS columns]

### Trait Files
- Format: TSV (tab-separated)
- Required columns: Same as GWAS files

## Output Files

The tool generates up to three output files:

1. `gwas_consortium_efforts.csv`: Filtered GWAS data
2. `traits.csv`: Filtered trait data
3. `shared_genetic_architecture.csv`: Results of SGA analysis

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with logging
RUST_LOG=debug cargo test -- --nocapture
```

### Directory Structure

```
.
├── Cargo.toml
├── config/
│   └── default.toml
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── error.rs
│   ├── types.rs
│   └── extractors/
│       ├── mod.rs
│       ├── gene.rs
│       └── sga.rs
└── tests/
    ├── data/
    └── integration_tests.rs
```

## Error Handling

The tool includes comprehensive error handling for:
- File I/O operations
- CSV/TSV parsing
- Configuration loading
- Command-line argument parsing
- JSON parsing

Error messages are logged to stderr with appropriate context.

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Original code by [Kasra Vand]
- Contributors: []

## Support

For support, please open an issue on the GitHub repository or contact heartbioportal@gmail.com.
