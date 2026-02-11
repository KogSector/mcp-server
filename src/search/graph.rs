// Graph Search Service - Direct access to knowledge graph knowledge layer
use crate::{search::*, mcp::McpTool, errors::{McpError, McpResult}};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use super::service_trait::SearchService;

pub struct GraphSearchService {
    base_url: String,
    client: reqwest::Client,
}

impl GraphSearchService {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    chunks: Vec<Value>,
    entities: Option<Vec<Value>>,
}

#[derive(Debug, Deserialize)]
struct EntityResponse {
    entity: Value,
    neighbors: Option<Vec<Value>>,
}

#[async_trait]
impl SearchService for GraphSearchService {
    fn id(&self) -> &'static str {
        "graph"
    }
    
    fn list_tools(&self) -> Vec<McpTool> {
        vec![
            McpTool {
                name: "graph.search".to_string(),
                description: "Hybrid semantic + graph search across the knowledge base".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max results (default: 10)",
                            "default": 10
                        },
                        "source_types": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Filter by source type (code, documentation, etc.)"
                        }
                    },
                    "required": ["query"]
                })),
            },
            McpTool {
                name: "graph.traverse".to_string(),
                description: "Traverse the knowledge graph from a starting entity".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "entity_id": {
                            "type": "string",
                            "description": "Starting entity ID"
                        },
                        "relationship_types": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Filter by relationship type (CALLS, IMPORTS, etc.)"
                        },
                        "depth": {
                            "type": "integer",
                            "description": "Max traversal depth (default: 2)",
                            "default": 2
                        }
                    },
                    "required": ["entity_id"]
                })),
            },
            McpTool {
                name: "graph.get_entity".to_string(),
                description: "Get details of a specific entity".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "entity_id": {
                            "type": "string",
                            "description": "Entity ID"
                        }
                    },
                    "required": ["entity_id"]
                })),
            },
            McpTool {
                name: "graph.list_ontologies".to_string(),
                description: "List available ontologies (entity type schemas)".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {}
                })),
            },
            McpTool {
                name: "graph.statistics".to_string(),
                description: "Get knowledge graph statistics".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {}
                })),
            },
        ]
    }
    
    async fn call_tool(&self, tool: &str, args: Value) -> McpResult<Value> {
        match tool {
            "search" => {
                let query = args.get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments("Missing 'query' argument".into()))?;
                
                let limit = args.get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10) as usize;
                
                let response = self.client
                    .post(format!("{}/api/search", self.base_url))
                    .json(&json!({
                        "query": query,
                        "limit": limit,
                        "include_entities": true
                    }))
                    .send()
                    .await
                    .map_err(|e| McpError::Internal(format!("Search request failed: {}", e)))?;
                
                if !response.status().is_success() {
                    return Err(McpError::Internal(format!(
                        "Relation graph returned {}", response.status()
                    )));
                }
                
                let result: Value = response.json().await
                    .map_err(|e| McpError::Internal(format!("Failed to parse response: {}", e)))?;
                
                Ok(result)
            }
            
            "traverse" => {
                let entity_id = args.get("entity_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments("Missing 'entity_id' argument".into()))?;
                
                let depth = args.get("depth")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(2);
                
                // Get entity and neighbors
                let response = self.client
                    .get(format!("{}/api/graph/entities/{}/neighbors", self.base_url, entity_id))
                    .query(&[("depth", depth.to_string())])
                    .send()
                    .await
                    .map_err(|e| McpError::Internal(format!("Traverse request failed: {}", e)))?;
                
                if !response.status().is_success() {
                    return Err(McpError::Internal(format!(
                        "Relation graph returned {}", response.status()
                    )));
                }
                
                let result: Value = response.json().await
                    .map_err(|e| McpError::Internal(format!("Failed to parse response: {}", e)))?;
                
                Ok(result)
            }
            
            "get_entity" => {
                let entity_id = args.get("entity_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments("Missing 'entity_id' argument".into()))?;
                
                let response = self.client
                    .get(format!("{}/api/graph/entities/{}", self.base_url, entity_id))
                    .send()
                    .await
                    .map_err(|e| McpError::Internal(format!("Get entity request failed: {}", e)))?;
                
                if !response.status().is_success() {
                    return Err(McpError::Internal(format!(
                        "Relation graph returned {}", response.status()
                    )));
                }
                
                let result: Value = response.json().await
                    .map_err(|e| McpError::Internal(format!("Failed to parse response: {}", e)))?;
                
                Ok(result)
            }
            
            "list_ontologies" => {
                let response = self.client
                    .get(format!("{}/api/ontology", self.base_url))
                    .send()
                    .await
                    .map_err(|e| McpError::Internal(format!("List ontologies request failed: {}", e)))?;
                
                if !response.status().is_success() {
                    return Err(McpError::Internal(format!(
                        "Relation graph returned {}", response.status()
                    )));
                }
                
                let result: Value = response.json().await
                    .map_err(|e| McpError::Internal(format!("Failed to parse response: {}", e)))?;
                
                Ok(result)
            }
            
            "statistics" => {
                let response = self.client
                    .get(format!("{}/api/graph/statistics", self.base_url))
                    .send()
                    .await
                    .map_err(|e| McpError::Internal(format!("Statistics request failed: {}", e)))?;
                
                if !response.status().is_success() {
                    return Err(McpError::Internal(format!(
                        "Relation graph returned {}", response.status()
                    )));
                }
                
                let result: Value = response.json().await
                    .map_err(|e| McpError::Internal(format!("Failed to parse response: {}", e)))?;
                
                Ok(result)
            }
            
            _ => Err(McpError::ToolNotFound(format!("Unknown tool: graph.{}", tool))),
        }
    }
}
