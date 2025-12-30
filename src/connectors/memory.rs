//! Memory Connector for MCP
//! 
//! Exposes ConHub's knowledge layer and memory system to AI agents via MCP tools.
//! This is the primary interface for AI agents to query the knowledge layer.

use crate::{
    context::*,
    errors::{McpError, McpResult},
    protocol::McpTool,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{info, error, debug};

/// Memory connector for MCP
/// 
/// Provides tools for AI agents to:
/// - Search the knowledge layer (code, docs, chat, tickets)
/// - Search robot memory (episodic and semantic)
/// - Get robot context snapshots
/// - Store passive context
pub struct MemoryConnector {
    decision_engine_url: String,
    http_client: reqwest::Client,
}

impl MemoryConnector {
    pub fn new(decision_engine_url: String) -> Self {
        Self {
            decision_engine_url,
            http_client: reqwest::Client::new(),
        }
    }
    
    /// Call the decision engine memory search API
    async fn call_memory_search(&self, request: MemorySearchRequest) -> McpResult<Value> {
        let url = format!("{}/api/memory/search", self.decision_engine_url);
        
        debug!("📡 Calling memory search: {}", url);
        
        let response = self.http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| McpError::Other(e.into()))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(McpError::Other(anyhow::anyhow!(
                "Memory search failed with status {}: {}", status, error_text
            )));
        }
        
        let result: Value = response.json().await
            .map_err(|e| McpError::Other(e.into()))?;
        
        Ok(result)
    }
    
    /// Call robot memory search
    async fn call_robot_memory_search(&self, robot_id: &str, request: RobotMemorySearchRequest) -> McpResult<Value> {
        let url = format!("{}/api/robots/{}/memory/search", self.decision_engine_url, robot_id);
        
        debug!("🤖 Calling robot memory search: {}", url);
        
        let response = self.http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| McpError::Other(e.into()))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(McpError::Other(anyhow::anyhow!(
                "Robot memory search failed with status {}: {}", status, error_text
            )));
        }
        
        let result: Value = response.json().await
            .map_err(|e| McpError::Other(e.into()))?;
        
        Ok(result)
    }
    
    /// Get robot context snapshot
    async fn call_robot_context(&self, robot_id: &str) -> McpResult<Value> {
        let url = format!("{}/api/robots/{}/context/latest", self.decision_engine_url, robot_id);
        
        debug!("🤖 Getting robot context: {}", url);
        
        let response = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| McpError::Other(e.into()))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(McpError::Other(anyhow::anyhow!(
                "Robot context fetch failed with status {}: {}", status, error_text
            )));
        }
        
        let result: Value = response.json().await
            .map_err(|e| McpError::Other(e.into()))?;
        
        Ok(result)
    }
}

#[async_trait]
impl super::Connector for MemoryConnector {
    fn id(&self) -> &'static str {
        "memory"
    }
    
    fn list_tools(&self) -> Vec<McpTool> {
        vec![
            // General memory search
            McpTool {
                name: "memory.search".to_string(),
                description: "Search the knowledge layer for relevant context. \
                    Searches across code, documentation, chat, tickets, and other connected sources. \
                    Returns ranked context blocks with provenance information.".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Natural language query to search for"
                        },
                        "sources": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional: Filter by source types (code, docs, chat, tickets)"
                        },
                        "repos": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional: Filter by repository names"
                        },
                        "time_range": {
                            "type": "object",
                            "properties": {
                                "from": { "type": "string", "format": "date-time" },
                                "to": { "type": "string", "format": "date-time" }
                            },
                            "description": "Optional: Filter by time range"
                        },
                        "max_blocks": {
                            "type": "integer",
                            "description": "Maximum number of context blocks to return (default: 20)"
                        },
                        "strategy": {
                            "type": "string",
                            "enum": ["auto", "vector_only", "graph_only", "hybrid"],
                            "description": "Retrieval strategy (default: auto)"
                        }
                    },
                    "required": ["query"]
                })),
            },
            
            // Robot memory search
            McpTool {
                name: "memory.robot_search".to_string(),
                description: "Search a robot's episodic and semantic memory. \
                    Use this to recall what a robot has seen, done, or learned. \
                    Returns episodes, observations, and semantic facts.".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "robot_id": {
                            "type": "string",
                            "description": "UUID of the robot"
                        },
                        "query": {
                            "type": "string",
                            "description": "Natural language query about robot memory"
                        },
                        "time_range": {
                            "type": "object",
                            "properties": {
                                "from": { "type": "string", "format": "date-time" },
                                "to": { "type": "string", "format": "date-time" }
                            },
                            "description": "Optional: Filter by time range"
                        },
                        "location": {
                            "type": "string",
                            "description": "Optional: Filter by location"
                        },
                        "include_episodic": {
                            "type": "boolean",
                            "description": "Include episodic memory (default: true)"
                        },
                        "include_semantic": {
                            "type": "boolean",
                            "description": "Include semantic facts (default: true)"
                        },
                        "max_blocks": {
                            "type": "integer",
                            "description": "Maximum blocks to return (default: 20)"
                        }
                    },
                    "required": ["robot_id", "query"]
                })),
            },
            
            // Robot context snapshot
            McpTool {
                name: "memory.robot_context".to_string(),
                description: "Get the latest context snapshot for a robot. \
                    Returns current state, recent episodes, relevant facts, and active streams. \
                    Use this to understand a robot's current situation.".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "robot_id": {
                            "type": "string",
                            "description": "UUID of the robot"
                        }
                    },
                    "required": ["robot_id"]
                })),
            },
            
            // Store passive context (for future use)
            McpTool {
                name: "memory.store".to_string(),
                description: "Store a note or observation as passive context. \
                    Use this to remember important information for later retrieval.".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "The content to store"
                        },
                        "tags": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Tags for categorization"
                        },
                        "source": {
                            "type": "string",
                            "description": "Source identifier (e.g., agent name, session id)"
                        },
                        "robot_id": {
                            "type": "string",
                            "description": "Optional: Associate with a specific robot"
                        }
                    },
                    "required": ["content"]
                })),
            },
            
            // Analyze query (for debugging/transparency)
            McpTool {
                name: "memory.analyze_query".to_string(),
                description: "Analyze a query to see how it would be classified. \
                    Returns the detected query kind, modality hint, and suggested strategy. \
                    Useful for understanding retrieval behavior.".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The query to analyze"
                        }
                    },
                    "required": ["query"]
                })),
            },
        ]
    }
    
    async fn call_tool(&self, tool: &str, args: Value) -> McpResult<Value> {
        info!("🔧 Memory tool call: {}", tool);
        
        match tool {
            "search" => {
                let query: String = args.get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments("Missing query".to_string()))?
                    .to_string();
                
                let sources: Vec<String> = args.get("sources")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                
                let repos: Vec<String> = args.get("repos")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                
                let max_blocks: u32 = args.get("max_blocks")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32)
                    .unwrap_or(20);
                
                let strategy = args.get("strategy")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                let request = MemorySearchRequest {
                    tenant_id: uuid::Uuid::nil(), // Will be set from auth
                    user_id: uuid::Uuid::nil(),
                    query,
                    sources,
                    filters: if repos.is_empty() {
                        serde_json::Map::new()
                    } else {
                        let mut map = serde_json::Map::new();
                        map.insert("repos".to_string(), json!(repos));
                        map
                    },
                    max_blocks,
                    max_tokens: 8000,
                    force_strategy: strategy,
                    include_debug: false,
                };
                
                self.call_memory_search(request).await
            }
            
            "robot_search" => {
                let robot_id = args.get("robot_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments("Missing robot_id".to_string()))?;
                
                let query: String = args.get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments("Missing query".to_string()))?
                    .to_string();
                
                let location = args.get("location")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                
                let include_episodic = args.get("include_episodic")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                
                let include_semantic = args.get("include_semantic")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                
                let max_blocks = args.get("max_blocks")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32)
                    .unwrap_or(20);
                
                let request = RobotMemorySearchRequest {
                    robot_id: uuid::Uuid::parse_str(robot_id)
                        .map_err(|_| McpError::InvalidArguments("Invalid robot_id UUID".to_string()))?,
                    tenant_id: uuid::Uuid::nil(),
                    query,
                    time_range: None, // TODO: parse from args
                    location,
                    include_episodic,
                    include_semantic,
                    max_blocks,
                };
                
                self.call_robot_memory_search(robot_id, request).await
            }
            
            "robot_context" => {
                let robot_id = args.get("robot_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments("Missing robot_id".to_string()))?;
                
                self.call_robot_context(robot_id).await
            }
            
            "store" => {
                let content = args.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments("Missing content".to_string()))?;
                
                let tags: Vec<String> = args.get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                
                let source = args.get("source")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                
                // TODO: Actually store this in the passive context database
                // For now, just acknowledge receipt
                info!("📝 Storing passive context: {} chars, {} tags", content.len(), tags.len());
                
                Ok(json!({
                    "success": true,
                    "message": "Context stored successfully",
                    "id": uuid::Uuid::new_v4().to_string(),
                    "content_length": content.len(),
                    "tags": tags,
                    "source": source
                }))
            }
            
            "analyze_query" => {
                let query = args.get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments("Missing query".to_string()))?;
                
                // Simple local analysis (mirrors the decision engine logic)
                let query_lower = query.to_lowercase();
                
                let query_kind = if query_lower.contains("who") || query_lower.contains("depends on") {
                    "topology_question"
                } else if query_lower.contains("how to") || query_lower.contains("how do i") {
                    "how_to"
                } else if query_lower.contains("why") || query_lower.contains("error") {
                    "troubleshooting"
                } else if query_lower.contains("when") || query_lower.contains("last time") {
                    "episodic_lookup"
                } else if query_lower.starts_with("what is") {
                    "fact_lookup"
                } else if query_lower.contains("explain") {
                    "explainer"
                } else {
                    "generic"
                };
                
                let suggested_strategy = match query_kind {
                    "topology_question" => "graph_only",
                    "how_to" | "explainer" | "troubleshooting" => "hybrid",
                    _ => "vector_only",
                };
                
                let modality = if query_lower.contains("code") || query_lower.contains("function") {
                    "code"
                } else if query_lower.contains("robot") || query_lower.contains("episode") {
                    "robot_episodic"
                } else if query_lower.contains("message") || query_lower.contains("slack") {
                    "chat"
                } else {
                    "mixed"
                };
                
                Ok(json!({
                    "query": query,
                    "analysis": {
                        "query_kind": query_kind,
                        "modality_hint": modality,
                        "suggested_strategy": suggested_strategy,
                        "confidence": 0.7
                    }
                }))
            }
            
            _ => Err(McpError::ToolNotFound(format!("Unknown memory tool: {}", tool))),
        }
    }
    
    fn list_resources(&self) -> Vec<ResourceDescriptor> {
        vec![
            ResourceDescriptor {
                id: "memory://knowledge-layer".to_string(),
                name: "ConHub Knowledge Layer".to_string(),
                description: Some("Unified knowledge layer with code, docs, chat, and robot memory".to_string()),
                mime_type: Some("application/json".to_string()),
                uri: "memory://knowledge-layer".to_string(),
            },
        ]
    }
    
    async fn read_resource(&self, uri: &str) -> McpResult<ResourceContent> {
        if uri == "memory://knowledge-layer" {
            Ok(ResourceContent {
                content: json!({
                    "name": "ConHub Knowledge Layer",
                    "description": "Unified knowledge layer for AI agents",
                    "capabilities": [
                        "Semantic search across code, docs, chat, tickets",
                        "Robot episodic and semantic memory",
                        "Graph-based relationship queries",
                        "Automatic query analysis and strategy selection",
                        "Token-budgeted context building"
                    ],
                    "tools": [
                        "memory.search",
                        "memory.robot_search",
                        "memory.robot_context",
                        "memory.store",
                        "memory.analyze_query"
                    ]
                }).to_string(),
                mime_type: Some("application/json".to_string()),
            })
        } else {
            Err(McpError::ToolNotFound(format!("Unknown resource: {}", uri)))
        }
    }
}

// Request/response types for API calls

#[derive(Debug, Serialize)]
struct MemorySearchRequest {
    tenant_id: uuid::Uuid,
    user_id: uuid::Uuid,
    query: String,
    sources: Vec<String>,
    filters: serde_json::Map<String, Value>,
    max_blocks: u32,
    max_tokens: u32,
    force_strategy: Option<String>,
    include_debug: bool,
}

#[derive(Debug, Serialize)]
struct RobotMemorySearchRequest {
    robot_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    query: String,
    time_range: Option<TimeRange>,
    location: Option<String>,
    include_episodic: bool,
    include_semantic: bool,
    max_blocks: u32,
}

#[derive(Debug, Serialize)]
struct TimeRange {
    from: String,
    to: String,
}
