// GitHub Connector - Full implementation
use super::Connector;
use crate::{context::*, errors::{McpError, McpResult}, protocol::McpTool, security_client::SecurityClient};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

pub struct GitHubConnector {
    api_base: String,
    security: Arc<SecurityClient>,

    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct GitHubRepo {
    id: u64,
    name: String,
    full_name: String,
    owner: GitHubOwner,
    private: bool,
    description: Option<String>,
    default_branch: String,
    html_url: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct GitHubOwner {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GitHubBranch {
    name: String,
    commit: GitHubCommit,
    protected: bool,
}

#[derive(Debug, Deserialize)]
struct GitHubCommit {
    sha: String,
}

#[derive(Debug, Deserialize)]
struct GitHubContent {
    name: String,
    path: String,
    sha: String,
    size: u64,
    #[serde(rename = "type")]
    item_type: String,
    download_url: Option<String>,
}

impl GitHubConnector {
    pub fn new(api_base: String, security: Arc<SecurityClient>) -> Self {
        Self {
            api_base,
            security,
            cache,
            client: reqwest::Client::new(),
        }
    }
    
    async fn get_token(&self, user_id: Option<&uuid::Uuid>) -> McpResult<String> {
        // For now, use env variable or get from security
        if let Ok(token) = std::env::var("GITHUB_ACCESS_TOKEN") {
            return Ok(token);
        }
        
        if let Some(uid) = user_id {
            if let Some(token) = self.security.get_user_token(uid, "github", "access_token").await.map_err(|e| McpError::Internal(e.to_string()))? {
                return Ok(token);
            }
        }
        
        Err(McpError::Unauthorized("No GitHub token available".to_string()))
    }
    
    async fn list_repositories(&self, args: Value) -> McpResult<Value> {
        let token = self.get_token(None).await?;
        let visibility = args.get("visibility").and_then(|v| v.as_str()).unwrap_or("all");
        
        let url = format!("{}/user/repos?visibility={}&per_page=100", self.api_base, visibility);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", "ConHub-MCP")
            .send()
            .await
            .map_err(|e| McpError::ProviderError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(McpError::ProviderError(format!("GitHub API error: {}", response.status())));
        }
        
        let repos: Vec<GitHubRepo> = response.json().await
            .map_err(|e| McpError::ProviderError(e.to_string()))?;
        
        let descriptors: Vec<RepositoryDescriptor> = repos.into_iter().map(|r| {
            RepositoryDescriptor {
                id: format!("gh:{}", r.full_name),
                provider: "github".to_string(),
                name: r.name,
                owner: r.owner.login,
                visibility: if r.private { "private" } else { "public" }.to_string(),
                default_branch: r.default_branch,
                description: r.description,
                url: r.html_url,
                updated_at: chrono::DateTime::parse_from_rfc3339(&r.updated_at)
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0),
            }
        }).collect();
        
        Ok(serde_json::to_value(descriptors)?)
    }
    
    async fn list_branches(&self, args: Value) -> McpResult<Value> {
        let token = self.get_token(None).await?;
        let repo_id = args.get("repo_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidArguments("Missing repo_id".to_string()))?;
        
        let repo_path = repo_id.strip_prefix("gh:").unwrap_or(repo_id);
        let url = format!("{}/repos/{}/branches", self.api_base, repo_path);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", "ConHub-MCP")
            .send()
            .await
            .map_err(|e| McpError::ProviderError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(McpError::ProviderError(format!("GitHub API error: {}", response.status())));
        }
        
        let branches: Vec<GitHubBranch> = response.json().await
            .map_err(|e| McpError::ProviderError(e.to_string()))?;
        
        let descriptors: Vec<BranchDescriptor> = branches.into_iter().map(|b| {
            BranchDescriptor {
                name: b.name,
                commit_id: b.commit.sha,
                is_default: false, // Would need to check against repo default
                protected: Some(b.protected),
            }
        }).collect();
        
        Ok(serde_json::to_value(descriptors)?)
    }
    
    async fn list_files(&self, args: Value) -> McpResult<Value> {
        let token = self.get_token(None).await?;
        let repo_id = args.get("repo_id").and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidArguments("Missing repo_id".to_string()))?;
        let branch = args.get("branch").and_then(|v| v.as_str()).unwrap_or("main");
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
        
        let repo_path = repo_id.strip_prefix("gh:").unwrap_or(repo_id);
        let url = format!("{}/repos/{}/contents/{}?ref={}", self.api_base, repo_path, path, branch);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", "ConHub-MCP")
            .send()
            .await
            .map_err(|e| McpError::ProviderError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(McpError::ProviderError(format!("GitHub API error: {}", response.status())));
        }
        
        let contents: Vec<GitHubContent> = response.json().await
            .map_err(|e| McpError::ProviderError(e.to_string()))?;
        
        let descriptors: Vec<FileDescriptor> = contents.into_iter().map(|c| {
            FileDescriptor {
                id: format!("{}:{}", repo_id, c.path),
                path: c.path.clone(),
                name: c.name,
                kind: if c.item_type == "dir" { "dir" } else { "file" }.to_string(),
                size: Some(c.size),
                language: Self::detect_language(&c.path),
                sha: Some(c.sha),
                last_modified: None,
                mime_type: None,
            }
        }).collect();
        
        Ok(serde_json::to_value(descriptors)?)
    }
    
    async fn get_file_content(&self, args: Value) -> McpResult<Value> {
        let token = self.get_token(None).await?;
        let repo_id = args.get("repo_id").and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidArguments("Missing repo_id".to_string()))?;
        let branch = args.get("branch").and_then(|v| v.as_str()).unwrap_or("main");
        let path = args.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidArguments("Missing path".to_string()))?;
        
        let repo_path = repo_id.strip_prefix("gh:").unwrap_or(repo_id);
        let url = format!("{}/repos/{}/contents/{}?ref={}", self.api_base, repo_path, path, branch);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", "ConHub-MCP")
            .header("Accept", "application/vnd.github.v3.raw")
            .send()
            .await
            .map_err(|e| McpError::ProviderError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(McpError::ProviderError(format!("GitHub API error: {}", response.status())));
        }
        
        let content = response.text().await
            .map_err(|e| McpError::ProviderError(e.to_string()))?;
        
        Ok(json!({
            "file": {
                "id": format!("{}:{}", repo_id, path),
                "path": path,
                "name": path.split('/').last().unwrap_or(path),
                "kind": "file",
                "language": Self::detect_language(path),
            },
            "content": content
        }))
    }
    
    fn detect_language(path: &str) -> Option<String> {
        path.rsplit('.').next().and_then(|ext| {
            match ext {
                "rs" => Some("rust"),
                "ts" | "tsx" => Some("typescript"),
                "js" | "jsx" => Some("javascript"),
                "py" => Some("python"),
                "go" => Some("go"),
                "java" => Some("java"),
                "cpp" | "cc" | "cxx" => Some("cpp"),
                "c" | "h" => Some("c"),
                "md" => Some("markdown"),
                "json" => Some("json"),
                "yaml" | "yml" => Some("yaml"),
                "toml" => Some("toml"),
                "sh" => Some("bash"),
                _ => None,
            }
        }).map(String::from)
    }
}

#[async_trait]
impl Connector for GitHubConnector {
    fn id(&self) -> &'static str {
        "github"
    }
    
    fn list_tools(&self) -> Vec<McpTool> {
        vec![
            McpTool {
                name: "github.list_repositories".to_string(),
                description: "List GitHub repositories for the authenticated user".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "visibility": {
                            "type": "string",
                            "enum": ["all", "public", "private"],
                            "description": "Filter by repository visibility"
                        }
                    }
                })),
            },
            McpTool {
                name: "github.list_branches".to_string(),
                description: "List branches for a GitHub repository".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "repo_id": {
                            "type": "string",
                            "description": "Repository ID (e.g. gh:owner/repo)"
                        }
                    },
                    "required": ["repo_id"]
                })),
            },
            McpTool {
                name: "github.list_files".to_string(),
                description: "List files and directories in a GitHub repository".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "repo_id": { "type": "string" },
                        "branch": { "type": "string" },
                        "path": { "type": "string" }
                    },
                    "required": ["repo_id"]
                })),
            },
            McpTool {
                name: "github.get_file_content".to_string(),
                description: "Get the content of a file from a GitHub repository".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "repo_id": { "type": "string" },
                        "branch": { "type": "string" },
                        "path": { "type": "string" }
                    },
                    "required": ["repo_id", "path"]
                })),
            },
        ]
    }
    
    async fn call_tool(&self, tool: &str, args: Value) -> McpResult<Value> {
        match tool {
            "list_repositories" => self.list_repositories(args).await,
            "list_branches" => self.list_branches(args).await,
            "list_files" => self.list_files(args).await,
            "get_file_content" => self.get_file_content(args).await,
            _ => Err(McpError::ToolNotFound(format!("Unknown GitHub tool: {}", tool))),
        }
    }
}
