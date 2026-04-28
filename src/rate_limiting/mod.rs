//! Sophisticated rate limiting system with multiple tiers and distributed support
//! 
//! Features:
//! - User-based tiers (authenticated/unauthenticated)
//! - IP-based rate limiting
//! - Distributed rate limiting with Redis
//! - Sliding window algorithm
//! - Configurable limits and policies

pub mod config;
pub mod limiter;
pub mod middleware;
pub mod storage;
pub mod types;
pub mod utils;

#[cfg(feature = "examples")]
pub mod examples;

#[cfg(test)]
mod tests;

pub use config::RateLimitConfig;
pub use limiter::RateLimiter;
pub use middleware::RateLimitMiddleware;
pub use types::*;
