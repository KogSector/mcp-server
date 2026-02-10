//! Zilliz Cloud (Milvus) vector search connector
//!
//! Performs vector similarity search for hybrid retrieval.

use serde::{Deserialize, Serialize};
use anyhow::Result;

pub struct MilvusSearchConnector {
    client: reqwest::Client,
    endpoint: String,
    token: String,
    collection_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    pub id: String,
    pub score: f32,
    pub source_id: String,
    pub chunk_id: String,
    pub workspace_id: String,
    pub content_type: String,
    pub filename: String,
    pub blob_path: String,
}

#[derive(Debug, Serialize)]
struct SearchRequest {
    #[serde(rename = "collectionName")]
    collection_name: String,
    vector: Vec<f32>,
    limit: usize,
    #[serde(rename = "outputFields")]
    output_fields: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    code: i32,
    data: Option<Vec<SearchHit>>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchHit {
    id: serde_json::Value,
    distance: f32,
    source_id: Option<String>,
    chunk_id: Option<String>,
    workspace_id: Option<String>,
    content_type: Option<String>,
    filename: Option<String>,
    blob_path: Option<String>,
}

impl MilvusSearchConnector {
    pub fn new(endpoint: &str, token: &str, collection_name: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            endpoint: endpoint.trim_end_matches('/').to_string(),
            token: token.to_string(),
            collection_name: collection_name.to_string(),
        }
    }

    pub fn from_env() -> Option<Self> {
        let endpoint = std::env::var("ZILLIZ_ENDPOINT").ok()?;
        let token = std::env::var("ZILLIZ_TOKEN").ok()?;
        let collection = std::env::var("ZILLIZ_COLLECTION_NAME")
            .unwrap_or_else(|_| "confuse_embeddings".to_string());
        Some(Self::new(&endpoint, &token, &collection))
    }

    /// Search for vectors similar to the query vector
    pub async fn search(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        workspace_id: Option<&str>,
        content_type: Option<&str>,
    ) -> Result<Vec<VectorSearchResult>> {
        let mut filters = vec![];
        if let Some(ws) = workspace_id {
            filters.push(format!("workspace_id == \"{}\"", ws));
        }
        if let Some(ct) = content_type {
            filters.push(format!("content_type == \"{}\"", ct));
        }

        let request = SearchRequest {
            collection_name: self.collection_name.clone(),
            vector: query_vector,
            limit,
            output_fields: vec![
                "source_id".into(), "chunk_id".into(), "workspace_id".into(),
                "content_type".into(), "filename".into(), "blob_path".into(),
            ],
            filter: if filters.is_empty() { None } else { Some(filters.join(" && ")) },
        };

        let url = format!("{}/v2/vectordb/entities/search", self.endpoint);
        let response: SearchResponse = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        if response.code != 0 {
            anyhow::bail!("Milvus search error: {}", response.message.unwrap_or_default());
        }

        Ok(response.data.unwrap_or_default().into_iter().map(|hit| {
            VectorSearchResult {
                id: match hit.id { serde_json::Value::String(s) => s, other => other.to_string() },
                score: hit.distance,
                source_id: hit.source_id.unwrap_or_default(),
                chunk_id: hit.chunk_id.unwrap_or_default(),
                workspace_id: hit.workspace_id.unwrap_or_default(),
                content_type: hit.content_type.unwrap_or_default(),
                filename: hit.filename.unwrap_or_default(),
                blob_path: hit.blob_path.unwrap_or_default(),
            }
        }).collect())
    }

    pub async fn health_check(&self) -> Result<()> {
        let url = format!("{}/v2/vectordb/collections/list", self.endpoint);
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .body("{}")
            .send()
            .await?;
        if !response.status().is_success() {
            anyhow::bail!("Milvus health check failed");
        }
        Ok(())
    }
}
