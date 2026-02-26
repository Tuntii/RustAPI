//! Event System and Lifecycle Hooks
//!
//! Provides an in-process publish/subscribe event bus and lifecycle hooks
//! for the RustAPI application.
//!
//! # Lifecycle Hooks
//!
//! ```rust,ignore
//! use rustapi_core::RustApi;
//!
//! RustApi::new()
//!     .on_start(|| async {
//!         println!("Server started!");
//!     })
//!     .on_shutdown(|| async {
//!         println!("Server shutting down...");
//!     })
//!     .run("127.0.0.1:8080")
//!     .await
//! ```
//!
//! # Event Bus
//!
//! ```rust,ignore
//! use rustapi_core::events::EventBus;
//! use rustapi_core::State;
//!
//! let bus = EventBus::new();
//!
//! // Subscribe to events
//! bus.on("user.created", |payload: &str| {
//!     println!("User created: {}", payload);
//! });
//!
//! // In a handler, emit events
//! async fn create_user(State(bus): State<EventBus>) -> impl IntoResponse {
//!     bus.emit("user.created", "user_123");
//!     "created"
//! }
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

// ─── Lifecycle Hooks ────────────────────────────────────────────────────────

/// A boxed async callback for lifecycle hooks
pub(crate) type LifecycleHook =
    Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

/// Stores registered lifecycle hooks
pub(crate) struct LifecycleHooks {
    pub on_start: Vec<LifecycleHook>,
    pub on_shutdown: Vec<LifecycleHook>,
}

impl LifecycleHooks {
    pub fn new() -> Self {
        Self {
            on_start: Vec::new(),
            on_shutdown: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn has_hooks(&self) -> bool {
        !self.on_start.is_empty() || !self.on_shutdown.is_empty()
    }
}

impl Default for LifecycleHooks {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Event Bus ──────────────────────────────────────────────────────────────

/// Callback type for synchronous event handlers
type SyncHandler = Arc<dyn Fn(&str) + Send + Sync>;

/// Callback type for async event handlers
type AsyncHandler = Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// In-process publish/subscribe event bus
///
/// Supports both synchronous and asynchronous event handlers.
/// Multiple handlers can be registered for the same event topic.
///
/// # Example
///
/// ```rust
/// use rustapi_core::events::EventBus;
///
/// let bus = EventBus::new();
///
/// // Synchronous handler
/// bus.on("user.created", |payload: &str| {
///     println!("User created: {}", payload);
/// });
///
/// // Emit events
/// bus.emit("user.created", "user_123");
/// ```
#[derive(Clone)]
pub struct EventBus {
    sync_handlers: Arc<RwLock<HashMap<String, Vec<SyncHandler>>>>,
    async_handlers: Arc<RwLock<HashMap<String, Vec<AsyncHandler>>>>,
}

impl EventBus {
    /// Create a new EventBus instance
    pub fn new() -> Self {
        Self {
            sync_handlers: Arc::new(RwLock::new(HashMap::new())),
            async_handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a synchronous event handler for a topic
    ///
    /// The handler will be called with the event payload string whenever
    /// the topic is emitted.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustapi_core::events::EventBus;
    ///
    /// let bus = EventBus::new();
    /// bus.on("order.completed", |payload: &str| {
    ///     println!("Order completed: {}", payload);
    /// });
    /// ```
    pub fn on<F>(&self, topic: &str, handler: F)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        let mut handlers = self.sync_handlers.write().unwrap();
        handlers
            .entry(topic.to_string())
            .or_default()
            .push(Arc::new(handler));
    }

    /// Register an async event handler for a topic
    ///
    /// The handler will be spawned as a tokio task when the topic is emitted.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_core::events::EventBus;
    ///
    /// let bus = EventBus::new();
    /// bus.on_async("email.send", |payload: String| {
    ///     Box::pin(async move {
    ///         send_email(&payload).await;
    ///     })
    /// });
    /// ```
    pub fn on_async<F>(&self, topic: &str, handler: F)
    where
        F: Fn(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static,
    {
        let mut handlers = self.async_handlers.write().unwrap();
        handlers
            .entry(topic.to_string())
            .or_default()
            .push(Arc::new(handler));
    }

    /// Emit an event synchronously
    ///
    /// Calls all synchronous handlers for the topic in registration order.
    /// Also spawns tokio tasks for any async handlers.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustapi_core::events::EventBus;
    ///
    /// let bus = EventBus::new();
    /// bus.on("log", |msg: &str| println!("{}", msg));
    /// bus.emit("log", "Hello!");
    /// ```
    pub fn emit(&self, topic: &str, payload: &str) {
        // Call sync handlers
        if let Ok(handlers) = self.sync_handlers.read() {
            if let Some(topic_handlers) = handlers.get(topic) {
                for handler in topic_handlers {
                    handler(payload);
                }
            }
        }

        // Spawn async handlers
        if let Ok(handlers) = self.async_handlers.read() {
            if let Some(topic_handlers) = handlers.get(topic) {
                for handler in topic_handlers {
                    let handler = handler.clone();
                    let payload = payload.to_string();
                    tokio::spawn(async move {
                        handler(payload).await;
                    });
                }
            }
        }
    }

    /// Emit an event and await all async handlers
    ///
    /// Unlike `emit()`, this waits for all async handlers to complete.
    pub async fn emit_await(&self, topic: &str, payload: &str) {
        // Call sync handlers
        {
            let handlers = self.sync_handlers.read().unwrap();
            if let Some(topic_handlers) = handlers.get(topic) {
                for handler in topic_handlers {
                    handler(payload);
                }
            }
        }

        // Await async handlers
        let tasks = {
            let handlers = self.async_handlers.read().unwrap();
            if let Some(topic_handlers) = handlers.get(topic) {
                topic_handlers
                    .iter()
                    .map(|handler| {
                        let handler = handler.clone();
                        let payload = payload.to_string();
                        tokio::spawn(async move {
                            handler(payload).await;
                        })
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        };
        for task in tasks {
            let _ = task.await;
        }
    }

    /// Get the number of registered handlers for a topic (both sync and async)
    pub fn handler_count(&self, topic: &str) -> usize {
        let sync_count = self
            .sync_handlers
            .read()
            .map(|h| h.get(topic).map_or(0, |v| v.len()))
            .unwrap_or(0);
        let async_count = self
            .async_handlers
            .read()
            .map(|h| h.get(topic).map_or(0, |v| v.len()))
            .unwrap_or(0);
        sync_count + async_count
    }

    /// Get all registered topic names
    pub fn topics(&self) -> Vec<String> {
        let mut topics = Vec::new();
        if let Ok(handlers) = self.sync_handlers.read() {
            topics.extend(handlers.keys().cloned());
        }
        if let Ok(handlers) = self.async_handlers.read() {
            for key in handlers.keys() {
                if !topics.contains(key) {
                    topics.push(key.clone());
                }
            }
        }
        topics
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_sync_event_handler() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        bus.on("test.event", move |_payload: &str| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        bus.emit("test.event", "hello");
        bus.emit("test.event", "world");

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_multiple_handlers() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let c1 = counter.clone();
        bus.on("multi", move |_| {
            c1.fetch_add(1, Ordering::SeqCst);
        });

        let c2 = counter.clone();
        bus.on("multi", move |_| {
            c2.fetch_add(10, Ordering::SeqCst);
        });

        bus.emit("multi", "");
        assert_eq!(counter.load(Ordering::SeqCst), 11);
    }

    #[test]
    fn test_handler_count() {
        let bus = EventBus::new();
        assert_eq!(bus.handler_count("topic"), 0);

        bus.on("topic", |_| {});
        assert_eq!(bus.handler_count("topic"), 1);

        bus.on("topic", |_| {});
        assert_eq!(bus.handler_count("topic"), 2);
    }

    #[test]
    fn test_topics() {
        let bus = EventBus::new();
        bus.on("a", |_| {});
        bus.on("b", |_| {});

        let topics = bus.topics();
        assert!(topics.contains(&"a".to_string()));
        assert!(topics.contains(&"b".to_string()));
    }

    #[test]
    fn test_unregistered_topic_is_noop() {
        let bus = EventBus::new();
        // Should not panic
        bus.emit("nonexistent", "payload");
    }

    #[tokio::test]
    async fn test_async_event_handler() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        bus.on_async("async.event", move |_payload: String| {
            let counter = counter_clone.clone();
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
            })
        });

        bus.emit_await("async.event", "hello").await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_emit_await_waits_for_all() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let c1 = counter.clone();
        bus.on_async("wait", move |_| {
            let c = c1.clone();
            Box::pin(async move {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                c.fetch_add(1, Ordering::SeqCst);
            })
        });

        let c2 = counter.clone();
        bus.on_async("wait", move |_| {
            let c = c2.clone();
            Box::pin(async move {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                c.fetch_add(1, Ordering::SeqCst);
            })
        });

        bus.emit_await("wait", "").await;
        // Both handlers should have completed
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_lifecycle_hooks_default() {
        let hooks = LifecycleHooks::new();
        assert!(!hooks.has_hooks());
    }
}
