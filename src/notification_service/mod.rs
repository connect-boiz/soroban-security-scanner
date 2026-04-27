//! Notification Service for Soroban Security Scanner
//! 
//! This module provides a comprehensive notification system supporting:
//! - Email notifications
//! - SMS notifications  
//! - Push notifications
//! - In-app alerts
//! - Template management
//! - Delivery tracking

pub mod providers;
pub mod templates;
pub mod tracking;
pub mod types;
pub mod service;

#[cfg(test)]
mod tests;

pub use service::NotificationService;
pub use types::*;
pub use providers::*;
pub use templates::*;
pub use tracking::*;
