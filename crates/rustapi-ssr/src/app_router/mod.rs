use std::sync::OnceLock;

use matchit::Router;
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub struct PageRoute {
    pub pattern: &'static str,
    pub source: &'static str,
}

inventory::collect!(PageRoute);

include!(concat!(env!("OUT_DIR"), "/app_router_routes.rs"));

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("route registry build failed: {0}")]
    Router(#[from] matchit::InsertError),
}

#[derive(Debug)]
pub struct MatchedRoute<'a> {
    pub route: &'a PageRoute,
    pub params: Vec<(&'a str, String)>,
}

#[derive(Debug)]
pub struct RouteRegistry {
    router: Router<&'static PageRoute>,
}

impl RouteRegistry {
    pub fn new() -> Result<Self, RegistryError> {
        let mut router = Router::new();
        for route in inventory::iter::<PageRoute> {
            router.insert(route.pattern, route)?;
        }
        Ok(Self { router })
    }

    pub fn match_route(&self, path: &str) -> Option<MatchedRoute<'_>> {
        let matched = self.router.at(path).ok()?;
        let params = matched
            .params
            .iter()
            .map(|(key, value)| (key, value.to_string()))
            .collect();
        Some(MatchedRoute {
            route: matched.value,
            params,
        })
    }
}

pub fn registry() -> Result<&'static RouteRegistry, RegistryError> {
    static REGISTRY: OnceLock<RouteRegistry> = OnceLock::new();
    REGISTRY.get_or_try_init(RouteRegistry::new)
}

pub fn routes() -> impl Iterator<Item = &'static PageRoute> {
    inventory::iter::<PageRoute>
}
