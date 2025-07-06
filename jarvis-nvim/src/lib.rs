pub mod ai_integration;
pub mod chat_interface;
pub mod code_actions;
pub mod lsp;
pub mod nvim_client;
pub mod plugin;

pub use ai_integration::AIIntegration;
pub use nvim_client::JarvisNvim;
pub use plugin::Plugin;
