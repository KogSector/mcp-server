// Local Filesystem Connector
use super::Connector;
use crate::{context::*, errors::{McpError, McpResult}, protocol::McpTool};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub struct LocalFsConnector {
    root_paths: Vec<PathBuf>,
    ignore_patterns: Vec<String>,
}

impl LocalFsConnector {
    pub fn new(root_paths: Vec<String>, ignore_patterns: Vec<String>) -> Self {
        Self {
            root_paths: root_paths.into_iter().map(PathBuf::from).collect(),
            ignore_patterns,
        }
    }
    
    fn is_safe_path(&self, path: &Path) -> bool {
        // Ensure path is within one of the allowed roots
        self.root_paths.iter().any(|root| path.starts_with(root))
    }
}

#[async_trait]
impl Connector for LocalFsConnector {
    fn id(&self) -> &'static str {
        "fs"
    }
    
    fn list_tools(&self) -> Vec<McpTool> {
        vec![
            McpTool {
                name: "fs.list_files".to_string(),
                description: "List files in local filesystem".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"]
                })),
            },
            McpTool {
                name: "fs.read_file".to_string(),
                description: "Read file content from local filesystem".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"]
                })),
            },
        ]
    }
    
    async fn call_tool(&self, _tool: &str, _args: Value) -> McpResult<Value> {
        // Stub - to be implemented with proper path validation
        Err(McpError::Internal("Local FS connector not yet fully implemented".to_string()))
    }
}
