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
    /// List of trusted proxy IPs
    pub trusted_proxies: Vec<IpAddr>,
    /// Whether to include rate limit headers in responses
    pub include_rate_limit_headers: bool,
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
            include_rate_limit_headers: true,
            context_extractor: None,
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

    /// Extract client IP address from request
    fn extract_client_ip(&self, request: &dyn HttpRequest) -> IpAddr {
        // Try real IP header first (for proxied requests)
        if let Some(header_name) = &self.config.real_ip_header {
            if let Some(ip_str) = request.get_header(header_name) {
                if let Ok(ip) = ip_str.parse::<IpAddr>() {
                    if !self.is_trusted_proxy(ip) {
                        return ip;
                    }
                }
            }
        }

        // Fall back to direct connection IP
        request.remote_addr()
    }

    /// Check if an IP is a trusted proxy
    fn is_trusted_proxy(&self, ip: IpAddr) -> bool {
        self.config.trusted_proxies.contains(&ip)
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
        let client_ip = self.extract_client_ip(request);
        let (user_id, tier) = self.extract_user_info(request);
        let api_key = self.extract_api_key(request);
        let user_agent = request.get_header("User-Agent").map(|s| s.to_string());

        let mut context = RateLimitContext::new(
            client_ip,
            request.path().to_string(),
            request.method().to_string(),
        )
        .with_user_id(user_id.unwrap_or_else(Uuid::new_v4))
        .with_tier(tier)
        .with_user_agent(user_agent.unwrap_or_default())
        .with_api_key(api_key.unwrap_or_default());

        // Add additional metadata
        if let Some(forwarded_for) = request.get_header("X-Forwarded-For") {
            context = context.with_metadata("forwarded_for".to_string(), forwarded_for.to_string());
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

    #[test]
    fn test_context_extraction() {
        let config = MiddlewareConfig::default();
        let limiter = RateLimiter::new(
            crate::rate_limiting::config::RateLimitConfig::default(),
            Box::new(crate::rate_limiting::storage::MemoryStorage::new()),
        );
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
}
