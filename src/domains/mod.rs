pub mod qr;
pub mod ocr;
pub mod rewards;
pub mod invoices;

// Re-export domain modules for easier access
pub use qr as qr_service;
pub use ocr as ocr_service;
pub use rewards as rewards_service;
pub use invoices as invoice_service;
