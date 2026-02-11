// MCP Service Configuration
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub service_port: u16,
    pub host: String,
    
    // Search and retrieval service URLs
    pub embeddings_service_url: Option<String>,
    pub relation_graph_url: Option<String>,
    pub decision_engine_url: Option<String>,
    pub ollama_url: Option<String>,
    
    // Azure Blob Storage configuration
    pub azure_blob_connection_string: Option<String>,
    pub azure_blob_container: String,
    
    // ChromaDB for vector storage
    pub chroma_api_key: Option<String>,
    pub chroma_collection_id: String,
    
    // Timeouts
    pub request_timeout_secs: u64,
    pub cache_ttl_secs: u64,
    
    // Rate limiting
    pub rate_limit_per_minute: u32,
}

impl McpConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            service_port: std::env::var("MCP_SERVICE_PORT")
                .unwrap_or_else(|_| "3004".to_string())
                .parse()?,
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            
            // Search and retrieval services
            embeddings_service_url: std::env::var("EMBEDDINGS_SERVICE_URL").ok(),
            relation_graph_url: std::env::var("RELATION_GRAPH_URL").ok(),
            decision_engine_url: std::env::var("DECISION_ENGINE_URL").ok(),
            ollama_url: std::env::var("OLLAMA_URL").ok(),
            
            // Azure Blob Storage
            azure_blob_connection_string: std::env::var("AZURE_BLOB_CONNECTION_STRING").ok(),
            azure_blob_container: std::env::var("AZURE_BLOB_CONTAINER")
                .unwrap_or_else(|_| "confuse-chunks".to_string()),
            
            // ChromaDB
            chroma_api_key: std::env::var("CHROMA_API_KEY").ok(),
            chroma_collection_id: std::env::var("CHROMA_COLLECTION_ID")
                .unwrap_or_else(|_| "e1376d02-4afd-41ec-993b-ba03e7c41ceb".to_string()),
            
            request_timeout_secs: std::env::var("REQUEST_TIMEOUT_SECS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()?,
            cache_ttl_secs: std::env::var("CACHE_TTL_SECS")
                .unwrap_or_else(|_| "300".to_string())
                .parse()?,
            
            rate_limit_per_minute: std::env::var("RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "60".to_string())
                .parse()?,
        })
    }
}
