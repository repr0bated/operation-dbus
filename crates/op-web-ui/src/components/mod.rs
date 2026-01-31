//! UI Components

mod chat;
mod system_monitor;
mod tool_log;
mod system_prompt;
mod tools_view;
mod workflows_view;
mod mcp_admin;
mod plugin_config;
mod system_log;

pub use chat::ChatInterface;
pub use system_monitor::SystemMonitor;
pub use tool_log::ToolLog;
pub use system_prompt::SystemPromptView;
pub use tools_view::ToolsView;
pub use workflows_view::WorkflowsView;
pub use mcp_admin::McpAdminView;
pub use plugin_config::PluginConfigView;
pub use system_log::SystemLogView;
