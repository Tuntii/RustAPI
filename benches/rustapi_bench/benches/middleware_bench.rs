//! Middleware composition benchmarks
//!
//! Benchmarks the overhead of middleware layers in RustAPI.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// Simulate middleware overhead with simple counter
fn simulate_middleware_layer(input: u64, layers: usize) -> u64 {
    let mut result = input;
    for _ in 0..layers {
        // Simulate minimal middleware work: check + transform
        if result > 0 {
            result = result.wrapping_add(1);
        }
    }
    result
}

/// Simulate request ID generation (UUID-like)
fn simulate_request_id_middleware(request_count: u64) -> String {
    format!("req_{:016x}", request_count)
}

/// Simulate header parsing overhead
fn simulate_header_parsing(headers: &[(&str, &str)]) -> usize {
    headers.iter().map(|(k, v)| k.len() + v.len()).sum()
}

/// Benchmark middleware layer composition
fn bench_middleware_layers(c: &mut Criterion) {
    let mut group = c.benchmark_group("middleware_layers");

    // Test with different numbers of middleware layers
    for layer_count in [0, 1, 3, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("layer_count", layer_count),
            layer_count,
            |b, &layers| b.iter(|| simulate_middleware_layer(black_box(42), layers)),
        );
    }

    group.finish();
}

/// Benchmark request ID generation
fn bench_request_id(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_id");

    group.bench_function("generate", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            counter += 1;
            simulate_request_id_middleware(black_box(counter))
        })
    });

    group.finish();
}

/// Benchmark header parsing
fn bench_header_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("header_parsing");

    // Minimal headers
    let minimal_headers = [("content-type", "application/json")];

    // Typical API headers
    let typical_headers = [
        ("content-type", "application/json"),
        ("accept", "application/json"),
        (
            "authorization",
            "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
        ),
        ("x-request-id", "550e8400-e29b-41d4-a716-446655440000"),
        ("user-agent", "RustAPI-Client/1.0"),
    ];

    // Many headers
    let many_headers: Vec<(&str, &str)> = (0..20)
        .map(|i| {
            let key: &'static str = Box::leak(format!("x-custom-header-{}", i).into_boxed_str());
            let value: &'static str = Box::leak(format!("value-{}", i).into_boxed_str());
            (key, value)
        })
        .collect();

    group.bench_function("minimal_headers", |b| {
        b.iter(|| simulate_header_parsing(black_box(&minimal_headers)))
    });

    group.bench_function("typical_headers", |b| {
        b.iter(|| simulate_header_parsing(black_box(&typical_headers)))
    });

    group.bench_function("many_headers", |b| {
        b.iter(|| simulate_header_parsing(black_box(&many_headers)))
    });

    group.finish();
}

/// Benchmark async middleware simulation
fn bench_middleware_chain(c: &mut Criterion) {
    let mut group = c.benchmark_group("middleware_chain");

    // Simulate a typical middleware chain:
    // 1. Request ID
    // 2. Tracing
    // 3. Auth check
    // 4. Rate limit check
    // 5. Body limit check

    group.bench_function("typical_chain", |b| {
        b.iter(|| {
            // Step 1: Generate request ID
            let request_id = simulate_request_id_middleware(black_box(12345));

            // Step 2: Tracing (record span)
            let _ = black_box(request_id.len());

            // Step 3: Auth check (simple token validation)
            let token = "Bearer valid_token";
            let is_valid = black_box(token.starts_with("Bearer "));

            // Step 4: Rate limit check (counter check)
            let rate_count = black_box(99u64);
            let under_limit = rate_count < 100;

            // Step 5: Body limit check
            let body_size = black_box(1024usize);
            let within_limit = body_size < 1_048_576; // 1MB

            (is_valid, under_limit, within_limit)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_middleware_layers,
    bench_request_id,
    bench_header_parsing,
    bench_middleware_chain,
);

criterion_main!(benches);
