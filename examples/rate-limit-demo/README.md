# Rate Limiting Demo

This example demonstrates RustAPI's built-in rate limiting capabilities.

## Features

- **IP-based rate limiting** — Automatic client identification
- **Per-endpoint configuration** — Different limits for different routes
- **Burst support** — Allow temporary spikes in traffic
- **Rate limit headers** — `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`
- **Graceful degradation** — 429 Too Many Requests with retry info

## Running

```bash
cargo run -p rate-limit-demo
```

Then visit:
- **Swagger UI**: http://127.0.0.1:8080/docs
- **Test endpoint**: http://127.0.0.1:8080/api/limited

## Testing

### Test strict rate limit (5 req/min):
```bash
for i in {1..10}; do
  curl -i http://127.0.0.1:8080/api/limited
  echo ""
done
```

You'll see:
- First 5 requests: **200 OK**
- Next 2 requests: **200 OK** (burst)
- Remaining requests: **429 Too Many Requests**

### Test relaxed rate limit (100 req/min):
```bash
for i in {1..50}; do
  curl http://127.0.0.1:8080/api/relaxed
done
```

All requests should succeed.

## Configuration

```rust
let config = RateLimitConfig {
    max_requests: 5,           // 5 requests per window
    window: Duration::from_secs(60),  // 1 minute window
    burst_size: 2,             // Allow 2 extra burst requests
};
```

## Response Headers

```
HTTP/1.1 200 OK
X-RateLimit-Limit: 5
X-RateLimit-Remaining: 3
X-RateLimit-Reset: 1704672000
```

## Production Tips

1. **Use Redis** — For distributed rate limiting across multiple servers
2. **User-based limits** — Rate limit by user ID instead of IP
3. **Tiered limits** — Different limits for free/premium users
4. **Adaptive limiting** — Adjust limits based on system load
