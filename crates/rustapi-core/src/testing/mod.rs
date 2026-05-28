//! Testing utilities for RustAPI
//!
//! This module provides test helpers for integration testing
//! without network binding. Available behind the `test-utils` feature.
//!
//! # Mock Server
//!
//! The `MockServer` allows you to mock HTTP services for integration testing.

pub mod client;
pub mod expectation;
pub mod matcher;
pub mod server;

pub use client::{TestClient, TestRequest, TestResponse};
pub use expectation::{Expectation, MockResponse, Times};
pub use matcher::RequestMatcher;
pub use server::{MockServer, RecordedRequest};
