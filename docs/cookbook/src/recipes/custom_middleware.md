# Custom Middleware

**Problem**: You need to execute code before or after every request (e.g., logging, metrics).

## Solution

Implement a `tower::Layer`.

```rust
#[derive(Clone)]
struct MyMiddleware<S> { inner: S }

impl<S, B> Service<Request<B>> for MyMiddleware<S>
where S: Service<Request<B>> {
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        println!("Request: {}", req.uri());
        self.inner.call(req)
    }
}
```

## Discussion

For simple cases, you can use `tower_http::TraceLayer` or `middleware::from_fn` instead of writing a full struct.
