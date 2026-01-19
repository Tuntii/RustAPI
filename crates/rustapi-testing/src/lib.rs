//! Testing utilities for RustAPI
//!
//! r# "Mock Server"
//!
//! The `MockServer` allows you to mock HTTP services for integration testing.

pub mod expectation;
pub mod matcher;
pub mod server;

pub use expectation::{Expectation, MockResponse, Times};
pub use matcher::RequestMatcher;
pub use server::{MockServer, RecordedRequest};
