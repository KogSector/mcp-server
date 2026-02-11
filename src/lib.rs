// MCP Service - Unified Model Context Protocol server for ConHub
// Provides intelligent search and retrieval tools for AI agents

pub mod config;
pub mod mcp;
pub mod search;
pub mod security;
pub mod errors;
pub mod db;

pub use config::McpConfig;
pub use errors::{McpError, McpResult};
pub use mcp::McpServer;
pub use search::SearchManager;
