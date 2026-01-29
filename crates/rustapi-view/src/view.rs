//! View response type

use crate::{Templates, ViewError};
use http::{header, Response, StatusCode};
use rustapi_core::{IntoResponse, ResponseBody};
use rustapi_openapi::{MediaType, Operation, ResponseModifier, ResponseSpec, SchemaRef};
use serde::Serialize;
use std::collections::BTreeMap;
use std::marker::PhantomData;

/// A response that renders a template with a context
///
/// This is the primary way to render HTML templates in RustAPI handlers.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_view::{View, Templates};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct HomeContext {
///     title: String,
/// }
///
/// async fn home(templates: State<Templates>) -> View<HomeContext> {
///     View::render(&templates, "home.html", HomeContext {
///         title: "Home".to_string(),
///     })
/// }
/// ```
pub struct View<T> {
    /// The rendered HTML content
    content: Result<String, ViewError>,
    /// Status code (default 200)
    status: StatusCode,
    /// Phantom data for the context type
    _phantom: PhantomData<T>,
}

impl<T: Serialize> View<T> {
    /// Create a view by rendering a template with a serializable context
    ///
    /// This is an async operation that renders the template immediately.
    /// For deferred rendering, use `View::deferred`.
    pub async fn render(templates: &Templates, template: &str, context: T) -> Self {
        let content = templates.render_with(template, &context).await;
        Self {
            content,
            status: StatusCode::OK,
            _phantom: PhantomData,
        }
    }

    /// Create a view with a specific status code
    pub async fn render_with_status(
        templates: &Templates,
        template: &str,
        context: T,
        status: StatusCode,
    ) -> Self {
        let content = templates.render_with(template, &context).await;
        Self {
            content,
            status,
            _phantom: PhantomData,
        }
    }

    /// Create a view from pre-rendered HTML
    pub fn from_html(html: impl Into<String>) -> Self {
        Self {
            content: Ok(html.into()),
            status: StatusCode::OK,
            _phantom: PhantomData,
        }
    }

    /// Create an error view
    pub fn error(err: ViewError) -> Self {
        Self {
            content: Err(err),
            status: StatusCode::INTERNAL_SERVER_ERROR,
            _phantom: PhantomData,
        }
    }

    /// Set the status code
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }
}

impl View<()> {
    /// Create a view by rendering a template with a tera Context
    pub async fn render_context(
        templates: &Templates,
        template: &str,
        context: &tera::Context,
    ) -> Self {
        let content = templates.render(template, context).await;
        Self {
            content,
            status: StatusCode::OK,
            _phantom: PhantomData,
        }
    }
}

impl<T> IntoResponse for View<T> {
    fn into_response(self) -> rustapi_core::Response {
        match self.content {
            Ok(html) => Response::builder()
                .status(self.status)
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(ResponseBody::from(html))
                .unwrap(),
            Err(err) => {
                tracing::error!("Template rendering failed: {}", err);
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                    .body(ResponseBody::from(
                        "<!DOCTYPE html><html><head><title>Error</title></head>\
                        <body><h1>500 Internal Server Error</h1>\
                        <p>Template rendering failed</p></body></html>",
                    ))
                    .unwrap()
            }
        }
    }
}

impl<T> ResponseModifier for View<T> {
    fn update_response(op: &mut Operation) {
        op.responses.insert(
            "200".to_string(),
            ResponseSpec {
                description: "HTML Content".to_string(),
                content: {
                    let mut map = BTreeMap::new();
                    map.insert(
                        "text/html".to_string(),
                        MediaType {
                            schema: Some(SchemaRef::Inline(serde_json::json!({ "type": "string" }))),
                            example: None,
                        },
                    );
                    map
                },
                headers: BTreeMap::new(),
            },
        );
    }
}

/// Helper for creating views with different status codes
impl<T: Serialize> View<T> {
    /// Create a 404 Not Found view
    pub async fn not_found(templates: &Templates, template: &str, context: T) -> Self {
        Self::render_with_status(templates, template, context, StatusCode::NOT_FOUND).await
    }

    /// Create a 403 Forbidden view
    pub async fn forbidden(templates: &Templates, template: &str, context: T) -> Self {
        Self::render_with_status(templates, template, context, StatusCode::FORBIDDEN).await
    }

    /// Create a 401 Unauthorized view
    pub async fn unauthorized(templates: &Templates, template: &str, context: T) -> Self {
        Self::render_with_status(templates, template, context, StatusCode::UNAUTHORIZED).await
    }
}
