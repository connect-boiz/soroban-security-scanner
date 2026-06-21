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
pub mod service;
pub mod templates;
pub mod tracking;
pub mod types;

#[cfg(feature = "database")]
pub mod db_storage;

#[cfg(test)]
mod tests;

pub use providers::*;
pub use service::NotificationService;
pub use templates::*;
pub use tracking::*;
pub use types::*;

#[cfg(feature = "database")]
pub use db_storage::*;
