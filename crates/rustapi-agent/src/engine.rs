use crate::{AgentContext, AgentError, ExecutionPlan, Step, StepResult};
use chrono::Utc;
use rustapi_context::{ExecutionEvent, TraceNodeKind};
use std::sync::Arc;

/// Configuration for the agent engine.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Maximum number of steps before the engine aborts (runaway protection).
    pub max_steps: usize,
    /// Whether to emit events to the event bus.
    pub emit_events: bool,
    /// Whether to record all step I/O in the trace tree.
    pub trace_step_io: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_steps: 50,
            emit_events: true,
            trace_step_io: true,
        }
    }
}

/// The result of a complete agent execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentResult {
    /// The final output value.
    pub value: serde_json::Value,
    /// Total steps executed.
    pub steps_executed: usize,
    /// Total execution time in milliseconds.
    pub duration_ms: u64,
    /// Partial yields emitted during execution (if any).
    pub yields: Vec<serde_json::Value>,
}

/// Step-based, deterministic agent execution engine.
///
/// Executes a sequence of [`Step`]s according to an [`ExecutionPlan`],
/// supporting branching, streaming yields, and cost-aware early termination.
///
/// # Example
///
/// ```ignore
/// let engine = AgentEngine::new(config);
/// let result = engine.run(plan, steps, &mut agent_ctx).await?;
/// ```
pub struct AgentEngine {
    config: EngineConfig,
}

impl AgentEngine {
    /// Create a new agent engine with the given configuration.
    pub fn new(config: EngineConfig) -> Self {
        Self { config }
    }

    /// Create an engine with default configuration.
    pub fn default_engine() -> Self {
        Self {
            config: EngineConfig::default(),
        }
    }

    /// Execute an execution plan with the provided steps.
    ///
    /// `step_impls` maps step names to their implementations.
    /// Steps in the plan are matched by name to find the implementation.
    pub async fn run(
        &self,
        plan: &ExecutionPlan,
        step_impls: &[Arc<dyn Step>],
        ctx: &mut AgentContext,
    ) -> Result<AgentResult, AgentError> {
        let start = std::time::Instant::now();
        let mut step_idx = 0;
        let mut total_executed = 0;
        let mut last_value = serde_json::Value::Null;

        if self.config.emit_events {
            if let Some(ref summary) = plan.summary {
                ctx.request_context().event_bus().emit(ExecutionEvent::PlanGenerated {
                    context_id: ctx.context_id().to_string(),
                    plan_summary: summary.clone(),
                    step_count: plan.steps.len(),
                    timestamp: Utc::now(),
                });
            }
        }

        loop {
            // Safety: prevent infinite loops.
            if total_executed >= self.config.max_steps {
                return Err(AgentError::MaxStepsExceeded {
                    max_steps: self.config.max_steps,
                });
            }

            // End of plan.
            if step_idx >= plan.steps.len() {
                break;
            }

            let planned = &plan.steps[step_idx];

            // Find the Step implementation by name.
            let step_impl = step_impls
                .iter()
                .find(|s| s.name() == planned.name)
                .ok_or_else(|| {
                    AgentError::step_failed(&planned.name, "No implementation found for step")
                })?;

            // Set up input state from the plan if provided.
            if let Some(ref input) = planned.input {
                ctx.set_state("step_input", input.clone());
            }
            ctx.set_state("previous_result", last_value.clone());

            // Emit event.
            if self.config.emit_events {
                ctx.request_context().event_bus().emit(ExecutionEvent::AgentStepStarted {
                    context_id: ctx.context_id().to_string(),
                    step_name: planned.name.clone(),
                    step_index: total_executed,
                    timestamp: Utc::now(),
                });
            }

            // Trace.
            let mut span = ctx
                .request_context()
                .trace()
                .start_span(TraceNodeKind::AgentStep, &planned.name);
            if self.config.trace_step_io {
                span.set_input(serde_json::json!({
                    "step_index": step_idx,
                    "step_name": planned.name,
                    "planned_tool": planned.tool_name,
                }));
            }

            // Execute step.
            let step_start = std::time::Instant::now();
            let result = step_impl.execute(ctx).await;
            let step_duration_ms = step_start.elapsed().as_millis() as u64;

            total_executed += 1;
            ctx.advance_step();

            match result {
                Ok(step_result) => {
                    // Emit completion event.
                    if self.config.emit_events {
                        ctx.request_context().event_bus().emit(
                            ExecutionEvent::AgentStepCompleted {
                                context_id: ctx.context_id().to_string(),
                                step_name: planned.name.clone(),
                                step_index: total_executed - 1,
                                duration_ms: step_duration_ms,
                                success: !matches!(step_result, StepResult::Error { .. }),
                                timestamp: Utc::now(),
                            },
                        );
                    }

                    match step_result {
                        StepResult::Continue { value } => {
                            span.complete(Some(value.clone()));
                            last_value = value;
                            step_idx += 1;
                        }
                        StepResult::Branch { branch_name, value } => {
                            if self.config.emit_events {
                                ctx.request_context().event_bus().emit(
                                    ExecutionEvent::BranchTaken {
                                        context_id: ctx.context_id().to_string(),
                                        branch_name: branch_name.clone(),
                                        condition: format!("step '{}' branched", planned.name),
                                        timestamp: Utc::now(),
                                    },
                                );
                            }

                            let target = plan
                                .branches
                                .get(&branch_name)
                                .ok_or_else(|| AgentError::branch_not_found(&branch_name))?;

                            span.complete(Some(serde_json::json!({
                                "branch": branch_name,
                                "target_step": target,
                                "value": value,
                            })));

                            last_value = value;
                            step_idx = *target;
                        }
                        StepResult::Yield { value } => {
                            span.complete(Some(value.clone()));
                            ctx.record_yield(value.clone());
                            last_value = value;
                            step_idx += 1;
                        }
                        StepResult::Complete { value } => {
                            span.complete(Some(value.clone()));
                            last_value = value;
                            break;
                        }
                        StepResult::Error { message } => {
                            span.fail(&message);
                            return Err(AgentError::step_failed(&planned.name, &message));
                        }
                    }
                }
                Err(e) => {
                    span.fail(e.to_string());
                    if self.config.emit_events {
                        ctx.request_context().event_bus().emit(
                            ExecutionEvent::AgentStepCompleted {
                                context_id: ctx.context_id().to_string(),
                                step_name: planned.name.clone(),
                                step_index: total_executed - 1,
                                duration_ms: step_duration_ms,
                                success: false,
                                timestamp: Utc::now(),
                            },
                        );
                    }
                    return Err(e);
                }
            }

            // Check cost budget after each step.
            ctx.request_context().cost().check_budget()?;
        }

        let total_duration_ms = start.elapsed().as_millis() as u64;

        // Complete the root trace.
        ctx.request_context().trace().complete_root(Some(last_value.clone()));

        Ok(AgentResult {
            value: last_value,
            steps_executed: total_executed,
            duration_ms: total_duration_ms,
            yields: ctx.yields().to_vec(),
        })
    }
}

impl Default for AgentEngine {
    fn default() -> Self {
        Self::default_engine()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PlannedStep, Step, StepResult};
    use async_trait::async_trait;
    use rustapi_context::RequestContextBuilder;
    use rustapi_memory::backend::InMemoryStore;
    use rustapi_tools::ToolRegistry;

    struct IncrementStep;

    #[async_trait]
    impl Step for IncrementStep {
        fn name(&self) -> &str {
            "increment"
        }
        async fn execute(&self, ctx: &mut AgentContext) -> Result<StepResult, AgentError> {
            let prev = ctx
                .get_state("previous_result")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            Ok(StepResult::cont(serde_json::json!(prev + 1)))
        }
    }

    struct CompleteStep;

    #[async_trait]
    impl Step for CompleteStep {
        fn name(&self) -> &str {
            "complete"
        }
        async fn execute(&self, ctx: &mut AgentContext) -> Result<StepResult, AgentError> {
            let prev = ctx
                .get_state("previous_result")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            Ok(StepResult::complete(prev))
        }
    }

    fn make_ctx() -> AgentContext {
        let request_ctx = RequestContextBuilder::new()
            .method("POST")
            .path("/test")
            .build();
        let tools = ToolRegistry::new();
        let memory = Arc::new(InMemoryStore::new());
        AgentContext::new(request_ctx, tools, memory)
    }

    #[tokio::test]
    async fn test_engine_basic_sequence() {
        let engine = AgentEngine::default_engine();
        let plan = ExecutionPlan::new(vec![
            PlannedStep::new("increment", "Increment counter"),
            PlannedStep::new("increment", "Increment again"),
            PlannedStep::new("complete", "Finish"),
        ]);

        let steps: Vec<Arc<dyn Step>> = vec![Arc::new(IncrementStep), Arc::new(CompleteStep)];

        let mut ctx = make_ctx();
        let result = engine.run(&plan, &steps, &mut ctx).await.unwrap();

        assert_eq!(result.steps_executed, 3);
        assert_eq!(result.value, serde_json::json!(2));
    }

    #[tokio::test]
    async fn test_engine_max_steps() {
        struct LoopStep;

        #[async_trait]
        impl Step for LoopStep {
            fn name(&self) -> &str {
                "loop"
            }
            async fn execute(&self, _ctx: &mut AgentContext) -> Result<StepResult, AgentError> {
                Ok(StepResult::branch(
                    "loop",
                    serde_json::json!("looping"),
                ))
            }
        }

        let engine = AgentEngine::new(EngineConfig {
            max_steps: 5,
            ..Default::default()
        });

        let plan = ExecutionPlan::new(vec![
            PlannedStep::new("loop", "Loop forever").with_branch_label("loop"),
        ])
        .with_branch("loop", 0);

        let steps: Vec<Arc<dyn Step>> = vec![Arc::new(LoopStep)];
        let mut ctx = make_ctx();

        let err = engine.run(&plan, &steps, &mut ctx).await.unwrap_err();
        assert!(matches!(err, AgentError::MaxStepsExceeded { max_steps: 5 }));
    }

    #[tokio::test]
    async fn test_engine_branching() {
        struct BranchStep;

        #[async_trait]
        impl Step for BranchStep {
            fn name(&self) -> &str {
                "decide"
            }
            async fn execute(&self, _ctx: &mut AgentContext) -> Result<StepResult, AgentError> {
                Ok(StepResult::branch("skip_to_end", serde_json::json!("branched")))
            }
        }

        let engine = AgentEngine::default_engine();
        let plan = ExecutionPlan::new(vec![
            PlannedStep::new("decide", "Make a decision"),
            PlannedStep::new("unreachable", "Should not run"),
            PlannedStep::new("complete", "Finish"),
        ])
        .with_branch("skip_to_end", 2);

        let steps: Vec<Arc<dyn Step>> =
            vec![Arc::new(BranchStep), Arc::new(IncrementStep), Arc::new(CompleteStep)];

        let mut ctx = make_ctx();
        let result = engine.run(&plan, &steps, &mut ctx).await.unwrap();

        // decide → complete (skips unreachable)
        assert_eq!(result.steps_executed, 2);
    }
}
