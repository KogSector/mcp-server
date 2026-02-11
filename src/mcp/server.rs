// MCP Server - JSON-RPC handler
use crate::{
    config::McpConfig,
    search::SearchManager,
    mcp::types::*,
    errors::{McpError, McpResult},
};
use anyhow::Result;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{info, error, debug};

pub struct McpServer {
    search_manager: SearchManager,
    config: McpConfig,
}

impl McpServer {
    pub fn new(search_manager: SearchManager, config: McpConfig) -> Self {
        Self {
            search_manager,
            config,
        }
    }
    
    pub async fn run(mut self) -> Result<()> {
        info!("🔗 ConHub MCP Server starting on stdio");
        info!("📡 Model Context Protocol ready");
        info!("🔌 {} search services enabled", self.search_manager.service_count());
        
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();
        
        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break; // EOF
            }
            
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            debug!("📨 Received request: {}", line);
            
            let response = match self.handle_request(line).await {
                Ok(resp) => resp,
                Err(e) => {
                    error!("❌ Error handling request: {}", e);
                    self.error_response(None, McpError::Other(e))
                }
            };
            
            let response_str = serde_json::to_string(&response)?;
            debug!("📤 Sending response: {}", response_str);
            
            stdout.write_all(response_str.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
        
        info!("MCP Server shutting down");
        Ok(())
    }
    
    async fn handle_request(&mut self, request_str: &str) -> Result<JsonRpcResponse> {
        let request: JsonRpcRequest = serde_json::from_str(request_str)?;
        
        let result = match request.method.as_str() {
            // Standard MCP protocol methods
            "initialize" => self.initialize(request.params).await,
            "tools/list" => self.list_tools().await,
            "tools/call" => self.call_tool(request.params).await,
            "resources/list" => self.list_resources().await,
            "resources/read" => self.read_resource(request.params).await,
            
            // Legacy compatibility (can be removed later)
            "mcp.listTools" => self.list_tools().await,
            "mcp.callTool" => self.call_tool(request.params).await,
            "mcp.listResources" => self.list_resources().await,
            "mcp.readResource" => self.read_resource(request.params).await,
            "mcp.health" => Ok(json!({"status": "healthy"})),
            
            _ => Err(McpError::ToolNotFound(format!("Unknown method: {}", request.method))),
        };
        
        match result {
            Ok(value) => Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(value),
                error: None,
            }),
            Err(e) => Ok(self.error_response(request.id, e)),
        }
    }
    
    async fn initialize(&mut self, params: Option<Value>) -> McpResult<Value> {
        info!("🔧 Initializing MCP connection");
        
        // Parse client info if provided
        let client_info = params
            .and_then(|p| p.get("clientInfo").cloned())
            .and_then(|c| serde_json::from_value::<ClientInfo>(c).ok());
        
        if let Some(info) = &client_info {
            info!("👤 Client: {} v{}", info.name, info.version);
        }
        
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "resources": {
                    "subscribe": false,
                    "listChanged": false
                },
                "tools": {
                    "listChanged": false
                },
                "prompts": {
                    "listChanged": false
                },
                "logging": {}
            },
            "serverInfo": {
                "name": "ConHub MCP Server",
                "version": "1.0.0"
            }
        }))
    }
    
    async fn list_tools(&self) -> McpResult<Value> {
        let tools = self.search_manager.list_all_tools();
        Ok(json!({ "tools": tools }))
    }
    
    async fn call_tool(&self, params: Option<Value>) -> McpResult<Value> {
        let call_request: ToolCallRequest = serde_json::from_value(
            params.ok_or_else(|| McpError::InvalidArguments("Missing params".to_string()))?
        )?;
        
        let result = self.search_manager
            .call_tool(&call_request.name, call_request.arguments)
            .await?;
        
        let tool_result = ToolCallResult::success(serde_json::to_string(&result)?);
        Ok(serde_json::to_value(tool_result)?)
    }
    
    async fn list_resources(&self) -> McpResult<Value> {
        let resources = self.search_manager.list_all_resources();
        Ok(json!({ "resources": resources }))
    }
    
    async fn read_resource(&self, params: Option<Value>) -> McpResult<Value> {
        let params = params.ok_or_else(|| McpError::InvalidArguments("Missing params".to_string()))?;
        let resource_id: String = serde_json::from_value(
            params.get("uri").cloned().ok_or_else(|| McpError::InvalidArguments("Missing uri".to_string()))?
        )?;
        
        let content = self.search_manager.read_resource(&resource_id).await?;
        Ok(serde_json::to_value(content)?)
    }
    
    fn error_response(&self, id: Option<Value>, error: McpError) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error.to_jsonrpc_error()),
        }
    }
}
