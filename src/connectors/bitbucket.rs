// Bitbucket Connector - Stub implementation
use super::Connector;
use crate::{context::*, errors::{McpError, McpResult}, protocol::McpTool, security_client::SecurityClient};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct BitbucketConnector {
    base_url: String,
    security: Arc<SecurityClient>,
}

impl BitbucketConnector {
    pub fn new(base_url: String, security: Arc<SecurityClient>) -> Self {
        Self { base_url, security }
    }
}

#[async_trait]
impl Connector for BitbucketConnector {
    fn id(&self) -> &'static str {
        "bitbucket"
    }
    
    fn list_tools(&self) -> Vec<McpTool> {
        vec![
            McpTool {
                name: "bitbucket.list_repositories".to_string(),
                description: "List Bitbucket repositories".to_string(),
                input_schema: None,
            },
        ]
    }
    
    async fn call_tool(&self, _tool: &str, _args: Value) -> McpResult<Value> {
        Err(McpError::Internal("Bitbucket connector not yet fully implemented".to_string()))
    }
}
