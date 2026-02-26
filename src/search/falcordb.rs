//! FalcorDB Vector Search Service
//!
//! Provides vector similarity search and hybrid search using FalcorDB (Neo4j)
//! with native vector indexing capabilities.

use anyhow::{Context, Result};
use neo4rs::{Graph, Query};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;

/// FalcorDB search service
pub struct FalcorDBSearchService {
    graph: Arc<Graph>,
}

impl FalcorDBSearchService {
    /// Create a new FalcorDB search service
    pub async fn new(uri: &str, username: &str, password: &str) -> Result<Self> {
        info!("Connecting to FalcorDB at {}", uri);
        
        let graph = Graph::new(uri, username, password)
            .await
            .context("Failed to connect to FalcorDB")?;
        
        info!("Successfully connected to FalcorDB");
        
        Ok(Self {
            graph: Arc::new(graph),
        })
    }
    
    /// Perform vector similarity search
    pub async fn similarity_search(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        threshold: f32,
        filters: Option<SearchFilters>,
    ) -> Result<Vec<VectorSearchResult>> {
        debug!(
            "Performing similarity search: limit={}, threshold={}, filters={:?}",
            limit, threshold, filters
        );
        
        // Build Cypher query
        let mut query_str = format!(
            r#"
            CALL db.index.vector.queryNodes('vector_chunk_embedding', $limit, $query_vector)
            YIELD node, score
            WHERE score >= $threshold
            "#
        );
        
        // Apply filters
        if let Some(ref filters) = filters {
            if filters.workspace_id.is_some() {
                query_str.push_str(" AND node.workspace_id = $workspace_id");
            }
        }
        
        query_str.push_str(
            r#"
            RETURN node.id as chunk_id,
                   node.chunk_text as chunk_text,
                   node.document_id as document_id,
                   node.source_id as source_id,
                   node.chunk_index as chunk_index,
                   node.metadata as metadata,
                   score as similarity_score
            ORDER BY score DESC
            "#
        );
        
        let mut query = Query::new(query_str)
            .param("query_vector", query_vector)
            .param("limit", limit as i64)
            .param("threshold", threshold);
        
        if let Some(filters) = filters {
            if let Some(workspace_id) = filters.workspace_id {
                query = query.param("workspace_id", workspace_id);
            }
        }
        
        let mut result = self.graph.execute(query).await
            .context("Failed to execute similarity search")?;
        
        let mut results = Vec::new();
        
        while let Some(row) = result.next().await
            .context("Failed to fetch search result")? {
            
            let chunk_id_str: String = row.get("chunk_id")
                .context("Missing chunk_id")?;
            let document_id_str: String = row.get("document_id")
                .context("Missing document_id")?;
            let metadata_str: String = row.get("metadata")
                .context("Missing metadata")?;
            
            results.push(VectorSearchResult {
                chunk_id: Uuid::parse_str(&chunk_id_str)
                    .context("Invalid chunk_id UUID")?,
                chunk_text: row.get("chunk_text")
                    .context("Missing chunk_text")?,
                document_id: Uuid::parse_str(&document_id_str)
                    .context("Invalid document_id UUID")?,
                source_id: row.get("source_id")
                    .context("Missing source_id")?,
                similarity_score: row.get("similarity_score")
                    .context("Missing similarity_score")?,
                chunk_index: row.get::<i64>("chunk_index")
                    .context("Missing chunk_index")? as usize,
                metadata: serde_json::from_str(&metadata_str)
                    .context("Failed to parse metadata")?,
            });
        }
        
        info!("Similarity search completed: {} results", results.len());
        
        Ok(results)
    }
    
    /// Perform hybrid search (vector + graph)
    pub async fn hybrid_search(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        max_depth: usize,
    ) -> Result<Vec<HybridSearchResult>> {
        debug!(
            "Performing hybrid search: limit={}, max_depth={}",
            limit, max_depth
        );
        
        // Step 1: Vector similarity search
        let vector_results = self
            .similarity_search(query_vector, limit * 2, 0.75, None)
            .await?;
        
        if vector_results.is_empty() {
            info!("Hybrid search: no vector results found");
            return Ok(Vec::new());
        }
        
        // Step 2: Graph traversal for each result
        let mut hybrid_results = Vec::new();
        
        for vector_result in vector_results {
            let related_chunks = self
                .get_related_chunks(vector_result.chunk_id, max_depth)
                .await?;
            
            let entities = self
                .get_chunk_entities(vector_result.chunk_id)
                .await?;
            
            // Calculate graph score
            let graph_score = if related_chunks.is_empty() {
                0.0
            } else {
                related_chunks.iter().map(|c| c.relationship_score).sum::<f32>()
                    / related_chunks.len() as f32
            };
            
            // Combined score: 70% vector, 30% graph
            let combined_score = (vector_result.similarity_score * 0.7) + (graph_score * 0.3);
            
            hybrid_results.push(HybridSearchResult {
                vector_result,
                related_chunks,
                entities,
                combined_score,
            });
        }
        
        // Sort by combined score
        hybrid_results.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Limit results
        hybrid_results.truncate(limit);
        
        info!("Hybrid search completed: {} results", hybrid_results.len());
        
        Ok(hybrid_results)
    }
    
    /// Get related chunks through graph traversal
    async fn get_related_chunks(
        &self,
        chunk_id: Uuid,
        max_depth: usize,
    ) -> Result<Vec<RelatedChunk>> {
        let query = Query::new(format!(
            r#"
            MATCH (vc:Vector_Chunk {{id: $chunk_id}})-[r:SIMILAR_TO|RELATED_TO|NEXT_CHUNK*1..{}]-(related:Vector_Chunk)
            RETURN related.id as chunk_id,
                   type(r[0]) as relationship_type,
                   1.0 / size(r) as relationship_score
            LIMIT 50
            "#,
            max_depth
        ))
        .param("chunk_id", chunk_id.to_string());
        
        let mut result = self.graph.execute(query).await
            .context("Failed to get related chunks")?;
        
        let mut related = Vec::new();
        
        while let Some(row) = result.next().await
            .context("Failed to fetch related chunk")? {
            
            let chunk_id_str: String = row.get("chunk_id")
                .context("Missing chunk_id")?;
            
            related.push(RelatedChunk {
                chunk_id: Uuid::parse_str(&chunk_id_str)
                    .context("Invalid chunk_id UUID")?,
                relationship_type: row.get("relationship_type")
                    .context("Missing relationship_type")?,
                relationship_score: row.get("relationship_score")
                    .context("Missing relationship_score")?,
            });
        }
        
        Ok(related)
    }
    
    /// Get entities associated with a chunk
    async fn get_chunk_entities(&self, chunk_id: Uuid) -> Result<Vec<Entity>> {
        let query = Query::new(
            r#"
            MATCH (vc:Vector_Chunk {id: $chunk_id})-[r:CONTAINS_ENTITY]->(e:Entity)
            RETURN e.id as id,
                   e.name as name,
                   e.type as entity_type,
                   r.mention_count as mention_count
            LIMIT 20
            "#
        )
        .param("chunk_id", chunk_id.to_string());
        
        let mut result = self.graph.execute(query).await
            .context("Failed to get chunk entities")?;
        
        let mut entities = Vec::new();
        
        while let Some(row) = result.next().await
            .context("Failed to fetch entity")? {
            
            entities.push(Entity {
                id: row.get("id").context("Missing entity id")?,
                name: row.get("name").context("Missing entity name")?,
                entity_type: row.get("entity_type").context("Missing entity_type")?,
                mention_count: row.get::<i64>("mention_count")
                    .context("Missing mention_count")? as usize,
            });
        }
        
        Ok(entities)
    }
}

/// Search filters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilters {
    pub workspace_id: Option<String>,
}

/// Vector search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    pub chunk_id: Uuid,
    pub chunk_text: String,
    pub document_id: Uuid,
    pub source_id: String,
    pub similarity_score: f32,
    pub chunk_index: usize,
    pub metadata: serde_json::Value,
}

/// Hybrid search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResult {
    pub vector_result: VectorSearchResult,
    pub related_chunks: Vec<RelatedChunk>,
    pub entities: Vec<Entity>,
    pub combined_score: f32,
}

/// Related chunk information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedChunk {
    pub chunk_id: Uuid,
    pub relationship_type: String,
    pub relationship_score: f32,
}

/// Entity information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub mention_count: usize,
}
