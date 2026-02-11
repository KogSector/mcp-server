//! Context Connector - Intelligent Hybrid Search for AI Agents
//!
//! This connector orchestrates hybrid vector + graph search to provide
//! highly relevant context to AI agents, significantly reducing hallucination.
//!
//! ## Architecture
//!
//! The Context Connector combines two search modalities:
//! - **Vector Search** (via Embeddings service): Semantic similarity
//! - **Graph Search** (via Relation-Graph service): Structural relationships
//!
//! ## Why Hybrid Search?
//!
//! Vector search excels at semantic similarity but misses structural relationships.
//! Graph search captures dependencies but requires exact entity matching.
//! By combining both, we get complete context with code + dependencies + tests + docs.
//!
//! ## Example
//!
//! Query: "How does JWT authentication work?"
//!
//! Vector finds: jwt_validator.rs, middleware.rs (semantic match)
//! Graph adds: config.rs (imports), auth_test.rs (tests), auth.md (docs)
//! Result: Complete context for the AI agent

use crate::{mcp::McpTool, errors::{McpError, McpResult}};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use super::service_trait::SearchService;

/// Configuration for hybrid search ranking
#[derive(Debug, Clone)]
pub struct RankingWeights {
    /// Weight for semantic similarity (0.0 - 1.0)
    pub semantic: f32,
    /// Weight for graph centrality (0.0 - 1.0)
    pub graph: f32,
    /// Weight for relationship depth (0.0 - 1.0)
    pub relationship: f32,
    /// Weight for recency (0.0 - 1.0)
    pub recency: f32,
    /// Weight for diversity bonus (0.0 - 1.0)
    pub diversity: f32,
}

impl Default for RankingWeights {
    fn default() -> Self {
        Self {
            semantic: 0.35,
            graph: 0.25,
            relationship: 0.20,
            recency: 0.10,
            diversity: 0.10,
        }
    }
}

/// Hybrid Search Service - Combines embeddings and graph search
pub struct HybridSearchService {
    embeddings_url: String,
    graph_url: String,
    ollama_url: String,
    expansion_model: String,
    client: reqwest::Client,
    weights: RankingWeights,
    max_results: usize,
}

impl HybridSearchService {
    pub fn new(
        embeddings_url: String,
        graph_url: String,
        ollama_url: String,
    ) -> Self {
        Self {
            embeddings_url,
            graph_url,
            ollama_url,
            expansion_model: "qwen2.5:7b".to_string(),
            client: reqwest::Client::new(),
            weights: RankingWeights::default(),
            max_results: 20,
        }
    }
    
    /// Expand query using LLM for semantic enhancement
    async fn expand_query(&self, query: &str) -> McpResult<ExpandedQuery> {
        let prompt = format!(
            r#"You are a code search assistant. Given a user query, expand it with:
1. Semantically similar programming terms
2. Related technical concepts
3. Potential file/function names

Query: "{}"

Respond in JSON format:
{{
  "semantic_terms": ["term1", "term2"],
  "technical_concepts": ["concept1", "concept2"],
  "potential_names": ["name1", "name2"]
}}"#,
            query
        );
        
        let response = self.client
            .post(format!("{}/api/generate", self.ollama_url))
            .json(&json!({
                "model": self.expansion_model,
                "prompt": prompt,
                "stream": false
            }))
            .send()
            .await
            .map_err(|e| McpError::Internal(format!("LLM request failed: {}", e)))?;
        
        if !response.status().is_success() {
            // Fallback to original query if LLM fails
            return Ok(ExpandedQuery {
                original: query.to_string(),
                semantic_terms: Vec::new(),
                technical_concepts: Vec::new(),
                potential_names: Vec::new(),
                combined: query.to_string(),
            });
        }
        
        let result: Value = response.json().await
            .map_err(|e| McpError::Internal(format!("Failed to parse LLM response: {}", e)))?;
        
        // Parse LLM response
        let llm_response = result.get("response")
            .and_then(|v| v.as_str())
            .unwrap_or("{}");
        
        // Try to extract JSON from response
        let expansion: ExpansionResponse = serde_json::from_str(llm_response)
            .unwrap_or_default();
        
        // Combine all terms into expanded query
        let mut all_terms = vec![query.to_string()];
        all_terms.extend(expansion.semantic_terms.clone());
        all_terms.extend(expansion.technical_concepts.clone());
        
        Ok(ExpandedQuery {
            original: query.to_string(),
            semantic_terms: expansion.semantic_terms,
            technical_concepts: expansion.technical_concepts,
            potential_names: expansion.potential_names,
            combined: all_terms.join(" "),
        })
    }
    
    /// Perform vector search via embeddings service
    async fn vector_search(&self, query: &str, limit: usize) -> McpResult<Vec<SearchResult>> {
        let response = self.client
            .post(format!("{}/api/v1/search", self.embeddings_url))
            .json(&json!({
                "query": query,
                "limit": limit,
                "include_content": true
            }))
            .send()
            .await
            .map_err(|e| McpError::Internal(format!("Vector search failed: {}", e)))?;
        
        if !response.status().is_success() {
            return Ok(Vec::new());
        }
        
        let result: VectorSearchResponse = response.json().await
            .unwrap_or_default();
        
        Ok(result.results.into_iter().map(|r| SearchResult {
            id: r.id,
            entity_id: r.entity_id,
            title: r.title.unwrap_or_default(),
            content: r.content,
            path: r.path,
            source: r.source.unwrap_or_else(|| "unknown".to_string()),
            content_type: r.content_type.unwrap_or_else(|| "code".to_string()),
            semantic_score: r.score,
            graph_score: 0.0,
            relationship_depth: 0,
            final_score: 0.0,
            related_ids: Vec::new(),
        }).collect())
    }
    
    /// Perform graph search via relation-graph service
    async fn graph_search(&self, query: &str, limit: usize) -> McpResult<Vec<SearchResult>> {
        let response = self.client
            .post(format!("{}/api/search", self.graph_url))
            .json(&json!({
                "query": query,
                "limit": limit,
                "include_entities": true
            }))
            .send()
            .await
            .map_err(|e| McpError::Internal(format!("Graph search failed: {}", e)))?;
        
        if !response.status().is_success() {
            return Ok(Vec::new());
        }
        
        let result: GraphSearchResponse = response.json().await
            .unwrap_or_default();
        
        Ok(result.entities.unwrap_or_default().into_iter().map(|e| SearchResult {
            id: e.id.clone(),
            entity_id: Some(e.id),
            title: e.name,
            content: e.content.unwrap_or_default(),
            path: e.path,
            source: e.source.unwrap_or_else(|| "graph".to_string()),
            content_type: e.entity_type,
            semantic_score: 0.0,
            graph_score: e.centrality.unwrap_or(0.5),
            relationship_depth: e.depth.unwrap_or(1) as usize,
            final_score: 0.0,
            related_ids: e.related_ids.unwrap_or_default(),
        }).collect())
    }
    
    /// Get related entities via graph traversal
    async fn get_related(&self, entity_ids: &[String], depth: usize) -> McpResult<Vec<SearchResult>> {
        let mut all_related = Vec::new();
        
        for entity_id in entity_ids.iter().take(5) {  // Limit to avoid too many requests
            let response = self.client
                .get(format!("{}/api/graph/entities/{}/neighbors", self.graph_url, entity_id))
                .query(&[("depth", depth.to_string())])
                .send()
                .await
                .map_err(|e| McpError::Internal(format!("Related search failed: {}", e)))?;
            
            if response.status().is_success() {
                if let Ok(result) = response.json::<RelatedResponse>().await {
                    for neighbor in result.neighbors.unwrap_or_default() {
                        all_related.push(SearchResult {
                            id: neighbor.id.clone(),
                            entity_id: Some(neighbor.id),
                            title: neighbor.name,
                            content: neighbor.content.unwrap_or_default(),
                            path: neighbor.path,
                            source: "graph_related".to_string(),
                            content_type: neighbor.entity_type,
                            semantic_score: 0.0,
                            graph_score: neighbor.weight.unwrap_or(0.3),
                            relationship_depth: depth,
                            final_score: 0.0,
                            related_ids: Vec::new(),
                        });
                    }
                }
            }
        }
        
        Ok(all_related)
    }
    
    /// Merge and rank results from both search modalities
    fn merge_and_rank(&self, vector_results: Vec<SearchResult>, graph_results: Vec<SearchResult>) -> Vec<SearchResult> {
        let mut merged: HashMap<String, SearchResult> = HashMap::new();
        
        // Add vector results
        for result in vector_results {
            merged.insert(result.id.clone(), result);
        }
        
        // Merge graph results
        for result in graph_results {
            if let Some(existing) = merged.get_mut(&result.id) {
                // Update graph score for items found in both
                existing.graph_score = result.graph_score;
                existing.relationship_depth = result.relationship_depth;
                existing.related_ids.extend(result.related_ids);
            } else {
                merged.insert(result.id.clone(), result);
            }
        }
        
        // Calculate final scores
        let mut results: Vec<SearchResult> = merged.into_values().collect();
        
        // Track content types for diversity calculation
        let mut type_counts: HashMap<String, usize> = HashMap::new();
        
        for result in &mut results {
            // Calculate base scores
            let semantic = result.semantic_score * self.weights.semantic;
            let graph = result.graph_score * self.weights.graph;
            let relationship = (1.0 / (result.relationship_depth as f32 + 1.0)) * self.weights.relationship;
            let recency = 0.5 * self.weights.recency;  // TODO: Calculate from timestamp
            
            // Diversity bonus: reward less common content types
            let type_count = type_counts.entry(result.content_type.clone()).or_insert(0);
            *type_count += 1;
            let diversity = (1.0 / (*type_count as f32)) * self.weights.diversity;
            
            result.final_score = semantic + graph + relationship + recency + diversity;
        }
        
        // Sort by final score
        results.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Return top results
        results.truncate(self.max_results);
        results
    }
    
    /// Assemble context bundle for AI consumption
    fn assemble_context(&self, results: &[SearchResult], query: &str, context_window: usize) -> ContextBundle {
        let mut bundle = ContextBundle {
            query: query.to_string(),
            items: Vec::new(),
            total_tokens: 0,
            context_window,
        };
        
        let tokens_per_char = 0.25;  // Rough estimate
        
        for result in results {
            let estimated_tokens = (result.content.len() as f32 * tokens_per_char) as usize;
            
            if bundle.total_tokens + estimated_tokens > context_window {
                break;
            }
            
            bundle.items.push(ContextItem {
                id: result.id.clone(),
                title: result.title.clone(),
                content: result.content.clone(),
                path: result.path.clone(),
                content_type: result.content_type.clone(),
                relevance_score: result.final_score,
                tokens: estimated_tokens,
            });
            
            bundle.total_tokens += estimated_tokens;
        }
        
        bundle
    }
}

// =============================================================================
// Response Types
// =============================================================================

#[derive(Debug, Serialize, Deserialize, Default)]
struct ExpansionResponse {
    #[serde(default)]
    semantic_terms: Vec<String>,
    #[serde(default)]
    technical_concepts: Vec<String>,
    #[serde(default)]
    potential_names: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExpandedQuery {
    original: String,
    semantic_terms: Vec<String>,
    technical_concepts: Vec<String>,
    potential_names: Vec<String>,
    combined: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct VectorSearchResponse {
    #[serde(default)]
    results: Vec<VectorResult>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VectorResult {
    id: String,
    entity_id: Option<String>,
    title: Option<String>,
    content: String,
    path: Option<String>,
    source: Option<String>,
    content_type: Option<String>,
    score: f32,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct GraphSearchResponse {
    #[serde(default)]
    chunks: Vec<Value>,
    entities: Option<Vec<GraphEntity>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GraphEntity {
    id: String,
    name: String,
    entity_type: String,
    content: Option<String>,
    path: Option<String>,
    source: Option<String>,
    centrality: Option<f32>,
    depth: Option<u32>,
    related_ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct RelatedResponse {
    entity: Option<Value>,
    neighbors: Option<Vec<Neighbor>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Neighbor {
    id: String,
    name: String,
    entity_type: String,
    content: Option<String>,
    path: Option<String>,
    weight: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchResult {
    id: String,
    entity_id: Option<String>,
    title: String,
    content: String,
    path: Option<String>,
    source: String,
    content_type: String,
    semantic_score: f32,
    graph_score: f32,
    relationship_depth: usize,
    final_score: f32,
    related_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContextBundle {
    query: String,
    items: Vec<ContextItem>,
    total_tokens: usize,
    context_window: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContextItem {
    id: String,
    title: String,
    content: String,
    path: Option<String>,
    content_type: String,
    relevance_score: f32,
    tokens: usize,
}

// =============================================================================
// Connector Implementation
// =============================================================================

#[async_trait]
impl SearchService for HybridSearchService {
    fn id(&self) -> &'static str {
        "context"
    }
    
    fn list_tools(&self) -> Vec<McpTool> {
        vec![
            McpTool {
                name: "context.search".to_string(),
                description: "Hybrid semantic + graph search with context assembly. Combines vector similarity with knowledge graph traversal for comprehensive results.".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query (natural language or code terms)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max results to return (default: 10)",
                            "default": 10
                        },
                        "context_window": {
                            "type": "integer",
                            "description": "Max tokens for context assembly (default: 8000)",
                            "default": 8000
                        },
                        "expand_query": {
                            "type": "boolean",
                            "description": "Use LLM to expand query with related terms (default: true)",
                            "default": true
                        },
                        "include_related": {
                            "type": "boolean",
                            "description": "Include graph-related entities (default: true)",
                            "default": true
                        }
                    },
                    "required": ["query"]
                })),
            },
            McpTool {
                name: "context.expand".to_string(),
                description: "Expand a query with semantic and domain-specific terms using LLM".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Query to expand"
                        }
                    },
                    "required": ["query"]
                })),
            },
            McpTool {
                name: "context.related".to_string(),
                description: "Get related entities from the knowledge graph with multi-hop traversal".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "entity_id": {
                            "type": "string",
                            "description": "Starting entity ID"
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
                
                let context_window = args.get("context_window")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(8000) as usize;
                
                let expand_query = args.get("expand_query")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                
                let include_related = args.get("include_related")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                
                // 1. Optionally expand the query
                let search_query = if expand_query {
                    let expanded = self.expand_query(query).await?;
                    expanded.combined
                } else {
                    query.to_string()
                };
                
                // 2. Run parallel vector + graph search
                let (vector_results, graph_results) = tokio::join!(
                    self.vector_search(&search_query, limit * 2),
                    self.graph_search(&search_query, limit * 2)
                );
                
                let vector_results = vector_results.unwrap_or_default();
                let graph_results = graph_results.unwrap_or_default();
                
                // 3. Merge and rank results
                let mut ranked = self.merge_and_rank(vector_results.clone(), graph_results.clone());
                
                // 4. Optionally fetch related entities
                if include_related && !ranked.is_empty() {
                    let entity_ids: Vec<String> = ranked.iter()
                        .filter_map(|r| r.entity_id.clone())
                        .take(3)
                        .collect();
                    
                    if !entity_ids.is_empty() {
                        if let Ok(related) = self.get_related(&entity_ids, 1).await {
                            // Add related with lower scores
                            let mut related_scored: Vec<SearchResult> = related.into_iter()
                                .map(|mut r| {
                                    r.final_score = 0.3;  // Lower than direct matches
                                    r
                                })
                                .collect();
                            ranked.append(&mut related_scored);
                        }
                    }
                }
                
                // 5. Assemble context bundle
                let context_bundle = self.assemble_context(&ranked, query, context_window);
                
                // 6. Build response
                let vector_count = vector_results.len();
                let graph_count = graph_results.len();
                
                Ok(json!({
                    "query": query,
                    "total_results": ranked.len(),
                    "vector_matches": vector_count,
                    "graph_matches": graph_count,
                    "context_bundle": context_bundle,
                    "results": ranked.iter().take(limit).map(|r| json!({
                        "id": r.id,
                        "title": r.title,
                        "path": r.path,
                        "content_type": r.content_type,
                        "relevance_score": r.final_score,
                        "semantic_score": r.semantic_score,
                        "graph_score": r.graph_score,
                        "source": r.source
                    })).collect::<Vec<_>>()
                }))
            }
            
            "expand" => {
                let query = args.get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments("Missing 'query' argument".into()))?;
                
                let expanded = self.expand_query(query).await?;
                
                Ok(json!({
                    "original": expanded.original,
                    "semantic_terms": expanded.semantic_terms,
                    "technical_concepts": expanded.technical_concepts,
                    "potential_names": expanded.potential_names,
                    "combined": expanded.combined
                }))
            }
            
            "related" => {
                let entity_id = args.get("entity_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments("Missing 'entity_id' argument".into()))?;
                
                let depth = args.get("depth")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(2) as usize;
                
                let related = self.get_related(&[entity_id.to_string()], depth).await?;
                
                Ok(json!({
                    "entity_id": entity_id,
                    "depth": depth,
                    "related_count": related.len(),
                    "related": related.iter().map(|r| json!({
                        "id": r.id,
                        "title": r.title,
                        "path": r.path,
                        "content_type": r.content_type,
                        "graph_score": r.graph_score
                    })).collect::<Vec<_>>()
                }))
            }
            
            _ => Err(McpError::ToolNotFound(format!("Unknown tool: context.{}", tool))),
        }
    }
}
