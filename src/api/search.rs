//! Search API endpoints for FalcorDB vector search

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::search::falcordb::{FalcorDBSearchService, SearchFilters};

/// Semantic search request
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub workspace_id: Option<String>,
    pub limit: Option<usize>,
    pub similarity_threshold: Option<f32>,
}

/// Hybrid search request
#[derive(Debug, Deserialize)]
pub struct HybridSearchRequest {
    pub query: String,
    pub workspace_id: Option<String>,
    pub limit: Option<usize>,
    pub include_related: Option<bool>,
    pub max_depth: Option<usize>,
}

/// Search result item
#[derive(Debug, Serialize)]
pub struct SearchResultItem {
    pub chunk_id: String,
    pub text: String,
    pub source: String,
    pub document_id: String,
    pub similarity_score: f32,
    pub chunk_index: usize,
    pub metadata: serde_json::Value,
}

/// Search response
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResultItem>,
    pub total: usize,
    pub query_info: QueryInfo,
}

/// Query information
#[derive(Debug, Serialize)]
pub struct QueryInfo {
    pub query: String,
    pub workspace_id: Option<String>,
    pub limit: usize,
    pub threshold: f32,
    pub search_time_ms: u64,
}

/// Hybrid search result item
#[derive(Debug, Serialize)]
pub struct HybridSearchResultItem {
    pub chunk_id: String,
    pub text: String,
    pub source: String,
    pub document_id: String,
    pub vector_score: f32,
    pub graph_score: f32,
    pub combined_score: f32,
    pub chunk_index: usize,
    pub related_chunks: Vec<RelatedChunkInfo>,
    pub entities: Vec<EntityInfo>,
}

/// Related chunk information
#[derive(Debug, Serialize)]
pub struct RelatedChunkInfo {
    pub chunk_id: String,
    pub relationship_type: String,
    pub score: f32,
}

/// Entity information
#[derive(Debug, Serialize)]
pub struct EntityInfo {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub mention_count: usize,
}

/// Hybrid search response
#[derive(Debug, Serialize)]
pub struct HybridSearchResponse {
    pub results: Vec<HybridSearchResultItem>,
    pub related_entities: Vec<String>,
    pub graph_connections: usize,
    pub total: usize,
    pub query_info: QueryInfo,
}

/// Application state
#[derive(Clone)]
pub struct AppState {
    pub search_service: Arc<FalcorDBSearchService>,
}

/// Semantic search endpoint
async fn semantic_search(
    State(state): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, String)> {
    let start = std::time::Instant::now();
    
    info!(
        "Semantic search request: query={}, workspace={:?}, limit={:?}",
        req.query, req.workspace_id, req.limit
    );
    
    // Validate query
    if req.query.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Query cannot be empty".to_string(),
        ));
    }
    
    let limit = req.limit.unwrap_or(10).min(50).max(1);
    let threshold = req.similarity_threshold.unwrap_or(0.75).clamp(0.0, 1.0);
    
    // Generate embedding for query (placeholder - should call embeddings service)
    let query_embedding = generate_query_embedding(&req.query).await
        .map_err(|e| {
            error!("Failed to generate embedding: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Embedding generation failed: {}", e))
        })?;
    
    // Build filters
    let filters = if let Some(workspace_id) = &req.workspace_id {
        Some(SearchFilters {
            workspace_id: Some(workspace_id.clone()),
            ..Default::default()
        })
    } else {
        None
    };
    
    // Perform vector search
    let results = state
        .search_service
        .similarity_search(query_embedding, limit, threshold, filters)
        .await
        .map_err(|e| {
            error!("Search failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Search failed: {}", e))
        })?;
    
    let search_time_ms = start.elapsed().as_millis() as u64;
    
    // Convert results
    let search_results: Vec<SearchResultItem> = results
        .iter()
        .map(|r| SearchResultItem {
            chunk_id: r.chunk_id.to_string(),
            text: r.chunk_text.clone(),
            source: r.source_id.clone(),
            document_id: r.document_id.to_string(),
            similarity_score: r.similarity_score,
            chunk_index: r.chunk_index,
            metadata: r.metadata.clone(),
        })
        .collect();
    
    let total = search_results.len();
    
    info!(
        "Semantic search completed: query={}, results={}, time={}ms",
        req.query, total, search_time_ms
    );
    
    Ok(Json(SearchResponse {
        results: search_results,
        total,
        query_info: QueryInfo {
            query: req.query,
            workspace_id: req.workspace_id,
            limit,
            threshold,
            search_time_ms,
        },
    }))
}

/// Hybrid search endpoint
async fn hybrid_search(
    State(state): State<AppState>,
    Json(req): Json<HybridSearchRequest>,
) -> Result<Json<HybridSearchResponse>, (StatusCode, String)> {
    let start = std::time::Instant::now();
    
    info!(
        "Hybrid search request: query={}, workspace={:?}, limit={:?}, max_depth={:?}",
        req.query, req.workspace_id, req.limit, req.max_depth
    );
    
    // Validate query
    if req.query.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Query cannot be empty".to_string(),
        ));
    }
    
    let limit = req.limit.unwrap_or(10).min(50).max(1);
    let max_depth = req.max_depth.unwrap_or(2).min(3).max(1);
    
    // Generate embedding for query
    let query_embedding = generate_query_embedding(&req.query).await
        .map_err(|e| {
            error!("Failed to generate embedding: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Embedding generation failed: {}", e))
        })?;
    
    // Perform hybrid search
    let results = state
        .search_service
        .hybrid_search(query_embedding, limit, max_depth)
        .await
        .map_err(|e| {
            error!("Hybrid search failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Hybrid search failed: {}", e))
        })?;
    
    let search_time_ms = start.elapsed().as_millis() as u64;
    
    // Convert results
    let hybrid_results: Vec<HybridSearchResultItem> = results
        .iter()
        .map(|r| HybridSearchResultItem {
            chunk_id: r.vector_result.chunk_id.to_string(),
            text: r.vector_result.chunk_text.clone(),
            source: r.vector_result.source_id.clone(),
            document_id: r.vector_result.document_id.to_string(),
            vector_score: r.vector_result.similarity_score,
            graph_score: r.combined_score - (r.vector_result.similarity_score * 0.7),
            combined_score: r.combined_score,
            chunk_index: r.vector_result.chunk_index,
            related_chunks: r
                .related_chunks
                .iter()
                .map(|rc| RelatedChunkInfo {
                    chunk_id: rc.chunk_id.to_string(),
                    relationship_type: rc.relationship_type.clone(),
                    score: rc.relationship_score,
                })
                .collect(),
            entities: r
                .entities
                .iter()
                .map(|e| EntityInfo {
                    id: e.id.clone(),
                    name: e.name.clone(),
                    entity_type: e.entity_type.clone(),
                    mention_count: e.mention_count,
                })
                .collect(),
        })
        .collect();
    
    // Extract unique entities
    let related_entities: Vec<String> = results
        .iter()
        .flat_map(|r| r.entities.iter().map(|e| e.name.clone()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    
    let graph_connections: usize = results
        .iter()
        .map(|r| r.related_chunks.len())
        .sum();
    
    let total = hybrid_results.len();
    
    info!(
        "Hybrid search completed: query={}, results={}, entities={}, connections={}, time={}ms",
        req.query, total, related_entities.len(), graph_connections, search_time_ms
    );
    
    Ok(Json(HybridSearchResponse {
        results: hybrid_results,
        related_entities,
        graph_connections,
        total,
        query_info: QueryInfo {
            query: req.query,
            workspace_id: req.workspace_id,
            limit,
            threshold: 0.75,
            search_time_ms,
        },
    }))
}

/// Health check endpoint
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "mcp-server",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Placeholder for embedding generation (should call embeddings-service)
async fn generate_query_embedding(query: &str) -> Result<Vec<f32>, String> {
    // TODO: Call embeddings-service gRPC endpoint
    // For now, return a dummy 384-dimensional vector
    info!("Generating embedding for query: {}", query);
    
    // This should be replaced with actual embeddings-service call
    Ok(vec![0.0; 384])
}

/// Create search routes
pub fn search_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/search/semantic", post(semantic_search))
        .route("/api/v1/search/hybrid", post(hybrid_search))
        .route("/health", get(health))
        .with_state(state)
}
