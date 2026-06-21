# Issue 7: [Rate Limiting] IP-Based Rate Limiting Does Not Account for Reverse Proxy Headers

## Description

The `RateLimiter` in `src/rate_limiting/limiter.rs` identifies clients by their connection IP address (`std::net::SocketAddr`), which is obtained from the TCP connection. When the application runs behind a reverse proxy (e.g., Nginx, AWS ALB, or Kubernetes Ingress), the connection IP is always the proxy's IP address, not the actual end-user IP. This means that all traffic from different users behind the same proxy is rate-limited as if it came from a single client, causing legitimate users to be unfairly blocked. The `rate_limiting/middleware.rs` and its Axum middleware integration do not inspect `X-Forwarded-For` or `X-Real-IP` headers, nor do they allow configuration of trusted proxy CIDR ranges. This makes the rate-limiting feature ineffective for production deployments behind any proxy infrastructure.

## Acceptance Criteria

- [ ] Add a `trusted_proxies` configuration field to `RateLimitConfig` accepting a list of CIDR ranges (e.g., `["10.0.0.0/8", "172.16.0.0/12"]`)
- [ ] Implement `X-Forwarded-For` header parsing that uses the rightmost untrusted IP (per common reverse proxy conventions)
- [ ] Add `X-Real-IP` header support as a fallback when `X-Forwarded-For` is not present
- [ ] Update the middleware to use the resolved end-user IP for rate limit key calculation
- [ ] Log the resolved IP and the original connection IP for debugging purposes
- [ ] Write tests in `src/rate_limiting/tests.rs` that simulate requests through multiple proxy layers

## Additional Context

Key files: `src/rate_limiting/limiter.rs`, `src/rate_limiting/middleware.rs`, `src/rate_limiting/config.rs`, `src/rate_limiting/types.rs`, `src/rate_limiting/tests.rs`.
