// Connector Manager - Routes tool calls to appropriate connector
use super::*;
use crate::{
    config::McpConfig,
    context::*,
    errors::{McpError, McpResult},
    protocol::McpTool,
    security_client::SecurityClient,
};
use conhub_database::Database;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ConnectorManager {
    connectors: HashMap<String, Arc<dyn Connector>>,
}

impl ConnectorManager {
    pub async fn new(database: Database, config: &McpConfig) -> anyhow::Result<Self> {
        let mut connectors: HashMap<String, Arc<dyn Connector>> = HashMap::new();
        
        let security_client = Arc::new(SecurityClient::new(database.clone()));
        let cache = database.cache().cloned();
        
        // Initialize GitHub connector
        if config.is_connector_enabled("github") {
            let github = github::GitHubConnector::new(
                config.github_api_base.clone(),
                security_client.clone(),
                cache.clone(),
            );
            connectors.insert("github".to_string(), Arc::new(github));
        }
        
        // Initialize GitLab connector
        if config.is_connector_enabled("gitlab") {
            let gitlab = gitlab::GitLabConnector::new(
                config.gitlab_base_url.clone().unwrap_or_else(|| "https://gitlab.com".to_string()),
                security_client.clone(),
                cache.clone(),
            );
            connectors.insert("gitlab".to_string(), Arc::new(gitlab));
        }
        
        // Initialize Bitbucket connector
        if config.is_connector_enabled("bitbucket") {
            let bitbucket = bitbucket::BitbucketConnector::new(
                config.bitbucket_base_url.clone(),
                security_client.clone(),
                cache.clone(),
            );
            connectors.insert("bitbucket".to_string(), Arc::new(bitbucket));
        }
        
        // Initialize Local FS connector
        if config.is_connector_enabled("fs") {
            let local_fs = local_fs::LocalFsConnector::new(
                config.fs_root_paths.clone(),
                config.fs_ignore_patterns.clone(),
            );
            connectors.insert("fs".to_string(), Arc::new(local_fs));
        }
        
        // Initialize Google Drive connector
        if config.is_connector_enabled("gdrive") {
            let gdrive = google_drive::GoogleDriveConnector::new(
                security_client.clone(),
                cache.clone(),
            );
            connectors.insert("gdrive".to_string(), Arc::new(gdrive));
        }
        
        // Initialize Dropbox connector
        if config.is_connector_enabled("dropbox") {
            let dropbox = dropbox::DropboxConnector::new(
                security_client.clone(),
                cache.clone(),
            );
            connectors.insert("dropbox".to_string(), Arc::new(dropbox));
        }
        
        // Initialize Notion connector
        if config.is_connector_enabled("notion") {
            let notion = notion::NotionConnector::new(
                security_client.clone(),
                cache.clone(),
            );
            connectors.insert("notion".to_string(), Arc::new(notion));
        }
        
        // Initialize Memory connector (always enabled - this is core functionality)
        let decision_engine_url = std::env::var("DECISION_ENGINE_URL")
            .unwrap_or_else(|_| "http://localhost:3016".to_string());
        let memory_connector = memory::MemoryConnector::new(decision_engine_url);
        connectors.insert("memory".to_string(), Arc::new(memory_connector));
        
        Ok(Self { connectors })
    }
    
    pub fn connector_count(&self) -> usize {
        self.connectors.len()
    }
    
    /// List all tools from all connectors
    pub fn list_all_tools(&self) -> Vec<McpTool> {
        let mut tools = Vec::new();
        for connector in self.connectors.values() {
            tools.extend(connector.list_tools());
        }
        tools
    }
    
    /// List all resources from all connectors
    pub fn list_all_resources(&self) -> Vec<ResourceDescriptor> {
        let mut resources = Vec::new();
        for connector in self.connectors.values() {
            resources.extend(connector.list_resources());
        }
        resources
    }
    
    /// Call a tool - routes to appropriate connector based on prefix
    /// Tool names are: "connector.tool_name" (e.g. "github.list_repositories")
    pub async fn call_tool(&self, fully_qualified_name: &str, args: serde_json::Value) -> McpResult<serde_json::Value> {
        let parts: Vec<&str> = fully_qualified_name.splitn(2, '.').collect();
        
        if parts.len() != 2 {
            return Err(McpError::InvalidArguments(
                format!("Tool name must be in format 'connector.tool': {}", fully_qualified_name)
            ));
        }
        
        let (connector_id, tool_name) = (parts[0], parts[1]);
        
        let connector = self.connectors.get(connector_id)
            .ok_or_else(|| McpError::ToolNotFound(
                format!("Connector not found or disabled: {}", connector_id)
            ))?;
        
        connector.call_tool(tool_name, args).await
    }
    
    /// Read a resource - routes based on URI prefix
    pub async fn read_resource(&self, uri: &str) -> McpResult<ResourceContent> {
        // Parse URI to extract connector (e.g. "github://..." or "fs://...")
        if let Some(colon_pos) = uri.find("://") {
            let connector_id = &uri[..colon_pos];
            
            let connector = self.connectors.get(connector_id)
                .ok_or_else(|| McpError::ToolNotFound(
                    format!("Connector not found: {}", connector_id)
                ))?;
            
            connector.read_resource(uri).await
        } else {
            Err(McpError::InvalidArguments(
                format!("Invalid resource URI format: {}", uri)
            ))
        }
    }
}
