use rustapi_context::TraceNode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A recorded session that can be replayed deterministically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaySession {
    /// Unique session identifier.
    pub session_id: String,
    /// The original trace tree snapshot.
    pub trace: TraceNode,
    /// Recorded LLM responses, keyed by trace node id.
    pub llm_responses: HashMap<String, serde_json::Value>,
    /// Recorded tool outputs, keyed by trace node id.
    pub tool_outputs: HashMap<String, serde_json::Value>,
    /// The final output of the original execution.
    pub final_output: serde_json::Value,
    /// Timestamp of recording.
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}

/// Result of a replay, with divergence detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    /// Whether the replay matched the original execution.
    pub matched: bool,
    /// The output produced by the replay.
    pub replay_output: serde_json::Value,
    /// Divergences detected (if any).
    pub divergences: Vec<ReplayDivergence>,
    /// Total replay duration in milliseconds.
    pub duration_ms: u64,
}

/// A specific point where replay diverged from the original.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayDivergence {
    /// Step index where divergence occurred.
    pub step_index: usize,
    /// Node id in the trace tree.
    pub node_id: String,
    /// What kind of divergence.
    pub kind: DivergenceKind,
    /// Description.
    pub message: String,
}

/// Classification of replay divergence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DivergenceKind {
    /// The step produced a different output.
    OutputMismatch,
    /// A step that should have been skipped was executed.
    UnexpectedExecution,
    /// A step that should have executed was skipped.
    MissingExecution,
    /// A different branch was taken.
    BranchMismatch,
}

/// Engine for deterministic replay of recorded agent executions.
///
/// During replay, LLM calls and tool executions are replaced with
/// recorded responses from the [`ReplaySession`], allowing exact
/// reproduction of a previous execution.
pub struct ReplayEngine;

impl ReplayEngine {
    /// Create a replay session from a trace tree snapshot.
    pub fn record(
        trace: TraceNode,
        llm_responses: HashMap<String, serde_json::Value>,
        tool_outputs: HashMap<String, serde_json::Value>,
        final_output: serde_json::Value,
    ) -> ReplaySession {
        ReplaySession {
            session_id: uuid::Uuid::new_v4().to_string(),
            trace,
            llm_responses,
            tool_outputs,
            final_output,
            recorded_at: chrono::Utc::now(),
        }
    }

    /// Compare a new execution's trace with a recorded session.
    ///
    /// Returns a [`ReplayResult`] indicating whether the outputs match
    /// and listing any divergences.
    pub fn compare(
        session: &ReplaySession,
        new_trace: &TraceNode,
        new_output: &serde_json::Value,
    ) -> ReplayResult {
        let start = std::time::Instant::now();
        let mut divergences = Vec::new();

        // Compare children count.
        if session.trace.children.len() != new_trace.children.len() {
            divergences.push(ReplayDivergence {
                step_index: 0,
                node_id: new_trace.id.clone(),
                kind: DivergenceKind::OutputMismatch,
                message: format!(
                    "Step count differs: original={}, replay={}",
                    session.trace.children.len(),
                    new_trace.children.len()
                ),
            });
        }

        // Compare individual steps.
        let len = session
            .trace
            .children
            .len()
            .min(new_trace.children.len());
        for i in 0..len {
            let original = &session.trace.children[i];
            let replayed = &new_trace.children[i];

            // Compare labels.
            if original.label != replayed.label {
                divergences.push(ReplayDivergence {
                    step_index: i,
                    node_id: replayed.id.clone(),
                    kind: DivergenceKind::BranchMismatch,
                    message: format!(
                        "Step label differs: original='{}', replay='{}'",
                        original.label, replayed.label
                    ),
                });
            }

            // Compare outputs.
            if original.output != replayed.output {
                divergences.push(ReplayDivergence {
                    step_index: i,
                    node_id: replayed.id.clone(),
                    kind: DivergenceKind::OutputMismatch,
                    message: format!("Output differs at step {i}"),
                });
            }
        }

        // Compare final output.
        let output_matched = &session.final_output == new_output;
        if !output_matched {
            divergences.push(ReplayDivergence {
                step_index: len,
                node_id: "final".to_string(),
                kind: DivergenceKind::OutputMismatch,
                message: "Final output differs".to_string(),
            });
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        ReplayResult {
            matched: divergences.is_empty(),
            replay_output: new_output.clone(),
            divergences,
            duration_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustapi_context::{TraceNode, TraceNodeKind};

    #[test]
    fn test_replay_record_and_compare_identical() {
        let mut trace = TraceNode::new(TraceNodeKind::HttpReceived, "POST /test");
        let mut step = TraceNode::new(TraceNodeKind::AgentStep, "think");
        step.complete(Some(serde_json::json!({"thought": "ok"})));
        trace.add_child(step);
        trace.complete(Some(serde_json::json!({"result": "done"})));

        let session = ReplayEngine::record(
            trace.clone(),
            HashMap::new(),
            HashMap::new(),
            serde_json::json!({"result": "done"}),
        );

        let result =
            ReplayEngine::compare(&session, &trace, &serde_json::json!({"result": "done"}));
        assert!(result.matched);
        assert!(result.divergences.is_empty());
    }

    #[test]
    fn test_replay_detect_output_divergence() {
        let mut trace1 = TraceNode::new(TraceNodeKind::HttpReceived, "POST /test");
        let mut step1 = TraceNode::new(TraceNodeKind::AgentStep, "think");
        step1.complete(Some(serde_json::json!({"thought": "a"})));
        trace1.add_child(step1);

        let mut trace2 = TraceNode::new(TraceNodeKind::HttpReceived, "POST /test");
        let mut step2 = TraceNode::new(TraceNodeKind::AgentStep, "think");
        step2.complete(Some(serde_json::json!({"thought": "b"})));
        trace2.add_child(step2);

        let session = ReplayEngine::record(
            trace1,
            HashMap::new(),
            HashMap::new(),
            serde_json::json!({"result": "x"}),
        );

        let result =
            ReplayEngine::compare(&session, &trace2, &serde_json::json!({"result": "y"}));
        assert!(!result.matched);
        assert!(!result.divergences.is_empty());
    }
}
