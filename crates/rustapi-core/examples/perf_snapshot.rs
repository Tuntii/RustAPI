use bytes::Bytes;
use http::{Extensions, Method, StatusCode};
use rustapi_core::interceptor::{RequestInterceptor, ResponseInterceptor};
use rustapi_core::middleware::{BoxedNext, MiddlewareLayer};
use rustapi_core::{get, BodyVariant, IntoResponse, PathParams, Request, Response, RouteMatch, RustApi};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

const DEFAULT_WARMUP_ITERS: usize = 2_000;
const DEFAULT_SAMPLE_ITERS: usize = 20_000;

#[derive(Clone)]
struct NoopRequestInterceptor;

impl RequestInterceptor for NoopRequestInterceptor {
    fn intercept(&self, request: Request) -> Request {
        request
    }

    fn clone_box(&self) -> Box<dyn RequestInterceptor> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
struct NoopResponseInterceptor;

impl ResponseInterceptor for NoopResponseInterceptor {
    fn intercept(&self, response: Response) -> Response {
        response
    }

    fn clone_box(&self) -> Box<dyn ResponseInterceptor> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
struct NoopMiddleware;

impl MiddlewareLayer for NoopMiddleware {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        Box::pin(async move { next(req).await })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
struct Scenario {
    name: &'static str,
    path_kind: &'static str,
    features: &'static str,
    router: Arc<rustapi_core::Router>,
    layers: Arc<rustapi_core::middleware::LayerStack>,
    interceptors: Arc<rustapi_core::InterceptorChain>,
}

#[derive(Debug, Clone)]
struct ScenarioResult {
    name: &'static str,
    path_kind: &'static str,
    features: &'static str,
    throughput_req_s: f64,
    p50_us: f64,
    p95_us: f64,
    p99_us: f64,
    mean_us: f64,
}

fn scenario(name: &'static str, path_kind: &'static str, features: &'static str, app: RustApi) -> Scenario {
    let layers = app.layers().clone();
    let interceptors = app.interceptors().clone();
    let router = app.into_router();

    Scenario {
        name,
        path_kind,
        features,
        router: Arc::new(router),
        layers: Arc::new(layers),
        interceptors: Arc::new(interceptors),
    }
}

fn build_request(state: Arc<Extensions>) -> Request {
    let req = http::Request::builder()
        .method(Method::GET)
        .uri("/hello")
        .body(())
        .expect("request build should succeed");
    let (parts, _) = req.into_parts();

    Request::new(
        parts,
        BodyVariant::Buffered(Bytes::new()),
        state,
        PathParams::new(),
    )
}

async fn route_request_direct(
    router: &rustapi_core::Router,
    request: Request,
    path: &str,
    method: &Method,
) -> Response {
    match router.match_route(path, method) {
        RouteMatch::Found { handler, .. } => {
            handler(request).await
        }
        RouteMatch::NotFound => rustapi_core::ApiError::not_found("Not found").into_response(),
        RouteMatch::MethodNotAllowed { allowed } => {
            let allowed_str: Vec<&str> = allowed.iter().map(|m| m.as_str()).collect();
            let mut response = rustapi_core::ApiError::new(
                StatusCode::METHOD_NOT_ALLOWED,
                "method_not_allowed",
                "Method not allowed",
            )
            .into_response();
            response.headers_mut().insert(
                http::header::ALLOW,
                allowed_str.join(", ").parse().expect("allow header should parse"),
            );
            response
        }
    }
}

async fn execute_scenario_request(scenario: &Scenario) -> Response {
    let method = Method::GET;
    let path = "/hello";
    let request = build_request(scenario.router.state_ref());

    if scenario.layers.is_empty() && scenario.interceptors.is_empty() {
        route_request_direct(&scenario.router, request, path, &method).await
    } else if scenario.layers.is_empty() {
        let request = scenario.interceptors.intercept_request(request);
        let response = route_request_direct(&scenario.router, request, path, &method).await;
        scenario.interceptors.intercept_response(response)
    } else {
        let request = scenario.interceptors.intercept_request(request);
        let router = scenario.router.clone();
        let path = path.to_string();
        let method = method.clone();

        let final_handler: BoxedNext = Arc::new(move |req: Request| {
            let router = router.clone();
            let path = path.clone();
            let method = method.clone();
            Box::pin(async move { route_request_direct(&router, req, &path, &method).await })
                as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        let response = scenario.layers.execute(request, final_handler).await;
        scenario.interceptors.intercept_response(response)
    }
}

async fn measure_scenario(
    scenario: &Scenario,
    warmup_iters: usize,
    sample_iters: usize,
) -> ScenarioResult {
    for _ in 0..warmup_iters {
        let response = execute_scenario_request(scenario).await;
        std::hint::black_box(response.status());
    }

    let mut latencies_ns = Vec::with_capacity(sample_iters);
    let wall_clock_start = Instant::now();

    for _ in 0..sample_iters {
        let request_start = Instant::now();
        let response = execute_scenario_request(scenario).await;
        let elapsed = request_start.elapsed();

        assert_eq!(response.status(), StatusCode::OK, "benchmark scenario must stay healthy");

        latencies_ns.push(elapsed.as_nanos() as u64);
        std::hint::black_box(response.status());
    }

    let wall_clock_elapsed = wall_clock_start.elapsed();
    latencies_ns.sort_unstable();

    let total_ns: u128 = latencies_ns.iter().map(|&v| v as u128).sum();
    let mean_ns = total_ns as f64 / sample_iters as f64;

    ScenarioResult {
        name: scenario.name,
        path_kind: scenario.path_kind,
        features: scenario.features,
        throughput_req_s: sample_iters as f64 / wall_clock_elapsed.as_secs_f64(),
        p50_us: percentile_us(&latencies_ns, 50.0),
        p95_us: percentile_us(&latencies_ns, 95.0),
        p99_us: percentile_us(&latencies_ns, 99.0),
        mean_us: mean_ns / 1_000.0,
    }
}

fn percentile_us(sorted_latencies_ns: &[u64], percentile: f64) -> f64 {
    if sorted_latencies_ns.is_empty() {
        return 0.0;
    }

    let max_index = sorted_latencies_ns.len() - 1;
    let rank = ((percentile / 100.0) * max_index as f64).round() as usize;
    sorted_latencies_ns[rank.min(max_index)] as f64 / 1_000.0
}

fn parse_env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn print_results(results: &[ScenarioResult]) {
    println!("# RustAPI Performance Snapshot");
    println!();
    println!("Synthetic in-process request pipeline benchmark.");
    println!();
    println!(
        "| Scenario | Execution path | Features | Req/s | Mean (µs) | p50 (µs) | p95 (µs) | p99 (µs) |"
    );
    println!(
        "|---|---|---|---:|---:|---:|---:|---:|"
    );

    for result in results {
        println!(
            "| {} | {} | {} | {:.0} | {:.2} | {:.2} | {:.2} | {:.2} |",
            result.name,
            result.path_kind,
            result.features,
            result.throughput_req_s,
            result.mean_us,
            result.p50_us,
            result.p95_us,
            result.p99_us,
        );
    }

    println!();
    if let Some(baseline) = results.iter().find(|result| result.name == "baseline") {
        println!("## Relative overhead vs baseline");
        println!();
        println!("| Scenario | Req/s delta | p99 delta |",
        );
        println!("|---|---:|---:|");
        for result in results {
            let req_delta = ((result.throughput_req_s / baseline.throughput_req_s) - 1.0) * 100.0;
            let p99_delta = ((result.p99_us / baseline.p99_us) - 1.0) * 100.0;
            println!(
                "| {} | {:+.2}% | {:+.2}% |",
                result.name,
                req_delta,
                p99_delta,
            );
        }
    }
}

async fn hello() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let warmup_iters = parse_env_usize("RUSTAPI_PERF_WARMUP", DEFAULT_WARMUP_ITERS);
    let sample_iters = parse_env_usize("RUSTAPI_PERF_ITERS", DEFAULT_SAMPLE_ITERS);

    let scenarios = vec![
        scenario(
            "baseline",
            "ultra fast",
            "no middleware, no interceptors",
            RustApi::new().route("/hello", get(hello)),
        ),
        scenario(
            "request_interceptor",
            "fast",
            "1 request interceptor",
            RustApi::new()
                .request_interceptor(NoopRequestInterceptor)
                .route("/hello", get(hello)),
        ),
        scenario(
            "request_response_interceptors",
            "fast",
            "1 request + 1 response interceptor",
            RustApi::new()
                .request_interceptor(NoopRequestInterceptor)
                .response_interceptor(NoopResponseInterceptor)
                .route("/hello", get(hello)),
        ),
        scenario(
            "middleware_only",
            "full",
            "1 middleware layer",
            RustApi::new().layer(NoopMiddleware).route("/hello", get(hello)),
        ),
        scenario(
            "full_stack_minimal",
            "full",
            "1 middleware + 1 request + 1 response interceptor",
            RustApi::new()
                .layer(NoopMiddleware)
                .request_interceptor(NoopRequestInterceptor)
                .response_interceptor(NoopResponseInterceptor)
                .route("/hello", get(hello)),
        ),
        scenario(
            "request_id_layer",
            "full",
            "RequestIdLayer",
            RustApi::new()
                .layer(rustapi_core::RequestIdLayer::new())
                .route("/hello", get(hello)),
        ),
    ];

    println!("Warmup iterations: {}", warmup_iters);
    println!("Measured iterations: {}", sample_iters);
    println!();

    let mut results = Vec::with_capacity(scenarios.len());
    for scenario in &scenarios {
        println!("Running scenario: {}", scenario.name);
        results.push(measure_scenario(scenario, warmup_iters, sample_iters).await);
    }

    println!();
    print_results(&results);

    Ok(())
}