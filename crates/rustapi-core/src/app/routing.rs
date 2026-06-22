use super::helpers::{add_path_params_to_operation, normalize_prefix_for_openapi};
use super::types::RustApi;
use crate::response::IntoResponse;
use crate::router::{MethodRouter, Router};
use std::collections::BTreeMap;

impl RustApi {
    pub(super) fn mount_auto_routes_grouped(mut self) -> Self {
        let routes = crate::auto_route::collect_auto_routes();

        if routes.is_empty() {
            // This is a common source of confusion with linkme-based auto registration.
            // We emit a clear warning so users know their annotated handlers were not linked.
            tracing::warn!(
                target: "rustapi::auto",
                count = 0,
                "RustApi::auto() collected 0 routes. \
                 This usually means either:\n\
                 - No handlers were annotated with #[rustapi_rs::get], #[post], etc.\n\
                 - The binary/test was not linked with the annotated modules (common in some test setups).\n\
                 - You are building a library (cdylib/rlib) where linkme distributed slices may not be populated.\n\n\
                 You can still register routes manually with .route() or check with rustapi_rs::auto_route_count()."
            );
        } else {
            #[cfg(feature = "tracing")]
            tracing::debug!(
                target: "rustapi::auto",
                count = routes.len(),
                "Auto route collection found handlers"
            );
        }

        // Use BTreeMap for deterministic route registration order
        let mut by_path: BTreeMap<String, MethodRouter> = BTreeMap::new();

        for route in routes {
            let crate::handler::Route {
                path: route_path,
                method,
                handler,
                operation,
                component_registrar,
                ..
            } = route;

            let method_enum = match method {
                "GET" => http::Method::GET,
                "POST" => http::Method::POST,
                "PUT" => http::Method::PUT,
                "DELETE" => http::Method::DELETE,
                "PATCH" => http::Method::PATCH,
                _ => http::Method::GET,
            };

            let path = if route_path.starts_with('/') {
                route_path.to_string()
            } else {
                format!("/{}", route_path)
            };

            let entry = by_path.entry(path).or_default();
            entry.insert_boxed_with_operation(method_enum, handler, operation, component_registrar);
        }

        #[cfg(feature = "tracing")]
        let route_count: usize = by_path.values().map(|mr| mr.allowed_methods().len()).sum();
        #[cfg(feature = "tracing")]
        let path_count = by_path.len();

        for (path, method_router) in by_path {
            self = self.route(&path, method_router);
        }

        crate::trace_info!(
            paths = path_count,
            routes = route_count,
            "Auto-registered routes"
        );

        // Apply any auto-registered schemas.
        crate::auto_schema::apply_auto_schemas(&mut self.openapi_spec);

        self
    }

    /// Add a route
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .route("/", get(index))
    ///     .route("/users", get(list_users).post(create_user))
    ///     .route("/users/{id}", get(get_user).delete(delete_user))
    /// ```
    pub fn route(mut self, path: &str, method_router: MethodRouter) -> Self {
        for register_components in &method_router.component_registrars {
            register_components(&mut self.openapi_spec);
        }

        // Register operations in OpenAPI spec
        for (method, op) in &method_router.operations {
            let mut op = op.clone();
            add_path_params_to_operation(path, &mut op, &BTreeMap::new());
            self.openapi_spec = self.openapi_spec.path(path, method.as_str(), op);
        }

        self.router = self.router.route(path, method_router);
        self
    }

    /// Add a typed route
    pub fn typed<P: crate::typed_path::TypedPath>(self, method_router: MethodRouter) -> Self {
        self.route(P::PATH, method_router)
    }

    /// Mount a handler (convenience method)
    ///
    /// Alias for `.route(path, method_router)` for a single handler.
    #[deprecated(note = "Use route() directly or mount_route() for macro-based routing")]
    pub fn mount(self, path: &str, method_router: MethodRouter) -> Self {
        self.route(path, method_router)
    }

    /// Mount a route created with `#[rustapi::get]`, `#[rustapi::post]`, etc.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_rs::prelude::*;
    ///
    /// #[rustapi::get("/users")]
    /// async fn list_users() -> Json<Vec<User>> {
    ///     Json(vec![])
    /// }
    ///
    /// RustApi::new()
    ///     .mount_route(route!(list_users))
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    pub fn mount_route(mut self, route: crate::handler::Route) -> Self {
        let method_enum = match route.method {
            "GET" => http::Method::GET,
            "POST" => http::Method::POST,
            "PUT" => http::Method::PUT,
            "DELETE" => http::Method::DELETE,
            "PATCH" => http::Method::PATCH,
            _ => http::Method::GET,
        };

        (route.component_registrar)(&mut self.openapi_spec);

        // Register operation in OpenAPI spec
        let mut op = route.operation;
        add_path_params_to_operation(route.path, &mut op, &route.param_schemas);
        self.openapi_spec = self.openapi_spec.path(route.path, route.method, op);

        self.route_with_method(route.path, method_enum, route.handler)
    }

    /// Helper to mount a single method handler
    fn route_with_method(
        self,
        path: &str,
        method: http::Method,
        handler: crate::handler::BoxedHandler,
    ) -> Self {
        use crate::router::MethodRouter;
        // use http::Method; // Removed

        // This is simplified. In a real implementation we'd merge with existing router at this path
        // For now we assume one handler per path or we simply allow overwriting for this MVP step
        // (matchit router doesn't allow easy merging/updating existing entries without rebuilding)
        //
        // TOOD: Enhance Router to support method merging

        let path = if !path.starts_with('/') {
            format!("/{}", path)
        } else {
            path.to_string()
        };

        // Check if we already have this path?
        // For MVP, valid assumption: user calls .route() or .mount() once per path-method-combo
        // But we need to handle multiple methods on same path.
        // Our Router wrapper currently just inserts.

        // Since we can't easily query matchit, we'll just insert.
        // Limitations: strictly sequential mounting for now.

        let mut handlers = std::collections::HashMap::new();
        handlers.insert(method, handler);

        let method_router = MethodRouter::from_boxed(handlers);
        self.route(&path, method_router)
    }

    /// Nest a router under a prefix
    ///
    /// All routes from the nested router will be registered with the prefix
    /// prepended to their paths. OpenAPI operations from the nested router
    /// are also propagated to the parent's OpenAPI spec with prefixed paths.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let api_v1 = Router::new()
    ///     .route("/users", get(list_users));
    ///
    /// RustApi::new()
    ///     .nest("/api/v1", api_v1)
    /// ```
    pub fn nest(mut self, prefix: &str, router: Router) -> Self {
        // Normalize the prefix for OpenAPI paths
        let normalized_prefix = normalize_prefix_for_openapi(prefix);

        // Propagate OpenAPI operations from nested router with prefixed paths
        // We need to do this before calling router.nest() because it consumes the router
        for (matchit_path, method_router) in router.method_routers() {
            for register_components in &method_router.component_registrars {
                register_components(&mut self.openapi_spec);
            }

            // Get the display path from registered_routes (has {param} format)
            let display_path = router
                .registered_routes()
                .get(matchit_path)
                .map(|info| info.path.clone())
                .unwrap_or_else(|| matchit_path.clone());

            // Build the prefixed display path for OpenAPI
            let prefixed_path = if display_path == "/" {
                normalized_prefix.clone()
            } else {
                format!("{}{}", normalized_prefix, display_path)
            };

            // Register each operation in the OpenAPI spec
            for (method, op) in &method_router.operations {
                let mut op = op.clone();
                add_path_params_to_operation(&prefixed_path, &mut op, &BTreeMap::new());
                self.openapi_spec = self.openapi_spec.path(&prefixed_path, method.as_str(), op);
            }
        }

        // Delegate to Router::nest for actual route registration
        self.router = self.router.nest(prefix, router);
        self
    }

    /// Serve static files from a directory
    ///
    /// Maps a URL path prefix to a filesystem directory. Requests to paths under
    /// the prefix will serve files from the corresponding location in the directory.
    ///
    /// # Arguments
    ///
    /// * `prefix` - URL path prefix (e.g., "/static", "/assets")
    /// * `root` - Filesystem directory path
    ///
    /// # Features
    ///
    /// - Automatic MIME type detection
    /// - ETag and Last-Modified headers for caching
    /// - Index file serving for directories
    /// - Path traversal prevention
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_rs::prelude::*;
    ///
    /// RustApi::new()
    ///     .serve_static("/assets", "./public")
    ///     .serve_static("/uploads", "./uploads")
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    pub fn serve_static(self, prefix: &str, root: impl Into<std::path::PathBuf>) -> Self {
        self.serve_static_with_config(crate::static_files::StaticFileConfig::new(root, prefix))
    }

    /// Serve static files with custom configuration
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_core::static_files::StaticFileConfig;
    ///
    /// let config = StaticFileConfig::new("./public", "/assets")
    ///     .max_age(86400)  // Cache for 1 day
    ///     .fallback("index.html");  // SPA fallback
    ///
    /// RustApi::new()
    ///     .serve_static_with_config(config)
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    pub fn serve_static_with_config(self, config: crate::static_files::StaticFileConfig) -> Self {
        use crate::router::MethodRouter;
        use std::collections::HashMap;

        let prefix = config.prefix.clone();
        let catch_all_path = format!("{}/*path", prefix.trim_end_matches('/'));

        // Create the static file handler
        let handler: crate::handler::BoxedHandler =
            std::sync::Arc::new(move |req: crate::Request| {
                let config = config.clone();
                let path = req.uri().path().to_string();

                Box::pin(async move {
                    let relative_path = path.strip_prefix(&config.prefix).unwrap_or(&path);

                    match crate::static_files::StaticFile::serve(relative_path, &config).await {
                        Ok(response) => response,
                        Err(err) => err.into_response(),
                    }
                })
                    as std::pin::Pin<Box<dyn std::future::Future<Output = crate::Response> + Send>>
            });

        let mut handlers = HashMap::new();
        handlers.insert(http::Method::GET, handler);
        let method_router = MethodRouter::from_boxed(handlers);

        self.route(&catch_all_path, method_router)
    }

    /// Enable response compression
    ///
    /// Adds gzip/deflate compression for response bodies. The compression
    /// is based on the client's Accept-Encoding header.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_rs::prelude::*;
    ///
    /// RustApi::new()
    ///     .compression()
    ///     .route("/", get(handler))
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    #[cfg(feature = "compression")]
    pub fn compression(self) -> Self {
        self.layer(crate::middleware::CompressionLayer::new())
    }

    /// Enable response compression with custom configuration
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_core::middleware::CompressionConfig;
    ///
    /// RustApi::new()
    ///     .compression_with_config(
    ///         CompressionConfig::new()
    ///             .min_size(512)
    ///             .level(9)
    ///     )
    ///     .route("/", get(handler))
    /// ```
    #[cfg(feature = "compression")]
    pub fn compression_with_config(self, config: crate::middleware::CompressionConfig) -> Self {
        self.layer(crate::middleware::CompressionLayer::with_config(config))
    }
}
