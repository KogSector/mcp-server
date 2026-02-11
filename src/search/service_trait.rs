// Search Service Trait - Common interface for all search and retrieval services
use crate::{search::*, mcp::McpTool, errors::McpResult};
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait SearchService: Send + Sync {
    /// Service identifier (embeddings, graph, blob, etc.)
    fn id(&self) -> &'static str;
    
    /// List all tools this service exposes
    fn list_tools(&self) -> Vec<McpTool>;
    
    /// Call a tool with arguments
    async fn call_tool(&self, tool: &str, args: Value) -> McpResult<Value>;
    
    /// Optional: List resources (for browsable services)
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
