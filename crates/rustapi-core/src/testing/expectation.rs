use super::matcher::RequestMatcher;
use bytes::Bytes;
use http::{HeaderMap, StatusCode};

/// An expectation for a request
#[derive(Debug, Clone)]
pub struct Expectation {
    pub(crate) matcher: RequestMatcher,
    pub(crate) response: MockResponse,
    pub(crate) times: Times,
    pub(crate) call_count: usize,
}

impl Expectation {
    /// Create a new expectation
    pub fn new(matcher: RequestMatcher) -> Self {
        Self {
            matcher,
            response: MockResponse::default(),
            times: Times::Once,
            call_count: 0,
        }
    }

    /// Set the response to match
    pub fn respond_with(mut self, response: MockResponse) -> Self {
        self.response = response;
        self
    }

    /// Expect the request to be called exactly once
    pub fn once(mut self) -> Self {
        self.times = Times::Once;
        self
    }

    /// Expect the request to be called exactly n times
    pub fn times(mut self, n: usize) -> Self {
        self.times = Times::Exactly(n);
        self
    }

    /// Expect the request to be called at least once
    pub fn at_least_once(mut self) -> Self {
        self.times = Times::AtLeast(1);
        self
    }

    /// Expect the request to never be called
    pub fn never(mut self) -> Self {
        self.times = Times::Exactly(0);
        self
    }
}

/// Define how many times an expectation should be matched
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Times {
    Once,
    Exactly(usize),
    AtLeast(usize),
    AtMost(usize),
    Any,
}

/// A mocked response
#[derive(Debug, Clone)]
pub struct MockResponse {
    pub(crate) status: StatusCode,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Bytes,
}

impl Default for MockResponse {
    fn default() -> Self {
        Self {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: Bytes::new(),
        }
    }
}

impl MockResponse {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(
            http::header::HeaderName::from_bytes(key.as_bytes()).unwrap(),
            http::header::HeaderValue::from_str(value).unwrap(),
        );
        self
    }

    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }

    pub fn json(mut self, body: impl serde::Serialize) -> Self {
        self.headers.insert(
            http::header::CONTENT_TYPE,
            http::header::HeaderValue::from_static("application/json"),
        );
        self.body = serde_json::to_vec(&body).unwrap().into();
        self
    }
}
