// Search Manager - Routes tool calls to appropriate search service
use super::service_trait::SearchService;
use crate::{
    config::McpConfig,
    search::*,
    errors::{McpError, McpResult},
    mcp::McpTool,
    security::SecurityClient,
    db::Database,
};
use std::collections::HashMap;
use std::sync::Arc;

pub struct SearchManager {
    services: HashMap<String, Arc<dyn SearchService>>,
}

impl SearchManager {
    pub async fn new(database: Database, _config: &McpConfig) -> anyhow::Result<Self> {
        let mut services: HashMap<String, Arc<dyn SearchService>> = HashMap::new();
        
        let _security_client = Arc::new(SecurityClient::new(database.clone()));
        
        // Initialize Memory connector (decision engine integration)
        let decision_engine_url = std::env::var("DECISION_ENGINE_URL")
            .unwrap_or_else(|_| "http://localhost:3016".to_string());
        let memory_service = memory::MemoryService::new(decision_engine_url.clone());
        services.insert("memory".to_string(), Arc::new(memory_service));
        
        // Initialize Embeddings service (vector search)
        let embeddings_url = std::env::var("EMBEDDINGS_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:3001".to_string());
        let embeddings_service = embeddings::EmbeddingsService::new(embeddings_url.clone());
        services.insert("embeddings".to_string(), Arc::new(embeddings_service));
        
        // Initialize Graph service (knowledge graph search)
        let relation_graph_url = std::env::var("RELATION_GRAPH_URL")
            .unwrap_or_else(|_| "http://localhost:3003".to_string());
        let graph_service = graph::GraphSearchService::new(relation_graph_url.clone());
        services.insert("graph".to_string(), Arc::new(graph_service));
        
        // Initialize Hybrid service (search orchestrator)
        // Combines vector search (embeddings) and graph search for intelligent retrieval
        let ollama_url = std::env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://ollama-shared:11434".to_string());
        let hybrid_service = hybrid::HybridSearchService::new(
            embeddings_url.clone(),
            relation_graph_url,
            ollama_url,
        );
        services.insert("context".to_string(), Arc::new(hybrid_service));
        
        Ok(Self { services })
    }
    
    pub fn service_count(&self) -> usize {
        self.services.len()
    }
    
    /// List all tools from all search services
    pub fn list_all_tools(&self) -> Vec<McpTool> {
        let mut tools = Vec::new();
        for service in self.services.values() {
            tools.extend(service.list_tools());
        }
        tools
    }
    
    /// List all resources from all search services
    pub fn list_all_resources(&self) -> Vec<ResourceDescriptor> {
        let mut resources = Vec::new();
        for service in self.services.values() {
            resources.extend(service.list_resources());
        }
        resources
    }
    
    /// Call a tool - routes to appropriate search service based on prefix
    /// Tool names are: "service.tool_name" (e.g. "embeddings.search")
    pub async fn call_tool(&self, fully_qualified_name: &str, args: serde_json::Value) -> McpResult<serde_json::Value> {
        let parts: Vec<&str> = fully_qualified_name.splitn(2, '.').collect();
        
        if parts.len() != 2 {
            return Err(McpError::InvalidArguments(
                format!("Tool name must be in format 'service.tool': {}", fully_qualified_name)
            ));
        }
        
        let (service_id, tool_name) = (parts[0], parts[1]);
        
        let service = self.services.get(service_id)
            .ok_or_else(|| McpError::ToolNotFound(
                format!("Search service not found: {}", service_id)
            ))?;
        
        service.call_tool(tool_name, args).await
    }
    
    /// Read a resource - routes based on URI prefix
    pub async fn read_resource(&self, uri: &str) -> McpResult<ResourceContent> {
        // Parse URI to extract service (e.g. "blob://..." or "graph://...")
        if let Some(colon_pos) = uri.find("://") {
            let service_id = &uri[..colon_pos];
            
            let service = self.services.get(service_id)
                .ok_or_else(|| McpError::ToolNotFound(
                    format!("Search service not found: {}", service_id)
                ))?;
            
            service.read_resource(uri).await
        } else {
            Err(McpError::InvalidArguments(
                format!("Invalid resource URI format: {}", uri)
            ))
        }
    }
}
