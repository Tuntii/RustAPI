use crate::events::LifecycleHooks;
use crate::interceptor::InterceptorChain;
use crate::middleware::LayerStack;
use crate::router::Router;

pub struct RustApi {
    pub(super) router: Router,
    pub(super) openapi_spec: rustapi_openapi::OpenApiSpec,
    pub(super) layers: LayerStack,
    pub(super) body_limit: Option<usize>,
    pub(super) interceptors: InterceptorChain,
    pub(super) lifecycle_hooks: LifecycleHooks,
    pub(super) hot_reload: bool,
    #[cfg(feature = "http3")]
    pub(super) http3_config: Option<crate::http3::Http3Config>,
    pub(super) health_check: Option<crate::health::HealthCheck>,
    pub(super) health_endpoint_config: Option<crate::health::HealthEndpointConfig>,
    pub(super) status_config: Option<crate::status::StatusConfig>,
    #[cfg(feature = "dashboard")]
    pub(super) dashboard_config: Option<crate::dashboard::DashboardConfig>,
}
