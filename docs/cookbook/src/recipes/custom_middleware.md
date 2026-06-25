# Custom Middleware

**Problem**: You need to execute code before or after every request (e.g., logging, authentication, metrics) or modify the response.

## Solution

In RustAPI, the idiomatic way to implement custom middleware is by implementing the `MiddlewareLayer` trait. This trait provides a safe, asynchronous interface for inspecting and modifying requests and responses.

### The `MiddlewareLayer` Trait

The trait is defined in `rustapi_core::middleware`:

```rust,ignore
pub trait MiddlewareLayer: Send + Sync + 'static {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>>;

    fn clone_box(&self) -> Box<dyn MiddlewareLayer>;
}
```

### Basic Example: Logging Middleware

Here is a simple middleware that logs the incoming request method and URI, calls the next handler, and then logs the response status.

```rust
use rustapi_core::middleware::{MiddlewareLayer, BoxedNext};
use rustapi_core::{Request, Response};
use std::pin::Pin;
use std::future::Future;

#[derive(Clone)]
pub struct SimpleLogger;

impl MiddlewareLayer for SimpleLogger {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        // logic before handling request
        let method = req.method().clone();
        let uri = req.uri().clone();
        println!("Incoming: {} {}", method, uri);

        Box::pin(async move {
            // call the next middleware/handler
            let response = next(req).await;

            // logic after handling request
            println!("Completed: {} {} -> {}", method, uri, response.status());
            
            response
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}
```

### Applying Middleware

You can apply your custom middleware using `.layer()`:

```rust,ignore
RustApi::new()
    .layer(SimpleLogger)
    .route("/", get(handler))
    .run("127.0.0.1:8080")
    .await?;
```

## Advanced Patterns

### Configuration

You can pass configuration to your middleware struct.

```rust
#[derive(Clone)]
pub struct RateLimitLayer {
    max_requests: u32,
    window_secs: u64,
}

impl RateLimitLayer {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self { max_requests, window_secs }
    }
}

// impl MiddlewareLayer for RateLimitLayer ...
```

### Injecting State (Extensions)

Middleware can inject data into the request's extensions, which can then be retrieved by handlers (e.g., via `FromRequest` extractors).

```rust
// In your middleware
fn call(&self, mut req: Request, next: BoxedNext) -> ... {
    let user_id = "user_123".to_string();
    req.extensions_mut().insert(user_id);
    next(req)
}

// In your handler
async fn handler(req: Request) -> ... {
    let user_id = req.extensions().get::<String>().unwrap();
    // ...
}
```

### Short-Circuiting (Authentication)

If a request fails validation (e.g., invalid token), you can return a response immediately without calling `next(req)`.

```rust
fn call(&self, req: Request, next: BoxedNext) -> ... {
    if !is_authorized(&req) {
        return Box::pin(async {
            http::Response::builder()
                .status(401)
                .body("Unauthorized".into())
                .unwrap()
        });
    }
    
    next(req)
}
```

### Modifying the Response

You can inspect and modify the response returned by the handler.

```rust
let response = next(req).await;
let (mut parts, body) = response.into_parts();
parts.headers.insert("X-Custom-Header", "Value".parse().unwrap());
Response::from_parts(parts, body)
```

