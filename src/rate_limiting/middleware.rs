//! HTTP middleware for rate limiting

use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;
use crate::rate_limiting::{RateLimiter, RateLimitContext, RateLimitResult, RateLimitTier};

use tracing::{debug, warn, error};

/// HTTP middleware trait for rate limiting
#[async_trait]
pub trait RateLimitMiddleware: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    type Request;
    type Response;

    async fn handle_request(&self, request: Self::Request) -> Result<Self::Response, RateLimitError<Self::Error>>;
}

/// Rate limiting middleware for HTTP requests
pub struct HttpRateLimitMiddleware {
    limiter: RateLimiter,
    config: MiddlewareConfig,
}

/// Result of IP resolution through proxy headers
#[derive(Debug, Clone)]
pub struct ResolvedIp {
    /// The resolved client IP address
    pub client_ip: IpAddr,
    /// The direct TCP connection IP
    pub connection_ip: IpAddr,
    /// The source used for resolution ("X-Forwarded-For", "X-Real-IP", "direct")
    pub resolution_source: String,
}

impl std::fmt::Display for ResolvedIp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (resolved via {}, connection: {})",
            self.client_ip, self.resolution_source, self.connection_ip
        )
    }
}

/// Configuration for the middleware
#[derive(Debug, Clone)]
pub struct MiddlewareConfig {
    /// Header to extract user ID from
    pub user_id_header: Option<String>,
    /// Header to extract user tier from
    pub user_tier_header: Option<String>,
    /// Header to extract API key from
    pub api_key_header: Option<String>,
    /// Header to extract real IP from (when behind proxy)
    pub real_ip_header: Option<String>,
    /// List of trusted proxy IPs (exact match)
    pub trusted_proxies: Vec<IpAddr>,
    /// Trusted proxy CIDR ranges (e.g. 10.0.0.0/8, 172.16.0.0/12)
    pub trusted_proxy_ranges: Vec<IpNetwork>,
    /// Whether to include rate limit headers in responses
    pub include_rate_limit_headers: bool,
    /// Whether to log IP resolution details
    pub log_ip_resolution: bool,
    /// Custom context extractor
    pub context_extractor: Option<Box<dyn ContextExtractor>>,
}

impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            user_id_header: Some("X-User-ID".to_string()),
            user_tier_header: Some("X-User-Tier".to_string()),
            api_key_header: Some("X-API-Key".to_string()),
            real_ip_header: Some("X-Real-IP".to_string()),
            trusted_proxies: Vec::new(),
            trusted_proxy_ranges: Vec::new(),
            include_rate_limit_headers: true,
            log_ip_resolution: true,
            context_extractor: None,
        }
    }
}

impl MiddlewareConfig {
    /// Create a MiddlewareConfig from a RateLimitConfig, propagating trusted proxy settings.
    pub fn from_rate_limit_config(config: &RateLimitConfig) -> Self {
        let ip_restrictions = &config.ip_restrictions;
        Self {
            trusted_proxies: ip_restrictions.trusted_proxies.clone(),
            trusted_proxy_ranges: ip_restrictions.trusted_proxy_ranges
                .iter()
                .filter_map(|range| range.parse::<IpNetwork>().ok())
                .collect(),
            log_ip_resolution: true,
            ..Self::default()
        }
    }
}

/// Custom context extractor trait
#[async_trait]
pub trait ContextExtractor: Send + Sync {
    async fn extract_context(&self, request: &dyn std::any::Any) -> Option<RateLimitContext>;
}

impl HttpRateLimitMiddleware {
    pub fn new(limiter: RateLimiter, config: MiddlewareConfig) -> Self {
        Self { limiter, config }
    }

    /// Resolve the true client IP address considering reverse proxy headers.
    /// Uses the following priority:
    /// 1. X-Forwarded-For header (rightmost untrusted IP)
    /// 2. X-Real-IP header (if set by a trusted proxy)
    /// 3. Direct connection IP (fallback)
    fn resolve_client_ip(&self, request: &dyn HttpRequest) -> ResolvedIp {
        let connection_ip = request.remote_addr();

        // Try X-Forwarded-For first (most common behind proxies)
        if let Some(forwarded_for) = request.get_header("X-Forwarded-For") {
            if let Some(resolved) = self.parse_forwarded_for(forwarded_for, connection_ip) {
                if self.config.log_ip_resolution {
                    debug!(
                        "IP resolved via X-Forwarded-For: {} (connection: {}, chain: {})",
                        resolved, connection_ip, forwarded_for
                    );
                }
                return ResolvedIp {
                    client_ip: resolved,
                    connection_ip,
                    resolution_source: "X-Forwarded-For".to_string(),
                };
            }
        }

        // Fall back to X-Real-IP header
        if let Some(header_name) = &self.config.real_ip_header {
            if let Some(ip_str) = request.get_header(header_name) {
                if let Ok(parsed_ip) = ip_str.parse::<IpAddr>() {
                    // Only trust X-Real-IP if it came from a trusted proxy
                    if self.is_trusted_proxy(connection_ip) {
                        if self.config.log_ip_resolution {
                            debug!(
                                "IP resolved via X-Real-IP: {} (connection: {})",
                                parsed_ip, connection_ip
                            );
                        }
                        return ResolvedIp {
                            client_ip: parsed_ip,
                            connection_ip,
                            resolution_source: "X-Real-IP".to_string(),
                        };
                    }
                }
            }
        }

        // Fall back to direct connection IP
        if self.config.log_ip_resolution {
            debug!(
                "Using direct connection IP: {} (no proxy headers)",
                connection_ip
            );
        }
        ResolvedIp {
            client_ip: connection_ip,
            connection_ip,
            resolution_source: "direct".to_string(),
        }
    }

    /// Parse X-Forwarded-For header to find the rightmost untrusted IP.
    /// X-Forwarded-For format: "<client>, <proxy1>, <proxy2>"
    /// The rightmost IP that is NOT a trusted proxy is the real client.
    fn parse_forwarded_for(&self, header_value: &str, connection_ip: IpAddr) -> Option<IpAddr> {
        // Split by comma and process from right to left
        let ips: Vec<&str> = header_value.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        
        if ips.is_empty() {
            return None;
        }

        // Process from right to left: first rightmost IP that is not a trusted proxy is the client
        // If all IPs in the chain are trusted proxies, use the leftmost (which represents the original client)

        // First check: is the connecting IP a trusted proxy?
        // If not, the X-Forwarded-For header may be spoofed — use connection IP
        if !self.is_trusted_proxy(connection_ip) {
            // The request didn't come from a trusted proxy, so we can't trust the header
            warn!(
                "X-Forwarded-For header present but connection IP {} is not a trusted proxy — header may be spoofed",
                connection_ip
            );
            return None;
        }

        // Process right to left: find the first non-trusted IP
        for ip_str in ips.iter().rev() {
            if let Ok(ip) = ip_str.parse::<IpAddr>() {
                if !self.is_trusted_proxy(ip) {
                    return Some(ip); // First untrusted IP from right is the client
                }
            } else {
                // Invalid IP in chain — stop processing
                warn!("Invalid IP in X-Forwarded-For chain: {}", ip_str);
                return None;
            }
        }

        // All IPs in the chain are trusted proxies — the leftmost is the original client
        // This happens when there are more proxies than expected (e.g., internal networks)
        if let Some(leftmost_str) = ips.first() {
            if let Ok(ip) = leftmost_str.trim().parse::<IpAddr>() {
                debug!("All proxies in X-Forwarded-For chain are trusted, using leftmost: {}", ip);
                return Some(ip);
            }
        }

        None
    }

    /// Extract client IP address from request (legacy compatibility wrapper)
    fn extract_client_ip(&self, request: &dyn HttpRequest) -> IpAddr {
        self.resolve_client_ip(request).client_ip
    }

    /// Check if an IP is a trusted proxy (exact match or CIDR range)
    fn is_trusted_proxy(&self, ip: IpAddr) -> bool {
        // Check exact match first
        if self.config.trusted_proxies.contains(&ip) {
            return true;
        }

        // Check CIDR ranges
        for range in &self.config.trusted_proxy_ranges {
            if range.contains(ip) {
                return true;
            }
        }

        false
    }

    /// Extract user information from request
    fn extract_user_info(&self, request: &dyn HttpRequest) -> (Option<Uuid>, RateLimitTier) {
        let user_id = self.config.user_id_header
            .as_ref()
            .and_then(|header| request.get_header(header))
            .and_then(|id_str| Uuid::parse_str(id_str).ok());

        let tier = self.config.user_tier_header
            .as_ref()
            .and_then(|header| request.get_header(header))
            .map(|tier_str| RateLimitTier::from_role(tier_str))
            .unwrap_or(if user_id.is_some() {
                RateLimitTier::Basic
            } else {
                RateLimitTier::Unauthenticated
            });

        (user_id, tier)
    }

    /// Extract API key from request
    fn extract_api_key(&self, request: &dyn HttpRequest) -> Option<String> {
        self.config.api_key_header
            .as_ref()
            .and_then(|header| request.get_header(header))
            .map(|s| s.to_string())
    }

    /// Create rate limit context from HTTP request
    fn create_context(&self, request: &dyn HttpRequest) -> RateLimitContext {
        let resolved = self.resolve_client_ip(request);
        let (user_id, tier) = self.extract_user_info(request);
        let api_key = self.extract_api_key(request);
        let user_agent = request.get_header("User-Agent").map(|s| s.to_string());

        let mut context = RateLimitContext::new(
            resolved.client_ip,
            request.path().to_string(),
            request.method().to_string(),
        )
        .with_user_id(user_id.unwrap_or_else(Uuid::new_v4))
        .with_tier(tier)
        .with_user_agent(user_agent.unwrap_or_default())
        .with_api_key(api_key.unwrap_or_default());

        // Add IP resolution metadata for debugging and auditing
        context = context
            .with_metadata("connection_ip".to_string(), resolved.connection_ip.to_string())
            .with_metadata("ip_resolution_source".to_string(), resolved.resolution_source.clone())
            .with_metadata("resolved_client_ip".to_string(), resolved.client_ip.to_string());

        if let Some(forwarded_for) = request.get_header("X-Forwarded-For") {
            context = context.with_metadata("forwarded_for_chain".to_string(), forwarded_for.to_string());
        }

        if let Some(referer) = request.get_header("Referer") {
            context = context.with_metadata("referer".to_string(), referer.to_string());
        }

        context
    }

    /// Apply rate limit headers to response
    fn apply_rate_limit_headers(&self, response: &mut dyn HttpResponse, result: &RateLimitResult) {
        if !self.config.include_rate_limit_headers {
            return;
        }

        match result {
            RateLimitResult::Allowed { remaining, reset_time, .. } => {
                response.set_header("X-RateLimit-Remaining", remaining.to_string());
                response.set_header("X-RateLimit-Reset", reset_time.timestamp().to_string());
            }
            RateLimitResult::Blocked { retry_after, max_requests, .. } => {
                response.set_header("X-RateLimit-Limit", max_requests.to_string());
                response.set_header("Retry-After", retry_after.as_secs().to_string());
            }
        }
    }
}

/// Generic HTTP request trait
pub trait HttpRequest {
    fn get_header(&self, name: &str) -> Option<&str>;
    fn path(&self) -> &str;
    fn method(&self) -> &str;
    fn remote_addr(&self) -> IpAddr;
}

/// Generic HTTP response trait
pub trait HttpResponse {
    fn set_header(&mut self, name: &str, value: String);
    fn set_status(&mut self, status: u16);
    fn set_body(&mut self, body: String);
}

/// Rate limit error types
#[derive(Debug, thiserror::Error)]
pub enum RateLimitError<E: std::error::Error> {
    #[error("Rate limit exceeded: {reason}")]
    RateLimitExceeded {
        reason: String,
        retry_after: Duration,
    },
    #[error("IP address blocked: {reason}")]
    IpBlocked { reason: String },
    #[error("Geographic restriction: {reason}")]
    GeographicRestriction { reason: String },
    #[error("Internal error: {0}")]
    Internal(E),
}

/// Example implementation for Axum framework
#[cfg(feature = "axum")]
pub mod axum {
    use super::*;
    use axum::{
        extract::{Request, State},
        http::{StatusCode, HeaderMap, HeaderValue},
        response::{Response, IntoResponse},
        middleware::Next,
    };
    use std::net::SocketAddr;

    impl HttpRequest for Request {
        fn get_header(&self, name: &str) -> Option<&str> {
            self.headers().get(name)?.to_str().ok()
        }

        fn path(&self) -> &str {
            self.uri().path()
        }

        fn method(&self) -> &str {
            self.method().as_str()
        }

        fn remote_addr(&self) -> IpAddr {
            // In a real implementation, you'd extract this from connection info
            "127.0.0.1".parse().unwrap()
        }
    }

    pub struct AxumResponse(Response);

    impl HttpResponse for AxumResponse {
        fn set_header(&mut self, name: &str, value: String) {
            if let Ok(header_value) = HeaderValue::from_str(&value) {
                self.0.headers_mut().insert(name, header_value);
            }
        }

        fn set_status(&mut self, status: u16) {
            *self.0.status_mut() = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        }

        fn set_body(&mut self, body: String) {
            *self.0.body_mut() = body.into();
        }
    }

    pub async fn axum_rate_limit_middleware(
        State(limiter): State<RateLimiter>,
        request: Request,
        next: Next,
    ) -> Result<Response, RateLimitError<Box<dyn std::error::Error + Send + Sync>>> {
        let config = MiddlewareConfig::default();
        let middleware = HttpRateLimitMiddleware::new(limiter, config);

        let context = middleware.create_context(&request);
        let result = middleware.limiter.check_rate_limit(&context).await;

        match result {
            RateLimitResult::Allowed { .. } => {
                // Record the request
                if let Err(e) = middleware.limiter.record_request(&context).await {
                    error!("Failed to record request: {}", e);
                }

                let mut response = next.run(request).await;
                middleware.apply_rate_limit_headers(&mut AxumResponse(response), &result);
                Ok(response)
            }
            RateLimitResult::Blocked { reason, retry_after, .. } => {
                let error_response = axum::Json(serde_json::json!({
                    "error": "Rate limit exceeded",
                    "message": reason,
                    "retry_after": retry_after.as_secs()
                }));

                let mut response = (StatusCode::TOO_MANY_REQUESTS, error_response).into_response();
                middleware.apply_rate_limit_headers(&mut AxumResponse(response), &result);
                Ok(response)
            }
        }
    }
}

/// Example implementation for Actix-web framework
#[cfg(feature = "actix")]
pub mod actix {
    use super::*;
    use actix_web::{
        dev::{ServiceRequest, ServiceResponse, Transform},
        http::{header, StatusCode},
        Error, HttpMessage, HttpResponse,
    };
    use futures_util::future::{ok, Ready};
    use std::net::SocketAddr;

    impl HttpRequest for ServiceRequest {
        fn get_header(&self, name: &str) -> Option<&str> {
            self.headers().get(name)?.to_str().ok()
        }

        fn path(&self) -> &str {
            self.path()
        }

        fn method(&self) -> &str {
            self.method().as_str()
        }

        fn remote_addr(&self) -> IpAddr {
            self.connection_info().realip_remote_addr()
                .and_then(|addr| addr.parse().ok())
                .unwrap_or_else(|| "127.0.0.1".parse().unwrap())
        }
    }

    pub struct ActixRateLimitMiddleware;

    impl<S, B> Transform<S, ServiceRequest> for ActixRateLimitMiddleware
    where
        S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        S::Future: 'static,
        B: 'static,
    {
        type Response = ServiceResponse<B>;
        type Error = Error;
        type Transform = RateLimitMiddlewareService<S>;
        type InitError = ();
        type Future = Ready<Result<Self::Transform, Self::InitError>>;

        fn new_transform(&self, service: S) -> Self::Future {
            ok(RateLimitMiddlewareService { service })
        }
    }

    pub struct RateLimitMiddlewareService<S> {
        service: S,
    }

    impl<S, B> actix_web::dev::Service<ServiceRequest> for RateLimitMiddlewareService<S>
    where
        S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        S::Future: 'static,
        B: 'static,
    {
        type Response = ServiceResponse<B>;
        type Error = Error;
        type Future = futures_util::future::LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

        actix_web::dev::forward_ready!(service);

        fn call(&self, req: ServiceRequest) -> Self::Future {
            let limiter = req.app_data::<RateLimiter>()
                .expect("RateLimiter must be registered in app data")
                .clone();

            Box::pin(async move {
                let config = MiddlewareConfig::default();
                let middleware = HttpRateLimitMiddleware::new(limiter, config);

                let context = middleware.create_context(&req);
                let result = middleware.limiter.check_rate_limit(&context).await;

                match result {
                    RateLimitResult::Allowed { .. } => {
                        // Record the request
                        if let Err(e) = middleware.limiter.record_request(&context).await {
                            error!("Failed to record request: {}", e);
                        }

                        let response = self.service.call(req).await?;
                        Ok(response)
                    }
                    RateLimitResult::Blocked { reason, retry_after, .. } => {
                        let error_response = HttpResponse::TooManyRequests().json(serde_json::json!({
                            "error": "Rate limit exceeded",
                            "message": reason,
                            "retry_after": retry_after.as_secs()
                        }));

                        let mut response = error_response.respond_to(&req);
                        middleware.apply_rate_limit_headers(&mut response, &result);
                        Ok(req.into_response(response))
                    }
                }
            })
        }
    }
}

/// Example implementation for custom frameworks
pub struct CustomRateLimitMiddleware;

#[async_trait]
impl RateLimitMiddleware for CustomRateLimitMiddleware {
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Request = CustomRequest;
    type Response = CustomResponse;

    async fn handle_request(&self, request: Self::Request) -> Result<Self::Response, RateLimitError<Self::Error>> {
        // Implementation for custom framework
        todo!("Implement for your specific framework")
    }
}

// Example request/response types for custom framework
pub struct CustomRequest {
    pub method: String,
    pub path: String,
    pub headers: std::collections::HashMap<String, String>,
    pub remote_addr: IpAddr,
}

pub struct CustomResponse {
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
}

impl HttpRequest for CustomRequest {
    fn get_header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn method(&self) -> &str {
        &self.method
    }

    fn remote_addr(&self) -> IpAddr {
        self.remote_addr
    }
}

impl HttpResponse for CustomResponse {
    fn set_header(&mut self, name: &str, value: String) {
        self.headers.insert(name.to_string(), value);
    }

    fn set_status(&mut self, status: u16) {
        self.status = status;
    }

    fn set_body(&mut self, body: String) {
        self.body = body;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;
    use std::str::FromStr;

    struct TestRequest {
        path: String,
        method: String,
        headers: std::collections::HashMap<String, String>,
        remote_addr: IpAddr,
    }

    impl HttpRequest for TestRequest {
        fn get_header(&self, name: &str) -> Option<&str> {
            self.headers.get(name).map(|s| s.as_str())
        }

        fn path(&self) -> &str {
            &self.path
        }

        fn method(&self) -> &str {
            &self.method
        }

        fn remote_addr(&self) -> IpAddr {
            self.remote_addr
        }
    }

    async fn make_proxy_request(
        _forwarded_for: &str,
        _connection_ip: &str,
        trusted_ips: Vec<&str>,
        trusted_ranges: Vec<&str>,
        _real_ip: Option<&str>,
    ) -> HttpRateLimitMiddleware {
        let config = MiddlewareConfig {
            trusted_proxies: trusted_ips.iter().map(|s| s.parse().unwrap()).collect(),
            trusted_proxy_ranges: trusted_ranges.iter().map(|s| s.parse().unwrap()).collect(),
            log_ip_resolution: false,
            ..MiddlewareConfig::default()
        };
        let limiter = RateLimiter::new(
            crate::rate_limiting::config::RateLimitConfig::default(),
            Box::new(crate::rate_limiting::storage::MemoryStorage::new()),
        ).await.unwrap();
        HttpRateLimitMiddleware::new(limiter, config)
    }

    #[tokio::test]
    async    fn test_spoofed_xff_from_untrusted_connection() {
        let middleware = make_proxy_request(
            "",
            "203.0.113.5",
            vec![],
            vec![],
            None,
        ).await;

        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Forwarded-For".to_string(), "198.51.100.1".to_string());

        let request = TestRequest {
            path: "/api/test".to_string(),
            method: "GET".to_string(),
            headers,
            remote_addr: "203.0.113.5".parse().unwrap(),
        };

        // X-Forwarded-For should be ignored because connection IP is not a trusted proxy
        let resolved = middleware.resolve_client_ip(&request);
        assert_eq!(resolved.client_ip, "203.0.113.5".parse::<IpAddr>().unwrap());
        assert_eq!(resolved.resolution_source, "direct");
    }

    #[tokio::test]
    async fn test_single_trusted_proxy_xff() {
        let middleware = make_proxy_request(
            "",
            "10.0.0.1",
            vec!["10.0.0.1"],
            vec![],
            None,
        ).await;

        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Forwarded-For".to_string(), "198.51.100.1".to_string());

        let request = TestRequest {
            path: "/api/test".to_string(),
            method: "GET".to_string(),
            headers,
            remote_addr: "10.0.0.1".parse().unwrap(),
        };

        let resolved = middleware.resolve_client_ip(&request);
        assert_eq!(resolved.client_ip, "198.51.100.1".parse::<IpAddr>().unwrap());
        assert_eq!(resolved.resolution_source, "X-Forwarded-For");
    }

    #[tokio::test]
    async fn test_multi_proxy_chain() {
        // Real client -> Proxy1 (10.0.0.1) -> Proxy2 (10.0.0.2) -> App
        let middleware = make_proxy_request(
            "",
            "10.0.0.2", // connection comes from Proxy2
            vec!["10.0.0.1", "10.0.0.2"], // both proxies trusted
            vec![],
            None,
        ).await;

        let mut headers = std::collections::HashMap::new();
        // X-Forwarded-For: <client>, <proxy1>
        headers.insert("X-Forwarded-For".to_string(), "198.51.100.1, 10.0.0.1".to_string());

        let request = TestRequest {
            path: "/api/test".to_string(),
            method: "GET".to_string(),
            headers,
            remote_addr: "10.0.0.2".parse().unwrap(),
        };

        let resolved = middleware.resolve_client_ip(&request);
        // Rightmost untrusted IP is 198.51.100.1 (proxy1 is trusted)
        assert_eq!(resolved.client_ip, "198.51.100.1".parse::<IpAddr>().unwrap());
        assert_eq!(resolved.resolution_source, "X-Forwarded-For");
    }

    #[tokio::test]
    async fn test_all_proxies_trusted_uses_leftmost() {
        // Client -> Proxy1 -> Proxy2 -> App
        // Both proxies are trusted, so all IPs in chain are trusted
        let middleware = make_proxy_request(
            "",
            "10.0.0.2",
            vec!["10.0.0.1", "10.0.0.2"],
            vec![],
            None,
        ).await;

        let mut headers = std::collections::HashMap::new();
        // All IPs in chain are trusted proxies
        headers.insert("X-Forwarded-For".to_string(), "10.0.0.1".to_string());

        let request = TestRequest {
            path: "/api/test".to_string(),
            method: "GET".to_string(),
            headers,
            remote_addr: "10.0.0.2".parse().unwrap(),
        };

        let resolved = middleware.resolve_client_ip(&request);
        // All trusted — leftmost (10.0.0.1) is the client
        assert_eq!(resolved.client_ip, "10.0.0.1".parse::<IpAddr>().unwrap());
    }

    #[tokio::test]
    async fn test_cidr_trusted_proxy() {
        // Use CIDR range 10.0.0.0/8 to trust all 10.x.x.x IPs
        let middleware = make_proxy_request(
            "",
            "10.1.2.3",
            vec![],
            vec!["10.0.0.0/8"],
            None,
        ).await;

        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Forwarded-For".to_string(), "198.51.100.1".to_string());

        let request = TestRequest {
            path: "/api/test".to_string(),
            method: "GET".to_string(),
            headers,
            remote_addr: "10.1.2.3".parse().unwrap(),
        };

        let resolved = middleware.resolve_client_ip(&request);
        // 10.1.2.3 is in 10.0.0.0/8, so it's trusted
        assert_eq!(resolved.client_ip, "198.51.100.1".parse::<IpAddr>().unwrap());
    }

    #[tokio::test]
    async fn test_cidr_outside_range_not_trusted() {
        // CIDR range 10.0.0.0/8 — only 10.x.x.x should be trusted
        let middleware = make_proxy_request(
            "",
            "192.168.1.1",
            vec![],
            vec!["10.0.0.0/8"],
            None,
        ).await;

        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Forwarded-For".to_string(), "198.51.100.1".to_string());

        let request = TestRequest {
            path: "/api/test".to_string(),
            method: "GET".to_string(),
            headers,
            remote_addr: "192.168.1.1".parse().unwrap(),
        };

        let resolved = middleware.resolve_client_ip(&request);
        // 192.168.1.1 is NOT in 10.0.0.0/8, so X-Forwarded-For is rejected
        assert_eq!(resolved.client_ip, "192.168.1.1".parse::<IpAddr>().unwrap());
        assert_eq!(resolved.resolution_source, "direct");
    }

    #[tokio::test]
    async fn test_x_real_ip_fallback() {
        let middleware = make_proxy_request(
            "",
            "10.0.0.1",
            vec!["10.0.0.1"],
            vec![],
            Some("198.51.100.1"),
        ).await;

        // No X-Forwarded-For, only X-Real-IP header
        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Real-IP".to_string(), "198.51.100.1".to_string());

        let request = TestRequest {
            path: "/api/test".to_string(),
            method: "GET".to_string(),
            headers,
            remote_addr: "10.0.0.1".parse().unwrap(),
        };

        let resolved = middleware.resolve_client_ip(&request);
        // X-Real-IP is trusted via X-Real-IP header from trusted proxy
        assert_eq!(resolved.client_ip, "198.51.100.1".parse::<IpAddr>().unwrap());
        assert_eq!(resolved.resolution_source, "X-Real-IP");
    }

    #[tokio::test]
    async fn test_xff_takes_priority_over_x_real_ip() {
        let middleware = make_proxy_request(
            "",
            "10.0.0.1",
            vec!["10.0.0.1"],
            vec![],
            Some("203.0.113.1"),
        ).await;

        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Forwarded-For".to_string(), "198.51.100.1".to_string());

        let request = TestRequest {
            path: "/api/test".to_string(),
            method: "GET".to_string(),
            headers,
            remote_addr: "10.0.0.1".parse().unwrap(),
        };

        let resolved = middleware.resolve_client_ip(&request);
        // X-Forwarded-For should take priority
        assert_eq!(resolved.client_ip, "198.51.100.1".parse::<IpAddr>().unwrap());
        assert_eq!(resolved.resolution_source, "X-Forwarded-For");
    }

    #[tokio::test]
    async fn test_spoofed_xff_rejected() {
        // Attacker sends X-Forwarded-For but connection comes from non-trusted IP
        let middleware = make_proxy_request(
            "",
            "203.0.113.5", // direct internet connection, not a proxy
            vec!["10.0.0.1"],
            vec![],
            None,
        ).await;

        let mut headers = std::collections::HashMap::new();
        // Attacker sets X-Forwarded-For: 127.0.0.1 (trying to spoof as localhost)
        headers.insert("X-Forwarded-For".to_string(), "127.0.0.1".to_string());

        let request = TestRequest {
            path: "/api/test".to_string(),
            method: "GET".to_string(),
            headers,
            remote_addr: "203.0.113.5".parse().unwrap(),
        };

        let resolved = middleware.resolve_client_ip(&request);
        // Should use connection IP, not spoofed X-Forwarded-For
        assert_eq!(resolved.client_ip, "203.0.113.5".parse::<IpAddr>().unwrap());
        assert_eq!(resolved.resolution_source, "direct");
    }

    #[tokio::test]
    async fn test_multi_proxy_cidr_chain() {
        // Real client -> Proxy1 (10.0.0.1) -> Proxy2 (172.16.0.1) -> App
        // Both IPs are in trusted CIDR ranges
        let middleware = make_proxy_request(
            "",
            "172.16.0.1", // connection comes from Proxy2
            vec![],
            vec!["10.0.0.0/8", "172.16.0.0/12"], // both ranges trusted
            None,
        ).await;

        let mut headers = std::collections::HashMap::new();
        // X-Forwarded-For: <client>, <proxy1>
        headers.insert("X-Forwarded-For".to_string(), "198.51.100.1, 10.0.0.1".to_string());

        let request = TestRequest {
            path: "/api/test".to_string(),
            method: "GET".to_string(),
            headers,
            remote_addr: "172.16.0.1".parse().unwrap(),
        };

        let resolved = middleware.resolve_client_ip(&request);
        // 172.16.0.1 is in 172.16.0.0/12 — trusted
        // 10.0.0.1 is in 10.0.0.0/8 — trusted
        // Rightmost untrusted: 198.51.100.1
        assert_eq!(resolved.client_ip, "198.51.100.1".parse::<IpAddr>().unwrap());
        assert_eq!(resolved.resolution_source, "X-Forwarded-For");
    }

    #[tokio::test]
    async fn test_context_extraction() {
        let mut config = MiddlewareConfig::default();
        config.log_ip_resolution = false;
        let limiter = RateLimiter::new(
            crate::rate_limiting::config::RateLimitConfig::default(),
            Box::new(crate::rate_limiting::storage::MemoryStorage::new()),
        ).await.unwrap();
        let middleware = HttpRateLimitMiddleware::new(limiter, config);

        let mut headers = std::collections::HashMap::new();
        headers.insert("X-User-ID".to_string(), "550e8400-e29b-41d4-a716-446655440000".to_string());
        headers.insert("X-User-Tier".to_string(), "premium".to_string());

        let request = TestRequest {
            path: "/api/test".to_string(),
            method: "GET".to_string(),
            headers,
            remote_addr: "127.0.0.1".parse().unwrap(),
        };

        let context = middleware.create_context(&request);
        assert!(context.user_id.is_some());
        assert_eq!(context.tier, RateLimitTier::Premium);
    }

    #[tokio::test]
    async fn test_ip_metadata_in_context() {
        let middleware = make_proxy_request(
            "",
            "10.0.0.1",
            vec!["10.0.0.1"],
            vec![],
            None,
        ).await;

        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Forwarded-For".to_string(), "198.51.100.1".to_string());

        let request = TestRequest {
            path: "/api/test".to_string(),
            method: "GET".to_string(),
            headers,
            remote_addr: "10.0.0.1".parse().unwrap(),
        };

        let context = middleware.create_context(&request);
        assert_eq!(
            context.metadata.get("resolved_client_ip").unwrap(),
            "198.51.100.1"
        );
        assert_eq!(
            context.metadata.get("connection_ip").unwrap(),
            "10.0.0.1"
        );
        assert_eq!(
            context.metadata.get("ip_resolution_source").unwrap(),
            "X-Forwarded-For"
        );
        assert_eq!(
            context.metadata.get("forwarded_for_chain").unwrap(),
            "198.51.100.1"
        );
    }

    #[test]
    #[tokio::test]
    async fn test_is_trusted_proxy_exact_match() {
        let config = MiddlewareConfig {
            trusted_proxies: vec!["10.0.0.1".parse().unwrap(), "10.0.0.2".parse().unwrap()],
            trusted_proxy_ranges: vec![],
            log_ip_resolution: false,
            ..MiddlewareConfig::default()
        };
        let limiter = RateLimiter::new(
            crate::rate_limiting::config::RateLimitConfig::default(),
            Box::new(crate::rate_limiting::storage::MemoryStorage::new()),
        ).await.unwrap();
        let middleware = HttpRateLimitMiddleware::new(limiter, config);

        assert!(middleware.is_trusted_proxy("10.0.0.1".parse().unwrap()));
        assert!(middleware.is_trusted_proxy("10.0.0.2".parse().unwrap()));
        assert!(!middleware.is_trusted_proxy("10.0.0.3".parse().unwrap()));
        assert!(!middleware.is_trusted_proxy("192.168.1.1".parse().unwrap()));
    }

    #[test]
    #[tokio::test]
    async fn test_is_trusted_proxy_cidr_range() {
        let config = MiddlewareConfig {
            trusted_proxies: vec![],
            trusted_proxy_ranges: vec![
                "10.0.0.0/8".parse().unwrap(),
                "172.16.0.0/12".parse().unwrap(),
            ],
            log_ip_resolution: false,
            ..MiddlewareConfig::default()
        };
        let limiter = RateLimiter::new(
            crate::rate_limiting::config::RateLimitConfig::default(),
            Box::new(crate::rate_limiting::storage::MemoryStorage::new()),
        ).await.unwrap();
        let middleware = HttpRateLimitMiddleware::new(limiter, config);

        assert!(middleware.is_trusted_proxy("10.1.2.3".parse().unwrap()));  // in 10.0.0.0/8
        assert!(middleware.is_trusted_proxy("10.255.255.255".parse().unwrap()));  // in 10.0.0.0/8
        assert!(middleware.is_trusted_proxy("172.16.0.1".parse().unwrap()));  // in 172.16.0.0/12
        assert!(middleware.is_trusted_proxy("172.31.255.255".parse().unwrap()));  // in 172.16.0.0/12
        assert!(!middleware.is_trusted_proxy("192.168.1.1".parse().unwrap()));  // not in any range
        assert!(!middleware.is_trusted_proxy("8.8.8.8".parse().unwrap()));  // public DNS, not trusted
    }

    #[test]
    #[tokio::test]
    async fn test_is_trusted_proxy_exact_overrides_cidr() {
        let config = MiddlewareConfig {
            trusted_proxies: vec!["10.0.0.5".parse().unwrap()],  // explicitly trusted
            trusted_proxy_ranges: vec!["10.0.0.0/8".parse().unwrap()],
            log_ip_resolution: false,
            ..MiddlewareConfig::default()
        };
        let limiter = RateLimiter::new(
            crate::rate_limiting::config::RateLimitConfig::default(),
            Box::new(crate::rate_limiting::storage::MemoryStorage::new()),
        ).await.unwrap();
        let middleware = HttpRateLimitMiddleware::new(limiter, config);

        assert!(middleware.is_trusted_proxy("10.0.0.5".parse().unwrap()));  // exact match
        assert!(middleware.is_trusted_proxy("10.0.0.6".parse().unwrap()));  // CIDR match
        assert!(!middleware.is_trusted_proxy("11.0.0.1".parse().unwrap()));  // not in range
    }
}
