//! Sprint 2 Integration Tests — Event System
//!
//! Tests for EventBus and lifecycle hook integration via the facade.

use rustapi_rs::prelude::*;
use std::sync::{Arc, Mutex};

// ─── EventBus via Facade ────────────────────────────────────────────────────

#[test]
fn event_bus_sync_via_facade() {
    let bus = EventBus::new();
    let received = Arc::new(Mutex::new(Vec::new()));

    let r = received.clone();
    bus.on("test.event", move |payload: &str| {
        r.lock().unwrap().push(payload.to_string());
    });

    bus.emit("test.event", "hello");
    let data = received.lock().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0], "hello");
}

#[test]
fn event_bus_multiple_handlers_via_facade() {
    let bus = EventBus::new();
    let count = Arc::new(Mutex::new(0));

    for _ in 0..3 {
        let c = count.clone();
        bus.on("multi", move |_: &str| {
            *c.lock().unwrap() += 1;
        });
    }

    bus.emit("multi", "");
    assert_eq!(*count.lock().unwrap(), 3);
}

#[tokio::test]
async fn event_bus_async_emit_await() {
    let bus = EventBus::new();
    let result = Arc::new(Mutex::new(String::new()));

    let r = result.clone();
    bus.on_async("async.test", move |payload: String| {
        let r = r.clone();
        Box::pin(async move {
            *r.lock().unwrap() = payload;
        })
    });

    bus.emit_await("async.test", "async_payload").await;
    assert_eq!(*result.lock().unwrap(), "async_payload");
}

#[test]
fn event_bus_handler_count() {
    let bus = EventBus::new();
    assert_eq!(bus.handler_count("empty"), 0);

    bus.on("counted", |_: &str| {});
    bus.on("counted", |_: &str| {});
    assert_eq!(bus.handler_count("counted"), 2);
}

#[test]
fn event_bus_topics() {
    let bus = EventBus::new();
    bus.on("topic.a", |_: &str| {});
    bus.on("topic.b", |_: &str| {});

    let topics = bus.topics();
    assert!(topics.contains(&"topic.a".to_string()));
    assert!(topics.contains(&"topic.b".to_string()));
}

// ─── RustApi Lifecycle Hooks Builder ────────────────────────────────────────

#[test]
fn rustapi_on_start_builder_chain() {
    // Verify the builder pattern compiles and chains correctly
    let _app = RustApi::new()
        .on_start(|| async {
            // This would run before server starts
        })
        .on_start(|| async {
            // Multiple hooks supported
        })
        .on_shutdown(|| async {
            // Shutdown hook
        });
}
