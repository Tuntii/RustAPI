use crate::ContextError;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// CostBudget — per-request limits
// ---------------------------------------------------------------------------

/// Budget limits for a single AI execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBudget {
    /// Maximum total tokens (input + output) allowed.
    pub max_tokens: Option<u64>,
    /// Maximum cost in **micro-USD** (1 USD = 1_000_000).
    pub max_cost_micros: Option<u64>,
    /// Maximum number of LLM / tool API calls.
    pub max_api_calls: Option<u32>,
}

impl CostBudget {
    /// No limits.
    pub fn unlimited() -> Self {
        Self {
            max_tokens: None,
            max_cost_micros: None,
            max_api_calls: None,
        }
    }

    /// Limit by USD amount (converted to micro-USD internally).
    pub fn per_request_usd(usd: f64) -> Self {
        Self {
            max_tokens: None,
            max_cost_micros: Some((usd * 1_000_000.0) as u64),
            max_api_calls: None,
        }
    }

    /// Limit by total tokens.
    pub fn per_request_tokens(tokens: u64) -> Self {
        Self {
            max_tokens: Some(tokens),
            max_cost_micros: None,
            max_api_calls: None,
        }
    }

    /// Builder: set max tokens.
    pub fn with_max_tokens(mut self, tokens: u64) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Builder: set max cost in USD.
    pub fn with_max_cost_usd(mut self, usd: f64) -> Self {
        self.max_cost_micros = Some((usd * 1_000_000.0) as u64);
        self
    }

    /// Builder: set max API calls.
    pub fn with_max_api_calls(mut self, calls: u32) -> Self {
        self.max_api_calls = Some(calls);
        self
    }
}

impl Default for CostBudget {
    fn default() -> Self {
        Self::unlimited()
    }
}

// ---------------------------------------------------------------------------
// CostDelta — incremental cost report from a single operation
// ---------------------------------------------------------------------------

/// Incremental cost produced by a single LLM call or tool execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostDelta {
    /// Input tokens consumed.
    pub input_tokens: u64,
    /// Output tokens produced.
    pub output_tokens: u64,
    /// Cost in micro-USD for this operation.
    pub cost_micros: u64,
    /// Optional model identifier (e.g. "gpt-4o", "claude-sonnet-4-20250514").
    pub model: Option<String>,
}

// ---------------------------------------------------------------------------
// CostTracker — lock-free atomic accounting
// ---------------------------------------------------------------------------

/// Lock-free, thread-safe cost accounting for a request's lifetime.
///
/// All counters use relaxed atomic ordering — suitable for accounting
/// where exact cross-thread ordering is not required.
#[derive(Debug)]
pub struct CostTracker {
    input_tokens: AtomicU64,
    output_tokens: AtomicU64,
    total_cost_micros: AtomicU64,
    api_calls: AtomicU32,
    budget: Option<CostBudget>,
}

impl CostTracker {
    /// Create a tracker with no budget limits.
    pub fn new() -> Self {
        Self {
            input_tokens: AtomicU64::new(0),
            output_tokens: AtomicU64::new(0),
            total_cost_micros: AtomicU64::new(0),
            api_calls: AtomicU32::new(0),
            budget: None,
        }
    }

    /// Create a tracker with a budget.
    pub fn with_budget(budget: CostBudget) -> Self {
        Self {
            input_tokens: AtomicU64::new(0),
            output_tokens: AtomicU64::new(0),
            total_cost_micros: AtomicU64::new(0),
            api_calls: AtomicU32::new(0),
            budget: Some(budget),
        }
    }

    /// Record a cost delta and check budget.
    ///
    /// Returns `Err(ContextError::BudgetExceeded)` if any limit *would be*
    /// breached by this delta.  The delta is only applied when all checks
    /// pass, so a rejected call never modifies the running totals.
    ///
    /// # Concurrency note
    /// Budget enforcement uses relaxed atomic reads followed by atomic
    /// increments, so a TOCTOU race is theoretically possible under very high
    /// concurrent load.  The design is intentional: the lock-free accounting
    /// keeps overhead minimal and a slight overshoot under extreme concurrency
    /// is acceptable.  For hard limits, callers should pair this with an
    /// external quota gate.
    pub fn record(&self, delta: &CostDelta) -> Result<(), ContextError> {
        // Pre-check: verify that adding this delta will not breach the budget
        // *before* touching any counters.
        if let Some(ref budget) = self.budget {
            let new_tokens =
                self.total_tokens() + delta.input_tokens + delta.output_tokens;
            if let Some(max) = budget.max_tokens {
                if new_tokens > max {
                    return Err(ContextError::budget_exceeded(format!(
                        "Token limit {max} would be exceeded \
                         (current {}, delta {})",
                        self.total_tokens(),
                        delta.input_tokens + delta.output_tokens
                    )));
                }
            }
            let new_cost = self.total_cost_micros() + delta.cost_micros;
            if let Some(max) = budget.max_cost_micros {
                if new_cost > max {
                    return Err(ContextError::budget_exceeded(format!(
                        "Cost limit ${:.4} would be exceeded \
                         (current ${:.4}, delta ${:.4})",
                        max as f64 / 1_000_000.0,
                        self.total_cost_micros() as f64 / 1_000_000.0,
                        delta.cost_micros as f64 / 1_000_000.0
                    )));
                }
            }
            let new_calls = u64::from(self.api_calls()) + 1;
            if let Some(max) = budget.max_api_calls {
                if new_calls > u64::from(max) {
                    return Err(ContextError::budget_exceeded(format!(
                        "API call limit {max} would be exceeded ({new_calls} calls)"
                    )));
                }
            }
        }

        self.input_tokens
            .fetch_add(delta.input_tokens, Ordering::Relaxed);
        self.output_tokens
            .fetch_add(delta.output_tokens, Ordering::Relaxed);
        self.total_cost_micros
            .fetch_add(delta.cost_micros, Ordering::Relaxed);
        self.api_calls.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Check whether the current totals exceed the budget.
    pub fn check_budget(&self) -> Result<(), ContextError> {
        if let Some(ref budget) = self.budget {
            let total_tokens = self.total_tokens();
            if let Some(max) = budget.max_tokens {
                if total_tokens > max {
                    return Err(ContextError::budget_exceeded(format!(
                        "Token limit {max} exceeded (used {total_tokens})"
                    )));
                }
            }
            let cost = self.total_cost_micros();
            if let Some(max) = budget.max_cost_micros {
                if cost > max {
                    return Err(ContextError::budget_exceeded(format!(
                        "Cost limit ${:.4} exceeded (spent ${:.4})",
                        max as f64 / 1_000_000.0,
                        cost as f64 / 1_000_000.0
                    )));
                }
            }
            let calls = self.api_calls();
            if let Some(max) = budget.max_api_calls {
                if calls > max {
                    return Err(ContextError::budget_exceeded(format!(
                        "API call limit {max} exceeded ({calls} calls)"
                    )));
                }
            }
        }
        Ok(())
    }

    // -- Accessors --

    pub fn input_tokens(&self) -> u64 {
        self.input_tokens.load(Ordering::Relaxed)
    }

    pub fn output_tokens(&self) -> u64 {
        self.output_tokens.load(Ordering::Relaxed)
    }

    pub fn total_tokens(&self) -> u64 {
        self.input_tokens() + self.output_tokens()
    }

    pub fn total_cost_micros(&self) -> u64 {
        self.total_cost_micros.load(Ordering::Relaxed)
    }

    pub fn total_cost_usd(&self) -> f64 {
        self.total_cost_micros() as f64 / 1_000_000.0
    }

    pub fn api_calls(&self) -> u32 {
        self.api_calls.load(Ordering::Relaxed)
    }

    pub fn budget(&self) -> Option<&CostBudget> {
        self.budget.as_ref()
    }

    /// Produce a serialisable snapshot of the current state.
    pub fn snapshot(&self) -> CostSnapshot {
        CostSnapshot {
            input_tokens: self.input_tokens(),
            output_tokens: self.output_tokens(),
            total_tokens: self.total_tokens(),
            total_cost_micros: self.total_cost_micros(),
            total_cost_usd: self.total_cost_usd(),
            api_calls: self.api_calls(),
        }
    }
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Serialisable point-in-time cost snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSnapshot {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub total_cost_micros: u64,
    pub total_cost_usd: f64,
    pub api_calls: u32,
}

/// Wrap `CostTracker` in an `Arc` for cheap cloning across tasks.
pub type SharedCostTracker = Arc<CostTracker>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_tracker_unlimited() {
        let tracker = CostTracker::new();
        let delta = CostDelta {
            input_tokens: 100,
            output_tokens: 50,
            cost_micros: 300,
            model: Some("gpt-4o".into()),
        };
        assert!(tracker.record(&delta).is_ok());
        assert_eq!(tracker.total_tokens(), 150);
        assert_eq!(tracker.total_cost_micros(), 300);
        assert_eq!(tracker.api_calls(), 1);
    }

    #[test]
    fn test_cost_tracker_budget_exceeded() {
        let budget = CostBudget::per_request_tokens(100);
        let tracker = CostTracker::with_budget(budget);
        let delta = CostDelta {
            input_tokens: 80,
            output_tokens: 30,
            cost_micros: 200,
            model: None,
        };
        let result = tracker.record(&delta);
        assert!(result.is_err());
        match result.unwrap_err() {
            ContextError::BudgetExceeded { .. } => {}
            other => panic!("Expected BudgetExceeded, got {other:?}"),
        }
    }

    #[test]
    fn test_cost_budget_usd() {
        let budget = CostBudget::per_request_usd(0.05);
        assert_eq!(budget.max_cost_micros, Some(50_000));
    }

    #[test]
    fn test_cost_snapshot() {
        let tracker = CostTracker::new();
        tracker
            .record(&CostDelta {
                input_tokens: 200,
                output_tokens: 100,
                cost_micros: 1500,
                model: None,
            })
            .unwrap();
        let snap = tracker.snapshot();
        assert_eq!(snap.total_tokens, 300);
        assert_eq!(snap.total_cost_micros, 1500);
    }
}
