# Performance Tuning Guide

## Understanding Performance Characteristics

### Memory Usage vs Speed

Extractor provides several configuration options that affect the memory-speed trade-off:

1. Chunk Size
   - Larger chunks = faster processing but more memory
   - Smaller chunks = less memory but more I/O overhead

2. Memory Mapping
   - Pros: Fast random access, zero-copy reading
   - Cons: Virtual memory overhead

3. Indexing
   - Pros: Constant-time lookups
   - Cons: Index storage overhead

## Optimizing for Different Scenarios

### Large Files (>1GB)

```rust
let config = Config {
    chunk_size: 16 * 1024 * 1024,  // 16MB chunks
    parallel: true,
    num_threads: Some(num_cpus::get()),
    use_index: true,
};
```

### Memory-Constrained Systems

```rust
let config = Config {
    chunk_size: 512 * 1024,  // 512KB chunks
    parallel: false,  // Avoid multiple buffers
    use_index: false,
};
```

### Maximum Performance

```rust
let config = Config {
    chunk_size: 32 * 1024 * 1024,  // 32MB chunks
    parallel: true,
    num_threads: Some(num_cpus::get() * 2),
    use_index: true,
};
```

## Performance Monitoring

Example monitoring code:

```rust
use std::time::Instant;

let start = Instant::now();
let stats = filter.process()?;

println!("Processing stats:");
println!("Time taken: {:?}", start.elapsed());
println!("Rows processed: {}", stats.rows_processed);
println!("Memory used: {} MB", stats.memory_usage_mb);
println!("Throughput: {:.2} rows/sec", 
    stats.rows_processed as f64 / start.elapsed().as_secs_f64());
```

## Benchmarking Your Use Case

Use the built-in benchmarking tools:

```rust
cargo bench --bench performance_benchmarks -- --verbose
```

## Common Performance Pitfalls

1. Over-indexing
   - Only index columns you frequently query
   - Consider index size vs. query speed

2. Thread Contention
   - Too many threads can cause overhead
   - Monitor CPU and I/O usage

3. Memory Pressure
   - Watch for system swap usage
   - Adjust chunk size accordingly