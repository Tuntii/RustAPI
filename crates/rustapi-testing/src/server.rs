use super::expectation::{Expectation, MockResponse, Times};
use super::matcher::RequestMatcher;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

/// A mock HTTP server
pub struct MockServer {
    addr: SocketAddr,
    state: Arc<Mutex<ServerState>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

struct ServerState {
    expectations: Vec<Expectation>,
    unmatched_requests: Vec<RecordedRequest>,
}

#[derive(Debug, Clone)]
pub struct RecordedRequest {
    pub method: http::Method,
    pub path: String,
    pub headers: http::HeaderMap,
    pub body: Bytes,
}

impl MockServer {
    /// Start a new mock server on a random port
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let state = Arc::new(Mutex::new(ServerState {
            expectations: Vec::new(),
            unmatched_requests: Vec::new(),
        }));

        let state_clone = state.clone();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        tokio::spawn(async move {
            let mut stop_future = shutdown_rx;

            loop {
                tokio::select! {
                    res = listener.accept() => {
                        match res {
                            Ok((stream, _)) => {
                                let io = TokioIo::new(stream);
                                let state = state_clone.clone();

                                tokio::spawn(async move {
                                    if let Err(err) = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                                        .serve_connection(io, service_fn(move |req| handle_request(req, state.clone())))
                                        .await
                                    {
                                        eprintln!("Error serving connection: {:?}", err);
                                    }
                                });
                            }
                            Err(e) => eprintln!("Accept error: {}", e),
                        }
                    }
                    _ = &mut stop_future => {
                        break;
                    }
                }
            }
        });

        Self {
            addr,
            state,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    /// Get the base URL of the server
    pub fn kind_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Alias for kind_url but more standard name
    pub fn base_url(&self) -> String {
        self.kind_url()
    }

    /// Get requests that didn't match any expectation
    pub fn unmatched_requests(&self) -> Vec<RecordedRequest> {
        let state = self.state.lock().unwrap();
        state.unmatched_requests.clone()
    }

    /// Add an expectation
    pub fn expect(&self, matcher: RequestMatcher) -> ExpectationBuilder {
        ExpectationBuilder {
            server: self.state.clone(),
            expectation: Some(Expectation::new(matcher)),
        }
    }

    /// Verify that all expectations were met
    pub fn verify(&self) {
        let state = self.state.lock().unwrap();
        for exp in &state.expectations {
            match exp.times {
                Times::Once => assert_eq!(
                    exp.call_count, 1,
                    "Expectation {:?} expected 1 call, got {}",
                    exp.matcher, exp.call_count
                ),
                Times::Exactly(n) => assert_eq!(
                    exp.call_count, n,
                    "Expectation {:?} expected {} calls, got {}",
                    exp.matcher, n, exp.call_count
                ),
                Times::AtLeast(n) => assert!(
                    exp.call_count >= n,
                    "Expectation {:?} expected at least {} calls, got {}",
                    exp.matcher,
                    n,
                    exp.call_count
                ),
                Times::AtMost(n) => assert!(
                    exp.call_count <= n,
                    "Expectation {:?} expected at most {} calls, got {}",
                    exp.matcher,
                    n,
                    exp.call_count
                ),
                Times::Any => {}
            }
        }
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

pub struct ExpectationBuilder {
    server: Arc<Mutex<ServerState>>,
    expectation: Option<Expectation>,
}

impl ExpectationBuilder {
    pub fn respond_with(mut self, response: MockResponse) -> Self {
        if let Some(exp) = self.expectation.as_mut() {
            exp.response = response;
        }
        self
    }

    pub fn times(mut self, n: usize) -> Self {
        if let Some(exp) = self.expectation.as_mut() {
            exp.times = Times::Exactly(n);
        }
        self
    }

    pub fn once(mut self) -> Self {
        if let Some(exp) = self.expectation.as_mut() {
            exp.times = Times::Once;
        }
        self
    }

    pub fn at_least_once(mut self) -> Self {
        if let Some(exp) = self.expectation.as_mut() {
            exp.times = Times::AtLeast(1);
        }
        self
    }

    pub fn never(mut self) -> Self {
        if let Some(exp) = self.expectation.as_mut() {
            exp.times = Times::Exactly(0);
        }
        self
    }
}

impl Drop for ExpectationBuilder {
    fn drop(&mut self) {
        if let Some(exp) = self.expectation.take() {
            let mut state = self.server.lock().unwrap();
            state.expectations.push(exp);
        }
    }
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    state: Arc<Mutex<ServerState>>,
) -> Result<Response<Full<Bytes>>> {
    // Read the full body
    let (parts, body) = req.into_parts();
    let body_bytes = body.collect().await?.to_bytes();

    let mut state_guard = state.lock().unwrap();

    // Find matching expectation
    // We iterate in reverse to prioritize later expectations (override)
    let matching_idx = state_guard
        .expectations
        .iter()
        .enumerate()
        .rev()
        .find(|(_, exp)| {
            exp.matcher
                .matches(&parts.method, parts.uri.path(), &parts.headers, &body_bytes)
        })
        .map(|(i, _)| i);

    if let Some(idx) = matching_idx {
        let exp = &mut state_guard.expectations[idx];
        exp.call_count += 1;

        let resp_def = &exp.response;
        let mut response = Response::builder().status(resp_def.status);

        for (k, v) in &resp_def.headers {
            response = response.header(k, v);
        }

        Ok(response.body(Full::new(resp_def.body.clone()))?)
    } else {
        // Record unmatched
        state_guard.unmatched_requests.push(RecordedRequest {
            method: parts.method,
            path: parts.uri.path().to_string(),
            headers: parts.headers,
            body: body_bytes,
        });

        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("No expectation matched")))?)
    }
}
