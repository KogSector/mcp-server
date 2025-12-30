// Connector Trait - Common interface for all connectors
use crate::{context::*, protocol::McpTool, errors::McpResult};
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait Connector: Send + Sync {
    /// Connector identifier (github, gitlab, bitbucket, etc.)
    fn id(&self) -> &'static str;
    
    /// List all tools this connector exposes
    fn list_tools(&self) -> Vec<McpTool>;
    
    /// Call a tool with arguments
    async fn call_tool(&self, tool: &str, args: Value) -> McpResult<Value>;
    
    /// Optional: List resources (for browsable connectors)
    fn list_resources(&self) -> Vec<ResourceDescriptor> {
        vec![]
    }
    
    /// Optional: Read a resource by ID
    async fn read_resource(&self, _id: &str) -> McpResult<ResourceContent> {
        Err(crate::errors::McpError::ToolNotFound(
            "Resource reading not supported".to_string()
        ))
    }
}
