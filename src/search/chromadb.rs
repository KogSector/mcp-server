//! ChromaDB Cloud vector search connector
//!
//! Performs vector similarity search for hybrid retrieval.

use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::collections::HashMap;

pub struct ChromaSearchService {
    client: reqwest::Client,
    api_key: String,
    collection_id: String,
    base_url: String,
    tenant: String,
    database: String,
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
struct ChromaQueryRequest {
    query_embeddings: Vec<Vec<f32>>,
    n_results: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#where: Option<HashMap<String, serde_json::Value>>,
    include: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ChromaQueryResponse {
    ids: Vec<Vec<String>>,
    distances: Option<Vec<Vec<f32>>>,
    metadatas: Option<Vec<Vec<Option<HashMap<String, serde_json::Value>>>>>,
}

impl ChromaSearchService {
    pub fn new(api_key: &str, collection_id: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            api_key: api_key.to_string(),
            collection_id: collection_id.to_string(),
            base_url: "https://api.trychroma.com".to_string(),
            tenant: "default_tenant".to_string(),
            database: "default_database".to_string(),
        }
    }

    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("CHROMA_API_KEY").ok()?;
        let collection_id = std::env::var("CHROMA_COLLECTION_ID")
            .unwrap_or_else(|_| "e1376d02-4afd-41ec-993b-ba03e7c41ceb".to_string());
        Some(Self::new(&api_key, &collection_id))
    }

    fn collection_url(&self) -> String {
        format!(
            "{}/api/v2/tenants/{}/databases/{}/collections/{}",
            self.base_url, self.tenant, self.database, self.collection_id
        )
    }

    /// Search for vectors similar to the query vector
    pub async fn search(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        workspace_id: Option<&str>,
        content_type: Option<&str>,
    ) -> Result<Vec<VectorSearchResult>> {
        let where_filter = Self::build_where_filter(workspace_id, content_type);

        let request = ChromaQueryRequest {
            query_embeddings: vec![query_vector],
            n_results: limit,
            r#where: where_filter,
            include: vec![
                "metadatas".to_string(),
                "distances".to_string(),
            ],
        };

        let url = format!("{}/query", self.collection_url());
        let response: ChromaQueryResponse = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        // ChromaDB returns nested arrays (one per query embedding)
        let ids = response.ids.into_iter().next().unwrap_or_default();
        let distances = response.distances.and_then(|d| d.into_iter().next()).unwrap_or_default();
        let metadatas = response.metadatas.and_then(|m| m.into_iter().next()).unwrap_or_default();

        Ok(ids.into_iter().enumerate().map(|(i, id)| {
            let meta = metadatas.get(i).and_then(|m| m.as_ref()).cloned().unwrap_or_default();
            let score = distances.get(i).copied().unwrap_or(0.0);

            VectorSearchResult {
                id,
                score,
                source_id: Self::extract_meta(&meta, "source_id"),
                chunk_id: Self::extract_meta(&meta, "chunk_id"),
                workspace_id: Self::extract_meta(&meta, "workspace_id"),
                content_type: Self::extract_meta(&meta, "content_type"),
                filename: Self::extract_meta(&meta, "filename"),
                blob_path: Self::extract_meta(&meta, "blob_path"),
            }
        }).collect())
    }

    pub async fn health_check(&self) -> Result<()> {
        let url = format!("{}/api/v2/heartbeat", self.base_url);
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
        if !response.status().is_success() {
            anyhow::bail!("ChromaDB health check failed");
        }
        Ok(())
    }

    fn build_where_filter(
        workspace_id: Option<&str>,
        content_type: Option<&str>,
    ) -> Option<HashMap<String, serde_json::Value>> {
        let mut filter = HashMap::new();
        if let Some(ws) = workspace_id {
            filter.insert("workspace_id".to_string(), serde_json::json!(ws));
        }
        if let Some(ct) = content_type {
            filter.insert("content_type".to_string(), serde_json::json!(ct));
        }

        if filter.is_empty() {
            None
        } else if filter.len() == 1 {
            Some(filter)
        } else {
            let conditions: Vec<serde_json::Value> = filter
                .into_iter()
                .map(|(k, v)| serde_json::json!({ k: v }))
                .collect();
            let mut and_filter = HashMap::new();
            and_filter.insert("$and".to_string(), serde_json::json!(conditions));
            Some(and_filter)
        }
    }

    fn extract_meta(meta: &HashMap<String, serde_json::Value>, key: &str) -> String {
        meta.get(key)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    }
}
