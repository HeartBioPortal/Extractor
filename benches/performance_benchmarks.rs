use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use extractor::{BioFilter, Config, FileIndex, FilterCondition, ColumnFilter, NumericCondition};
use std::path::PathBuf;
use std::time::Duration;

criterion_main!(benches);
criterion_group!(
    benches, 
    bench_row_lookup,
    bench_file_sizes,
    bench_parallel_processing,
    bench_memory_usage
);

/// Benchmark row lookup with and without index
fn bench_row_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("row_lookup");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(100);

    // Setup test data
    setup_benchmark_data("lookup_test.csv", 1_000_000).unwrap();
    
    // Create index
    let index = FileIndex::builder("lookup_test.csv", "gene_id")
        .build()
        .unwrap();
    index.save("lookup_test.index").unwrap();

    group.bench_function("without_index", |b| {
        b.iter(|| {
            let mut filter = BioFilter::builder("lookup_test.csv", "output.csv")
                .build()
                .unwrap();
            
            filter.add_filter(Box::new(ColumnFilter::new(
                "gene_id".to_string(),
                FilterCondition::Equals("GENE_500000".to_string())
            ).unwrap()));
            
            black_box(filter.process().unwrap())
        })
    });

    group.bench_function("with_index", |b| {
        b.iter(|| {
            let mut filter = BioFilter::builder("lookup_test.csv", "output.csv")
                .with_index("lookup_test.index")
                .build()
                .unwrap();
            
            filter.add_filter(Box::new(ColumnFilter::new(
                "gene_id".to_string(),
                FilterCondition::Equals("GENE_500000".to_string())
            ).unwrap()));
            
            black_box(filter.process().unwrap())
        })
    });

    group.finish();
}

/// Benchmark different file sizes
fn bench_file_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_sizes");
    group.measurement_time(Duration::from_secs(30));
    
    for size in [100_000, 1_000_000, 10_000_000].iter() {
        let file_name = format!("size_test_{}.csv", size);
        setup_benchmark_data(&file_name, *size).unwrap();
        
        group.bench_with_input(BenchmarkId::new("sequential", size), size, |b, &size| {
            b.iter(|| {
                let mut filter = BioFilter::builder(&file_name, "output.csv")
                    .with_config(Config { parallel: false, ..Config::default() })
                    .build()
                    .unwrap();
                
                filter.add_filter(Box::new(ColumnFilter::new(
                    "expression".to_string(),
                    FilterCondition::Numeric(NumericCondition::GreaterThan(5.0))
                ).unwrap()));
                
                black_box(filter.process().unwrap())
            })
        });
    }
    
    group.finish();
}

/// Benchmark parallel processing
fn bench_parallel_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_processing");
    group.measurement_time(Duration::from_secs(30));
    
    setup_benchmark_data("parallel_test.csv", 5_000_000).unwrap();
    
    for threads in [1, 2, 4, 8, 16].iter() {
        group.bench_with_input(BenchmarkId::new("threads", threads), threads, |b, &threads| {
            b.iter(|| {
                let mut filter = BioFilter::builder("parallel_test.csv", "output.csv")
                    .with_config(Config {
                        parallel: true,
                        num_threads: Some(*threads),
                        ..Config::default()
                    })
                    .build()
                    .unwrap();
                
                filter.add_filter(Box::new(ColumnFilter::new(
                    "expression".to_string(),
                    FilterCondition::Numeric(NumericCondition::GreaterThan(5.0))
                ).unwrap()));
                
                black_box(filter.process().unwrap())
            })
        });
    }
    
    group.finish();
}

/// Benchmark memory usage
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    group.measurement_time(Duration::from_secs(30));
    
    for chunk_size in [1024, 4096, 16384, 65536].iter() {
        setup_benchmark_data("memory_test.csv", 1_000_000).unwrap();
        
        group.bench_with_input(BenchmarkId::new("chunk_size", chunk_size), chunk_size, |b, &chunk_size| {
            b.iter(|| {
                let mut filter = BioFilter::builder("memory_test.csv", "output.csv")
                    .with_config(Config {
                        chunk_size: *chunk_size,
                        ..Config::default()
                    })
                    .build()
                    .unwrap();
                
                filter.add_filter(Box::new(ColumnFilter::new(
                    "expression".to_string(),
                    FilterCondition::Numeric(NumericCondition::GreaterThan(5.0))
                ).unwrap()));
                
                black_box(filter.process().unwrap())
            })
        });
    }
    
    group.finish();
}

/// Helper function to create benchmark data
fn setup_benchmark_data(filename: &str, rows: usize) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(filename)?;
    
    // Write header
    writeln!(file, "gene_id,gene_name,chromosome,expression,p_value")?;
    
    // Generate test data
    for i in 0..rows {
        let gene_id = format!("GENE_{}", i);
        let gene_name = format!("Name_{}", i);
        let chr = format!("chr{}", (i % 23) + 1);
        let expression = (i as f64 % 100.0) + 0.1;
        let p_value = (i as f64 + 1.0).recip();
        
        writeln!(file, "{},{},{},{:.2},{:.4}", 
            gene_id, gene_name, chr, expression, p_value)?;
    }
    
    Ok(())
}