//! Routing micro-benchmarks using Criterion
//!
//! Benchmarks the core routing performance of RustAPI's matchit-based router.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use matchit::Router;

/// Benchmark static route matching
fn bench_static_routes(c: &mut Criterion) {
    let mut router = Router::new();

    // Add static routes
    router.insert("/", "root").unwrap();
    router.insert("/health", "health").unwrap();
    router.insert("/api/v1/users", "users").unwrap();
    router.insert("/api/v1/posts", "posts").unwrap();
    router.insert("/api/v1/comments", "comments").unwrap();
    router.insert("/api/v2/users", "users_v2").unwrap();
    router.insert("/api/v2/posts", "posts_v2").unwrap();

    let mut group = c.benchmark_group("static_routing");

    group.bench_function("match_root", |b| b.iter(|| router.at(black_box("/"))));

    group.bench_function("match_health", |b| {
        b.iter(|| router.at(black_box("/health")))
    });

    group.bench_function("match_nested_v1", |b| {
        b.iter(|| router.at(black_box("/api/v1/users")))
    });

    group.bench_function("match_nested_v2", |b| {
        b.iter(|| router.at(black_box("/api/v2/posts")))
    });

    group.finish();
}

/// Benchmark dynamic route matching with path parameters
fn bench_dynamic_routes(c: &mut Criterion) {
    let mut router = Router::new();

    router.insert("/users/{id}", "get_user").unwrap();
    router
        .insert("/users/{id}/posts", "get_user_posts")
        .unwrap();
    router
        .insert("/users/{user_id}/posts/{post_id}", "get_user_post")
        .unwrap();
    router
        .insert(
            "/users/{user_id}/posts/{post_id}/comments/{comment_id}",
            "get_comment",
        )
        .unwrap();
    router
        .insert(
            "/categories/{cat}/products/{prod}/reviews/{rev}",
            "get_review",
        )
        .unwrap();

    let mut group = c.benchmark_group("dynamic_routing");

    group.bench_function("single_param", |b| {
        b.iter(|| router.at(black_box("/users/123")))
    });

    group.bench_function("single_param_nested", |b| {
        b.iter(|| router.at(black_box("/users/123/posts")))
    });

    group.bench_function("two_params", |b| {
        b.iter(|| router.at(black_box("/users/123/posts/456")))
    });

    group.bench_function("three_params", |b| {
        b.iter(|| router.at(black_box("/users/123/posts/456/comments/789")))
    });

    group.finish();
}

/// Benchmark router scaling with many routes
fn bench_router_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("router_scaling");

    for route_count in [10, 50, 100, 500].iter() {
        let mut router = Router::new();

        for i in 0..*route_count {
            router.insert(&format!("/api/v1/resource{}", i), i).unwrap();
        }

        // Always match the middle route
        let search_path = format!("/api/v1/resource{}", route_count / 2);

        group.bench_with_input(
            BenchmarkId::new("lookup", route_count),
            route_count,
            |b, _| b.iter(|| router.at(black_box(&search_path))),
        );
    }

    group.finish();
}

/// Benchmark wildcard routes
fn bench_wildcard_routes(c: &mut Criterion) {
    let mut router = Router::new();

    router.insert("/static/{*path}", "static_files").unwrap();
    router.insert("/assets/{*filepath}", "assets").unwrap();

    let mut group = c.benchmark_group("wildcard_routing");

    group.bench_function("short_path", |b| {
        b.iter(|| router.at(black_box("/static/css/style.css")))
    });

    group.bench_function("long_path", |b| {
        b.iter(|| router.at(black_box("/static/images/icons/social/facebook.png")))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_static_routes,
    bench_dynamic_routes,
    bench_router_scaling,
    bench_wildcard_routes,
);

criterion_main!(benches);
