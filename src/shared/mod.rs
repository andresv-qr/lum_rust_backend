pub mod database;
pub mod redis;
pub mod redis_compat;
pub mod users;
pub mod whatsapp;
pub mod dashboard;
pub mod performance;

// Re-export shared services for easier access
pub use database as db_service;
pub use redis_compat as redis_service;  // Use compatibility layer
pub use users as user_service;
pub use whatsapp as whatsapp_service;
pub use dashboard as visual_dashboard_service;
pub use performance as performance_service;
