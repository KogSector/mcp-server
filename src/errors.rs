// MCP Error Types
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type McpResult<T> = Result<T, McpError>;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    
    #[error("Rate limited: {0}")]
    RateLimited(String),
    
    #[error("Provider error: {0}")]
    ProviderError(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Connector disabled: {0}")]
    ConnectorDisabled(String),
    
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpErrorResponse {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl McpError {
    pub fn to_jsonrpc_error(&self) -> McpErrorResponse {
        match self {
            McpError::ToolNotFound(msg) => McpErrorResponse {
                code: -32601,
                message: msg.clone(),
                data: None,
            },
            McpError::InvalidArguments(msg) => McpErrorResponse {
                code: -32602,
                message: msg.clone(),
                data: None,
            },
            McpError::Unauthorized(msg) => McpErrorResponse {
                code: -32001,
                message: msg.clone(),
                data: None,
            },
            McpError::RateLimited(msg) => McpErrorResponse {
                code: -32002,
                message: msg.clone(),
                data: None,
            },
            _ => McpErrorResponse {
                code: -32603,
                message: self.to_string(),
                data: None,
            },
        }
    }
}
