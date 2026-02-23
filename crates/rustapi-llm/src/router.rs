use crate::{CompletionRequest, CompletionResponse, LlmError, LlmProvider, ModelInfo, StreamChunk};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

// ---------------------------------------------------------------------------
// RoutingStrategy
// ---------------------------------------------------------------------------

/// Strategy the router uses to pick a provider + model for each request.
#[derive(Clone, Default)]
pub enum RoutingStrategy {
    /// Prefer the cheapest capable model.
    #[default]
    CostOptimized,
    /// Prefer the model with lowest expected latency.
    LatencyOptimized,
    /// Always use the highest-quality model available.
    QualityFirst,
    /// Round-robin across healthy providers.
    RoundRobin,
    /// User-supplied scoring function.
    Custom(Arc<dyn ProviderScorer>),
}

/// Trait for custom routing logic.
pub trait ProviderScorer: Send + Sync + 'static {
    /// Score a provider+model for a given request. Higher is better.
    fn score(&self, provider: &str, model: &ModelInfo, request: &CompletionRequest) -> f64;
}

impl std::fmt::Debug for RoutingStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CostOptimized => write!(f, "CostOptimized"),
            Self::LatencyOptimized => write!(f, "LatencyOptimized"),
            Self::QualityFirst => write!(f, "QualityFirst"),
            Self::RoundRobin => write!(f, "RoundRobin"),
            Self::Custom(_) => write!(f, "Custom(...)"),
        }
    }
}

// ---------------------------------------------------------------------------
// ProviderHealth – per-provider circuit breaker state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ProviderHealth {
    consecutive_failures: u32,
    last_failure: Option<std::time::Instant>,
    /// Provider is considered "open" (broken) after this many failures.
    failure_threshold: u32,
    /// Auto-recover after this duration.
    recovery_timeout: std::time::Duration,
}

impl Default for ProviderHealth {
    fn default() -> Self {
        Self {
            consecutive_failures: 0,
            last_failure: None,
            failure_threshold: 3,
            recovery_timeout: std::time::Duration::from_secs(30),
        }
    }
}

impl ProviderHealth {
    fn is_healthy(&self) -> bool {
        if self.consecutive_failures < self.failure_threshold {
            return true;
        }
        // Check if enough time has passed for a retry
        if let Some(last) = self.last_failure {
            last.elapsed() >= self.recovery_timeout
        } else {
            true
        }
    }

    fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.last_failure = None;
    }

    fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        self.last_failure = Some(std::time::Instant::now());
    }
}

// ---------------------------------------------------------------------------
// LlmRouter
// ---------------------------------------------------------------------------

/// Cost-aware, fallback-capable model router.
///
/// The router holds one or more [`LlmProvider`]s and dispatches requests
/// based on the configured [`RoutingStrategy`]. It includes per-provider
/// circuit breakers and automatic fallback on failure.
///
/// # Example
/// ```rust,no_run
/// use rustapi_llm::{LlmRouter, RoutingStrategy, MockProvider};
///
/// let router = LlmRouter::builder()
///     .strategy(RoutingStrategy::CostOptimized)
///     .provider(MockProvider::new("primary"))
///     .provider(MockProvider::new("fallback"))
///     .build();
/// ```
pub struct LlmRouter {
    /// Registered providers (insertion order = priority for fallback).
    providers: Vec<(String, Arc<dyn LlmProvider>)>,
    /// Per-provider health tracking (keyed by provider name).
    health: Arc<RwLock<HashMap<String, ProviderHealth>>>,
    /// Routing strategy.
    strategy: RoutingStrategy,
    /// Default model override.
    default_model: Option<String>,
    /// Maximum retries across the fallback chain.
    max_retries: u32,
}

impl LlmRouter {
    /// Create a builder for configuring the router.
    pub fn builder() -> LlmRouterBuilder {
        LlmRouterBuilder::default()
    }

    /// Send a completion request through the routing/fallback chain.
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let candidates = self.rank_providers(&request).await;

        if candidates.is_empty() {
            return Err(LlmError::all_failed("No healthy providers available"));
        }

        let mut last_error = None;

        for (attempts, (name, provider)) in candidates.iter().enumerate() {
            if attempts as u32 >= self.max_retries {
                break;
            }

            let mut req = request.clone();
            // Apply default model if request doesn't specify one
            if req.model.is_none() {
                if let Some(ref default) = self.default_model {
                    req.model = Some(default.clone());
                }
            }

            debug!(provider = %name, model = ?req.model, "Routing LLM request");

            match provider.complete(req).await {
                Ok(response) => {
                    self.record_success(name).await;
                    return Ok(response);
                }
                Err(e) => {
                    warn!(provider = %name, error = %e, "Provider failed, trying fallback");
                    self.record_failure(name).await;
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| LlmError::all_failed("All providers exhausted")))
    }

    /// Stream a completion through the routing/fallback chain.
    pub async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn futures_util::Stream<Item = Result<StreamChunk, LlmError>> + Send>>, LlmError>
    {
        let candidates = self.rank_providers(&request).await;

        if candidates.is_empty() {
            return Err(LlmError::all_failed("No healthy providers available"));
        }

        let mut last_error = None;

        for (name, provider) in &candidates {
            let mut req = request.clone();
            if req.model.is_none() {
                if let Some(ref default) = self.default_model {
                    req.model = Some(default.clone());
                }
            }

            debug!(provider = %name, "Routing streaming LLM request");

            match provider.complete_stream(req).await {
                Ok(stream) => {
                    // We can't easily record_success for streaming until it completes,
                    // so we optimistically mark success on stream creation.
                    self.record_success(name).await;
                    return Ok(stream);
                }
                Err(e) => {
                    warn!(provider = %name, error = %e, "Provider stream failed, trying fallback");
                    self.record_failure(name).await;
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| LlmError::all_failed("All providers exhausted")))
    }

    /// List all available models across all providers.
    pub fn available_models(&self) -> Vec<ModelInfo> {
        self.providers
            .iter()
            .flat_map(|(_, p)| p.available_models())
            .collect()
    }

    /// Check if any provider is healthy.
    pub async fn has_healthy_provider(&self) -> bool {
        let health = self.health.read().await;
        self.providers
            .iter()
            .any(|(name, _)| health.get(name).map_or(true, |h| h.is_healthy()))
    }

    // ---- internal ----

    async fn rank_providers(
        &self,
        request: &CompletionRequest,
    ) -> Vec<(String, Arc<dyn LlmProvider>)> {
        let health = self.health.read().await;

        let mut candidates: Vec<_> = self
            .providers
            .iter()
            .filter(|(name, _)| health.get(name).map_or(true, |h| h.is_healthy()))
            .cloned()
            .collect();

        match &self.strategy {
            RoutingStrategy::CostOptimized => {
                candidates.sort_by(|(_, a), (_, b)| {
                    let a_cost = a.available_models().first().map_or(f64::MAX, |m| m.cost_per_m_input);
                    let b_cost = b.available_models().first().map_or(f64::MAX, |m| m.cost_per_m_input);
                    a_cost.partial_cmp(&b_cost).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            RoutingStrategy::QualityFirst => {
                // Keep insertion order — first registered is highest quality.
            }
            RoutingStrategy::RoundRobin => {
                // For simplicity rotate by current second modulo.
                let offset = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as usize;
                if !candidates.is_empty() {
                    let len = candidates.len();
                    candidates.rotate_left(offset % len);
                }
            }
            RoutingStrategy::LatencyOptimized => {
                // Prefer providers with fewer recent failures (heuristic).
                candidates.sort_by(|(a_name, _), (b_name, _)| {
                    let a_fail = health.get(a_name).map_or(0, |h| h.consecutive_failures);
                    let b_fail = health.get(b_name).map_or(0, |h| h.consecutive_failures);
                    a_fail.cmp(&b_fail)
                });
            }
            RoutingStrategy::Custom(scorer) => {
                candidates.sort_by(|(a_name, a_prov), (b_name, b_prov)| {
                    let a_model = a_prov.available_models();
                    let b_model = b_prov.available_models();
                    let a_score = a_model.first().map_or(0.0, |m| scorer.score(a_name, m, request));
                    let b_score = b_model.first().map_or(0.0, |m| scorer.score(b_name, m, request));
                    b_score.partial_cmp(&a_score).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        candidates
    }

    async fn record_success(&self, provider: &str) {
        let mut health = self.health.write().await;
        health.entry(provider.to_string()).or_default().record_success();
    }

    async fn record_failure(&self, provider: &str) {
        let mut health = self.health.write().await;
        health.entry(provider.to_string()).or_default().record_failure();
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for [`LlmRouter`].
#[derive(Default)]
pub struct LlmRouterBuilder {
    providers: Vec<(String, Arc<dyn LlmProvider>)>,
    strategy: RoutingStrategy,
    default_model: Option<String>,
    max_retries: Option<u32>,
}

impl LlmRouterBuilder {
    /// Set the routing strategy.
    pub fn strategy(mut self, strategy: RoutingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Register a provider.
    pub fn provider(mut self, provider: impl LlmProvider) -> Self {
        let name = provider.name().to_string();
        self.providers.push((name, Arc::new(provider)));
        self
    }

    /// Register a provider behind an `Arc`.
    pub fn provider_arc(mut self, provider: Arc<dyn LlmProvider>) -> Self {
        let name = provider.name().to_string();
        self.providers.push((name, provider));
        self
    }

    /// Set a default model to use when the request doesn't specify one.
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }

    /// Maximum retry attempts across providers (default: number of providers).
    pub fn max_retries(mut self, n: u32) -> Self {
        self.max_retries = Some(n);
        self
    }

    /// Build the router.
    pub fn build(self) -> LlmRouter {
        let max_retries = self.max_retries.unwrap_or(self.providers.len() as u32);
        LlmRouter {
            providers: self.providers,
            health: Arc::new(RwLock::new(HashMap::new())),
            strategy: self.strategy,
            default_model: self.default_model,
            max_retries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockProvider;

    #[tokio::test]
    async fn test_router_routes_to_first_healthy() {
        let router = LlmRouter::builder()
            .provider(MockProvider::new("primary").with_default_content("from-primary"))
            .provider(MockProvider::new("fallback").with_default_content("from-fallback"))
            .build();

        let resp = router.complete(CompletionRequest::simple("hi")).await.unwrap();
        assert_eq!(resp.content, "from-primary");
    }

    #[tokio::test]
    async fn test_router_no_providers_fails() {
        let router = LlmRouter::builder().build();
        let err = router.complete(CompletionRequest::simple("hi")).await.unwrap_err();
        assert!(matches!(err, LlmError::AllProvidersFailed { .. }));
    }

    #[tokio::test]
    async fn test_router_has_healthy_provider() {
        let router = LlmRouter::builder()
            .provider(MockProvider::new("p"))
            .build();
        assert!(router.has_healthy_provider().await);
    }

    #[tokio::test]
    async fn test_router_available_models() {
        let router = LlmRouter::builder()
            .provider(MockProvider::new("a"))
            .provider(MockProvider::new("b"))
            .build();
        assert_eq!(router.available_models().len(), 2);
    }
}
