//! TOON Format Benchmarks
//!
//! Benchmarks comparing TOON vs JSON performance:
//! - Serialization speed
//! - Deserialization speed  
//! - Output size
//! - Token count estimation
//!
//! Run with: cargo bench --package toon-bench

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    role: String,
    active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UsersResponse {
    users: Vec<User>,
    total: usize,
    page: usize,
}

fn create_users(count: usize) -> Vec<User> {
    (1..=count)
        .map(|i| User {
            id: i as u64,
            name: format!("User{}", i),
            role: if i % 3 == 0 {
                "admin".into()
            } else {
                "user".into()
            },
            active: i % 2 == 0,
        })
        .collect()
}

fn create_response(user_count: usize) -> UsersResponse {
    let users = create_users(user_count);
    UsersResponse {
        total: users.len(),
        users,
        page: 1,
    }
}

fn benchmark_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    for size in [10, 50, 100, 500, 1000].iter() {
        let response = create_response(*size);

        group.bench_with_input(BenchmarkId::new("json", size), size, |b, _| {
            b.iter(|| {
                let _ = black_box(serde_json::to_string(&response));
            });
        });

        group.bench_with_input(BenchmarkId::new("toon", size), size, |b, _| {
            b.iter(|| {
                let _ = black_box(toon_format::encode_default(&response));
            });
        });
    }

    group.finish();
}

fn benchmark_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialization");

    for size in [10, 50, 100].iter() {
        let response = create_response(*size);
        let json_str = serde_json::to_string(&response).unwrap();

        group.bench_with_input(BenchmarkId::new("json", size), &json_str, |b, json| {
            b.iter(|| {
                let _: UsersResponse = black_box(serde_json::from_str(json).unwrap());
            });
        });
    }

    group.finish();
}

fn benchmark_output_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("output_size");

    for size in [10, 50, 100, 500, 1000].iter() {
        let response = create_response(*size);
        let json_str = serde_json::to_string(&response).unwrap();
        let toon_str = toon_format::encode_default(&response).unwrap();

        // Just measure sizes (not really a benchmark, more like a comparison)
        println!("\n=== {} users ===", size);
        println!("JSON bytes: {}", json_str.len());
        println!("TOON bytes: {}", toon_str.len());
        println!(
            "Byte savings: {:.2}%",
            (1.0 - (toon_str.len() as f64 / json_str.len() as f64)) * 100.0
        );

        // Estimate tokens (~4 chars per token)
        let json_tokens = json_str.len().div_ceil(4);
        let toon_tokens = toon_str.len().div_ceil(4);
        println!("JSON tokens (est): {}", json_tokens);
        println!("TOON tokens (est): {}", toon_tokens);
        println!(
            "Token savings: {:.2}%",
            (1.0 - (toon_tokens as f64 / json_tokens as f64)) * 100.0
        );

        // Benchmark the size calculation itself (trivial)
        group.bench_with_input(BenchmarkId::new("json_len", size), &json_str, |b, s| {
            b.iter(|| black_box(s.len()));
        });
        group.bench_with_input(BenchmarkId::new("toon_len", size), &toon_str, |b, s| {
            b.iter(|| black_box(s.len()));
        });
    }

    group.finish();
}

fn benchmark_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    for size in [10, 50, 100].iter() {
        let response = create_response(*size);

        group.bench_with_input(BenchmarkId::new("json", size), size, |b, _| {
            b.iter(|| {
                let json = serde_json::to_string(&response).unwrap();
                let _: UsersResponse = serde_json::from_str(&json).unwrap();
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_serialization,
    benchmark_deserialization,
    benchmark_output_size,
    benchmark_roundtrip,
);
criterion_main!(benches);
