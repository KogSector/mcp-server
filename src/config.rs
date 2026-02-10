// MCP Service Configuration
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub service_port: u16,
    pub host: String,
    
    // Feature flags per connector
    pub enabled_connectors: HashMap<String, bool>,
    
    // Provider configurations
    pub github_api_base: String,
    pub gitlab_base_url: Option<String>,
    pub bitbucket_base_url: String,
    
    // Local FS config
    pub fs_root_paths: Vec<String>,
    pub fs_ignore_patterns: Vec<String>,
    
    // Timeouts
    pub request_timeout_secs: u64,
    pub cache_ttl_secs: u64,
    
    // Rate limiting
    pub rate_limit_per_minute: u32,

    // Storage backends for hybrid retrieval
    pub zilliz_endpoint: Option<String>,
    pub zilliz_token: Option<String>,
    pub zilliz_collection_name: String,
    pub azure_blob_connection_string: Option<String>,
    pub azure_blob_container: String,
    pub neo4j_uri: Option<String>,
    pub neo4j_user: String,
    pub neo4j_password: Option<String>,
    pub embeddings_service_url: Option<String>,
}

impl McpConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            service_port: std::env::var("MCP_SERVICE_PORT")
                .unwrap_or_else(|_| "3004".to_string())
                .parse()?,
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            
            enabled_connectors: Self::parse_enabled_connectors(),
            
            github_api_base: std::env::var("GITHUB_API_BASE")
                .unwrap_or_else(|_| "https://api.github.com".to_string()),
            gitlab_base_url: std::env::var("GITLAB_BASE_URL").ok(),
            bitbucket_base_url: std::env::var("BITBUCKET_BASE_URL")
                .unwrap_or_else(|_| "https://api.bitbucket.org/2.0".to_string()),
            
            fs_root_paths: std::env::var("FS_ROOT_PATHS")
                .unwrap_or_default()
                .split(',')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
            fs_ignore_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                ".next".to_string(),
            ],
            
            request_timeout_secs: std::env::var("REQUEST_TIMEOUT_SECS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()?,
            cache_ttl_secs: std::env::var("CACHE_TTL_SECS")
                .unwrap_or_else(|_| "300".to_string())
                .parse()?,
            
            rate_limit_per_minute: std::env::var("RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "60".to_string())
                .parse()?,

            // Storage backends
            zilliz_endpoint: std::env::var("ZILLIZ_ENDPOINT").ok(),
            zilliz_token: std::env::var("ZILLIZ_TOKEN").ok(),
            zilliz_collection_name: std::env::var("ZILLIZ_COLLECTION_NAME")
                .unwrap_or_else(|_| "confuse_embeddings".to_string()),
            azure_blob_connection_string: std::env::var("AZURE_BLOB_CONNECTION_STRING").ok(),
            azure_blob_container: std::env::var("AZURE_BLOB_CONTAINER")
                .unwrap_or_else(|_| "confuse-chunks".to_string()),
            neo4j_uri: std::env::var("NEO4J_URI").ok(),
            neo4j_user: std::env::var("NEO4J_USER")
                .unwrap_or_else(|_| "neo4j".to_string()),
            neo4j_password: std::env::var("NEO4J_PASSWORD").ok(),
            embeddings_service_url: std::env::var("EMBEDDINGS_SERVICE_URL").ok(),
        })
    }
    
    fn parse_enabled_connectors() -> HashMap<String, bool> {
        let mut enabled = HashMap::new();
        
        let connectors = vec!["github", "gitlab", "bitbucket", "gdrive", "dropbox", "fs", "notion"];
        
        for connector in connectors {
            let env_key = format!("ENABLE_{}", connector.to_uppercase());
            let is_enabled = std::env::var(&env_key)
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true);
            enabled.insert(connector.to_string(), is_enabled);
        }
        
        enabled
    }
    
    pub fn is_connector_enabled(&self, connector: &str) -> bool {
        self.enabled_connectors.get(connector).copied().unwrap_or(false)
    }
}
