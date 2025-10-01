pub mod command_handler;
pub mod state_handler;
pub mod text_handler;
pub mod doc_handler;
pub mod image_handler;
pub mod interactive_handler;
pub mod webhook_handler;

// Re-export main handlers for easy access
pub use webhook_handler::{get_webhook, post_webhook};
pub use image_handler::handle_image_message;
pub use interactive_handler::handle_interactive_message;
pub use command_handler::handle_command;
pub use text_handler::handle_text_message;
pub use state_handler::handle_user_state;
