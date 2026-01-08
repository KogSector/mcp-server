// MCP Service - Unified Model Context Protocol server for ConHub
// Single service exposing all connectors: GitHub, GitLab, Bitbucket, Google Drive, Dropbox, local FS, Notion

pub mod config;
pub mod protocol;
pub mod context;
pub mod connectors;
pub mod security_client;
pub mod errors;
pub mod db;

pub use config::McpConfig;
pub use errors::{McpError, McpResult};
