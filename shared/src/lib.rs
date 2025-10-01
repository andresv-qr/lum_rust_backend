//! Shared library for Lüm microservices
//! 
//! This library contains common functionality used across all microservices:
//! - Database operations and models
//! - Authentication and authorization
//! - Redis caching
//! - Common types and utilities
//! - Service communication helpers

pub mod auth;
pub mod cache;
pub mod config;
pub mod database;
pub mod error;
pub mod models;
pub mod service_client;
pub mod types;
pub mod utils;

// Re-export commonly used types
pub use auth::{AuthService, Claims, TokenPair};
pub use cache::RedisService;
pub use config::Config;
pub use database::DatabaseService;
pub use error::{AppError, Result};
pub use models::*;
pub use service_client::ServiceClient;
pub use types::*;