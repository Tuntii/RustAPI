//! Extractor overhead benchmarks
//!
//! Benchmarks the performance of different extractor types in RustAPI.

#![allow(dead_code)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Simple query params struct
#[derive(Deserialize)]
struct SimpleQuery {
    page: Option<u32>,
    limit: Option<u32>,
}

/// Complex query params struct
#[derive(Deserialize)]
struct ComplexQuery {
    page: Option<u32>,
    limit: Option<u32>,
    sort: Option<String>,
    filter: Option<String>,
    include: Option<Vec<String>>,
}

/// User request body
#[derive(Serialize, Deserialize)]
struct UserBody {
    name: String,
    email: String,
    age: u32,
}

/// Complex request body
#[derive(Serialize, Deserialize)]
struct ComplexBody {
    user: UserBody,
    tags: Vec<String>,
    metadata: HashMap<String, String>,
}

/// Benchmark path parameter extraction
fn bench_path_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_extraction");

    // Single path param
    group.bench_function("single_param", |b| {
        let path = "/users/12345";
        b.iter(|| {
            let id: u64 = black_box(path)
                .strip_prefix("/users/")
                .unwrap()
                .parse()
                .unwrap();
            id
        })
    });

    // Multiple path params
    group.bench_function("multiple_params", |b| {
        let path = "/users/12345/posts/67890";
        b.iter(|| {
            let parts: Vec<&str> = black_box(path).split('/').collect();
            let user_id: u64 = parts[2].parse().unwrap();
            let post_id: u64 = parts[4].parse().unwrap();
            (user_id, post_id)
        })
    });

    // UUID path param
    group.bench_function("uuid_param", |b| {
        let path = "/items/550e8400-e29b-41d4-a716-446655440000";
        b.iter(|| {
            let uuid_str = black_box(path).strip_prefix("/items/").unwrap();
            // Just validate format, don't parse to actual UUID
            uuid_str.len() == 36 && uuid_str.chars().filter(|c| *c == '-').count() == 4
        })
    });

    group.finish();
}

/// Benchmark query string extraction
fn bench_query_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_extraction");

    // Simple query
    let simple_query = "page=1&limit=10";
    group.bench_function("simple_query", |b| {
        b.iter(|| serde_urlencoded::from_str::<SimpleQuery>(black_box(simple_query)).unwrap())
    });

    // Complex query
    let complex_query =
        "page=1&limit=10&sort=created_at&filter=active&include=posts&include=comments";
    group.bench_function("complex_query", |b| {
        b.iter(|| serde_urlencoded::from_str::<ComplexQuery>(black_box(complex_query)).unwrap())
    });

    // Empty query
    let empty_query = "";
    group.bench_function("empty_query", |b| {
        b.iter(|| serde_urlencoded::from_str::<SimpleQuery>(black_box(empty_query)).unwrap())
    });

    group.finish();
}

/// Benchmark JSON body extraction
fn bench_json_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_extraction");

    // Simple body
    let simple_json = r#"{"name":"John Doe","email":"john@example.com","age":30}"#;
    group.bench_function("simple_body", |b| {
        b.iter(|| serde_json::from_str::<UserBody>(black_box(simple_json)).unwrap())
    });

    // Complex body
    let complex_json = r#"{
        "user": {"name":"John Doe","email":"john@example.com","age":30},
        "tags": ["rust", "api", "web"],
        "metadata": {"source": "mobile", "version": "1.0"}
    }"#;
    group.bench_function("complex_body", |b| {
        b.iter(|| serde_json::from_str::<ComplexBody>(black_box(complex_json)).unwrap())
    });

    // Large array body
    let users: Vec<UserBody> = (0..100)
        .map(|i| UserBody {
            name: format!("User {}", i),
            email: format!("user{}@example.com", i),
            age: 20 + (i as u32 % 50),
        })
        .collect();
    let large_json = serde_json::to_string(&users).unwrap();

    group.bench_function("large_array_body", |b| {
        b.iter(|| serde_json::from_str::<Vec<UserBody>>(black_box(&large_json)).unwrap())
    });

    group.finish();
}

/// Benchmark header extraction
fn bench_header_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("header_extraction");

    // Content-Type extraction
    group.bench_function("content_type", |b| {
        let header = "application/json; charset=utf-8";
        b.iter(|| {
            let content_type = black_box(header).split(';').next().unwrap().trim();
            content_type == "application/json"
        })
    });

    // Authorization extraction
    group.bench_function("authorization", |b| {
        let header = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ";
        b.iter(|| {
            let token = black_box(header).strip_prefix("Bearer ").unwrap();
            token.len() > 0
        })
    });

    // Accept header parsing
    group.bench_function("accept_parsing", |b| {
        let header = "application/json, application/xml;q=0.9, text/html;q=0.8, */*;q=0.1";
        b.iter(|| {
            let types: Vec<&str> = black_box(header)
                .split(',')
                .map(|s| s.split(';').next().unwrap().trim())
                .collect();
            types
        })
    });

    group.finish();
}

/// Benchmark combined extraction (typical request)
fn bench_combined_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("combined_extraction");

    // Typical GET request
    group.bench_function("typical_get", |b| {
        let path = "/users/12345";
        let query = "page=1&limit=10";
        let auth = "Bearer token123";

        b.iter(|| {
            // Extract path param
            let user_id: u64 = black_box(path)
                .strip_prefix("/users/")
                .unwrap()
                .parse()
                .unwrap();

            // Extract query params
            let query_params = serde_urlencoded::from_str::<SimpleQuery>(black_box(query)).unwrap();

            // Extract auth token
            let token = black_box(auth).strip_prefix("Bearer ").unwrap();

            (user_id, query_params.page, token.len())
        })
    });

    // Typical POST request
    group.bench_function("typical_post", |b| {
        let _path = "/users";
        let body = r#"{"name":"John Doe","email":"john@example.com","age":30}"#;
        let content_type = "application/json";
        let auth = "Bearer token123";

        b.iter(|| {
            // Verify content type
            let is_json = black_box(content_type) == "application/json";

            // Extract auth token
            let token = black_box(auth).strip_prefix("Bearer ").unwrap();

            // Parse body
            let user = serde_json::from_str::<UserBody>(black_box(body)).unwrap();

            (is_json, token.len(), user.name.len())
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_path_extraction,
    bench_query_extraction,
    bench_json_extraction,
    bench_header_extraction,
    bench_combined_extraction,
);

criterion_main!(benches);
