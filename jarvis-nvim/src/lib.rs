pub mod lsp;
pub mod nvim_client;
pub mod plugin;
pub mod ai_integration;
pub mod chat_interface;
pub mod code_actions;

pub use nvim_client::JarvisNvim;
pub use plugin::Plugin;
pub use ai_integration::AIIntegration;
