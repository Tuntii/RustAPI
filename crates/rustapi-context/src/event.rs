use crate::ContextError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

// ---------------------------------------------------------------------------
// ExecutionEvent — everything that can happen during AI execution
// ---------------------------------------------------------------------------

/// Events emitted through the [`EventBus`] during AI request processing.
///
/// Subscribers can observe these for logging, metrics, cost dashboards,
/// or custom side-effects without coupling into the execution pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExecutionEvent {
    /// HTTP request received and context created.
    RequestReceived {
        context_id: String,
        method: String,
        path: String,
        timestamp: DateTime<Utc>,
    },

    /// Immutable RequestContext has been constructed.
    ContextCreated {
        context_id: String,
        timestamp: DateTime<Utc>,
    },

    /// An agent execution step has started.
    AgentStepStarted {
        context_id: String,
        step_name: String,
        step_index: usize,
        timestamp: DateTime<Utc>,
    },

    /// An agent execution step has completed.
    AgentStepCompleted {
        context_id: String,
        step_name: String,
        step_index: usize,
        duration_ms: u64,
        success: bool,
        timestamp: DateTime<Utc>,
    },

    /// A tool is about to execute.
    ToolExecuting {
        context_id: String,
        tool_name: String,
        timestamp: DateTime<Utc>,
    },

    /// A tool has completed execution.
    ToolCompleted {
        context_id: String,
        tool_name: String,
        duration_ms: u64,
        success: bool,
        timestamp: DateTime<Utc>,
    },

    /// An LLM call has started.
    LlmCallStarted {
        context_id: String,
        model: String,
        timestamp: DateTime<Utc>,
    },

    /// An LLM call has completed.
    LlmCallCompleted {
        context_id: String,
        model: String,
        input_tokens: u64,
        output_tokens: u64,
        cost_micros: u64,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },

    /// Memory was accessed (read/write/search).
    MemoryAccessed {
        context_id: String,
        operation: String,
        key: Option<String>,
        timestamp: DateTime<Utc>,
    },

    /// The final response has been generated.
    ResponseGenerated {
        context_id: String,
        status_code: u16,
        total_duration_ms: u64,
        timestamp: DateTime<Utc>,
    },

    /// An error occurred during execution.
    ErrorOccurred {
        context_id: String,
        error: String,
        timestamp: DateTime<Utc>,
    },

    /// Cost counters were updated.
    CostUpdated {
        context_id: String,
        total_tokens: u64,
        total_cost_micros: u64,
        api_calls: u32,
        timestamp: DateTime<Utc>,
    },

    /// A planning decision was made.
    PlanGenerated {
        context_id: String,
        plan_summary: String,
        step_count: usize,
        timestamp: DateTime<Utc>,
    },

    /// A branch was taken in the execution graph.
    BranchTaken {
        context_id: String,
        branch_name: String,
        condition: String,
        timestamp: DateTime<Utc>,
    },
}

impl ExecutionEvent {
    /// Extract the context_id from any event variant.
    pub fn context_id(&self) -> &str {
        match self {
            Self::RequestReceived { context_id, .. }
            | Self::ContextCreated { context_id, .. }
            | Self::AgentStepStarted { context_id, .. }
            | Self::AgentStepCompleted { context_id, .. }
            | Self::ToolExecuting { context_id, .. }
            | Self::ToolCompleted { context_id, .. }
            | Self::LlmCallStarted { context_id, .. }
            | Self::LlmCallCompleted { context_id, .. }
            | Self::MemoryAccessed { context_id, .. }
            | Self::ResponseGenerated { context_id, .. }
            | Self::ErrorOccurred { context_id, .. }
            | Self::CostUpdated { context_id, .. }
            | Self::PlanGenerated { context_id, .. }
            | Self::BranchTaken { context_id, .. } => context_id,
        }
    }
}

// ---------------------------------------------------------------------------
// EventBus — broadcast-based event delivery
// ---------------------------------------------------------------------------

/// Broadcast-based event bus for the AI execution pipeline.
///
/// Uses `tokio::sync::broadcast` underneath, which is non-blocking and
/// supports multiple consumers. Slow consumers may miss events (lagged).
#[derive(Debug, Clone)]
pub struct EventBus {
    sender: Arc<broadcast::Sender<ExecutionEvent>>,
}

impl EventBus {
    /// Create a new event bus with the given channel capacity.
    ///
    /// A capacity of 256–1024 is usually sufficient for a single request.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender: Arc::new(sender),
        }
    }

    /// Emit an event to all subscribers.
    ///
    /// If there are no active subscribers the event is silently dropped.
    pub fn emit(&self, event: ExecutionEvent) {
        // Ignore send error — it means there are no active receivers.
        let _ = self.sender.send(event);
    }

    /// Subscribe to events.
    pub fn subscribe(&self) -> EventSubscriber {
        EventSubscriber {
            receiver: self.sender.subscribe(),
        }
    }

    /// Number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(512)
    }
}

// ---------------------------------------------------------------------------
// EventSubscriber
// ---------------------------------------------------------------------------

/// A subscriber that receives [`ExecutionEvent`]s from the [`EventBus`].
pub struct EventSubscriber {
    receiver: broadcast::Receiver<ExecutionEvent>,
}

impl EventSubscriber {
    /// Receive the next event, waiting if necessary.
    pub async fn recv(&mut self) -> Result<ExecutionEvent, ContextError> {
        loop {
            match self.receiver.recv().await {
                Ok(event) => return Ok(event),
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(missed = n, "Event subscriber lagged, skipping {n} events");
                    continue; // try again
                }
                Err(broadcast::error::RecvError::Closed) => {
                    return Err(ContextError::event_bus("Event bus closed"));
                }
            }
        }
    }

    /// Try to receive without waiting. Returns `None` if no event is ready.
    pub fn try_recv(&mut self) -> Option<ExecutionEvent> {
        self.receiver.try_recv().ok()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_pub_sub() {
        let bus = EventBus::new(16);
        let mut sub = bus.subscribe();

        bus.emit(ExecutionEvent::ContextCreated {
            context_id: "ctx-1".into(),
            timestamp: Utc::now(),
        });

        let event = sub.recv().await.unwrap();
        assert_eq!(event.context_id(), "ctx-1");
    }

    #[tokio::test]
    async fn test_event_bus_no_subscribers() {
        let bus = EventBus::new(16);
        // No panic when emitting without subscribers.
        bus.emit(ExecutionEvent::ContextCreated {
            context_id: "ctx-2".into(),
            timestamp: Utc::now(),
        });
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let bus = EventBus::new(16);
        let mut sub1 = bus.subscribe();
        let mut sub2 = bus.subscribe();

        bus.emit(ExecutionEvent::ToolExecuting {
            context_id: "ctx-3".into(),
            tool_name: "web_search".into(),
            timestamp: Utc::now(),
        });

        let e1 = sub1.recv().await.unwrap();
        let e2 = sub2.recv().await.unwrap();
        assert_eq!(e1.context_id(), "ctx-3");
        assert_eq!(e2.context_id(), "ctx-3");
    }

    #[test]
    fn test_event_serialization() {
        let event = ExecutionEvent::LlmCallCompleted {
            context_id: "ctx-4".into(),
            model: "gpt-4o".into(),
            input_tokens: 500,
            output_tokens: 200,
            cost_micros: 1500,
            duration_ms: 850,
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("llm_call_completed"));
        assert!(json.contains("gpt-4o"));
    }
}
