//! JSON serialization/deserialization benchmarks
//!
//! Benchmarks serde_json performance which is critical for API frameworks.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde::{Deserialize, Serialize};

/// Simple response structure
#[derive(Serialize, Deserialize)]
struct SimpleResponse {
    message: String,
    status: u16,
}

/// User response with more fields
#[derive(Serialize, Deserialize)]
struct UserResponse {
    id: i64,
    name: String,
    email: String,
    created_at: String,
    is_active: bool,
}

/// Complex response with nested data
#[derive(Serialize, Deserialize)]
struct ComplexResponse {
    users: Vec<UserResponse>,
    total: usize,
    page: usize,
    per_page: usize,
    has_more: bool,
}

/// Create test data
fn create_simple() -> SimpleResponse {
    SimpleResponse {
        message: "Hello, World!".to_string(),
        status: 200,
    }
}

fn create_user(id: i64) -> UserResponse {
    UserResponse {
        id,
        name: format!("User {}", id),
        email: format!("user{}@example.com", id),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        is_active: true,
    }
}

fn create_complex(count: usize) -> ComplexResponse {
    ComplexResponse {
        users: (0..count as i64).map(create_user).collect(),
        total: count * 10,
        page: 1,
        per_page: count,
        has_more: true,
    }
}

/// Benchmark JSON serialization
fn bench_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_serialize");

    let simple = create_simple();
    let user = create_user(1);
    let complex_10 = create_complex(10);
    let complex_100 = create_complex(100);

    group.bench_function("simple", |b| {
        b.iter(|| serde_json::to_string(black_box(&simple)))
    });

    group.bench_function("user", |b| {
        b.iter(|| serde_json::to_string(black_box(&user)))
    });

    group.bench_function("complex_10_users", |b| {
        b.iter(|| serde_json::to_string(black_box(&complex_10)))
    });

    group.bench_function("complex_100_users", |b| {
        b.iter(|| serde_json::to_string(black_box(&complex_100)))
    });

    group.finish();
}

/// Benchmark JSON serialization to bytes (more realistic for HTTP)
fn bench_serialize_to_vec(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_serialize_vec");

    let simple = create_simple();
    let user = create_user(1);
    let complex_10 = create_complex(10);

    group.bench_function("simple", |b| {
        b.iter(|| serde_json::to_vec(black_box(&simple)))
    });

    group.bench_function("user", |b| b.iter(|| serde_json::to_vec(black_box(&user))));

    group.bench_function("complex_10_users", |b| {
        b.iter(|| serde_json::to_vec(black_box(&complex_10)))
    });

    group.finish();
}

/// Benchmark JSON deserialization
fn bench_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_deserialize");

    let simple_json = serde_json::to_string(&create_simple()).unwrap();
    let user_json = serde_json::to_string(&create_user(1)).unwrap();
    let complex_10_json = serde_json::to_string(&create_complex(10)).unwrap();
    let complex_100_json = serde_json::to_string(&create_complex(100)).unwrap();

    group.bench_function("simple", |b| {
        b.iter(|| serde_json::from_str::<SimpleResponse>(black_box(&simple_json)))
    });

    group.bench_function("user", |b| {
        b.iter(|| serde_json::from_str::<UserResponse>(black_box(&user_json)))
    });

    group.bench_function("complex_10_users", |b| {
        b.iter(|| serde_json::from_str::<ComplexResponse>(black_box(&complex_10_json)))
    });

    group.bench_function("complex_100_users", |b| {
        b.iter(|| serde_json::from_str::<ComplexResponse>(black_box(&complex_100_json)))
    });

    group.finish();
}

/// Benchmark request body parsing (typical API scenario)
fn bench_request_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_body_parsing");

    // Simulate incoming request bodies
    let create_user_body = r#"{"name": "John Doe", "email": "john@example.com"}"#;
    let create_post_body = r#"{"title": "Hello World", "content": "This is a blog post with some content that is reasonably long to simulate real world usage.", "author_id": 123}"#;
    let bulk_import_body = serde_json::to_string(
        &(0..50)
            .map(|i| {
                serde_json::json!({
                    "name": format!("User {}", i),
                    "email": format!("user{}@example.com", i)
                })
            })
            .collect::<Vec<_>>(),
    )
    .unwrap();

    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct CreateUser {
        name: String,
        email: String,
    }

    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct CreatePost {
        title: String,
        content: String,
        author_id: i64,
    }

    group.bench_function("create_user", |b| {
        b.iter(|| serde_json::from_str::<CreateUser>(black_box(create_user_body)))
    });

    group.bench_function("create_post", |b| {
        b.iter(|| serde_json::from_str::<CreatePost>(black_box(create_post_body)))
    });

    group.bench_function("bulk_import_50", |b| {
        b.iter(|| serde_json::from_str::<Vec<CreateUser>>(black_box(&bulk_import_body)))
    });

    group.finish();
}

/// Benchmark scaling with response size
fn bench_response_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("response_scaling");

    for user_count in [1, 10, 50, 100, 500].iter() {
        let response = create_complex(*user_count);

        group.bench_with_input(
            BenchmarkId::new("serialize", user_count),
            user_count,
            |b, _| b.iter(|| serde_json::to_vec(black_box(&response))),
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_serialize,
    bench_serialize_to_vec,
    bench_deserialize,
    bench_request_parsing,
    bench_response_scaling,
);

criterion_main!(benches);
