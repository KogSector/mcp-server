// Context Schema - Token-efficient, normalized data structures
use serde::{Deserialize, Serialize};

/// Repository descriptor - normalized across GitHub, GitLab, Bitbucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryDescriptor {
    pub id: String,              // "gh:owner/repo" | "gl:group/project" | "bb:workspace/repo"
    pub provider: String,        // "github" | "gitlab" | "bitbucket"
    pub name: String,            // "ConHub"
    pub owner: String,           // "KogSector"
    pub visibility: String,      // "public" | "private"
    pub default_branch: String,  // "main"
    pub description: Option<String>,
    pub url: String,
    pub updated_at: i64,         // unix timestamp
}

/// Branch descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchDescriptor {
    pub name: String,
    pub commit_id: String,
    pub is_default: bool,
    pub protected: Option<bool>,
}

/// File/directory descriptor - normalized across all file-based sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDescriptor {
    pub id: String,              // "repo-id:path" | "drive-id:path"
    pub path: String,            // "src/lib.rs" | "/Documents/file.pdf"
    pub name: String,            // "lib.rs"
    pub kind: String,            // "file" | "dir"
    pub size: Option<u64>,       // bytes
    pub language: Option<String>, // "rust" | "typescript" | null
    pub sha: Option<String>,     // git SHA or content hash
    pub last_modified: Option<i64>,
    pub mime_type: Option<String>,
}

/// Document descriptor - for ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentDescriptor {
    pub id: String,              // internal doc id
    pub source: String,          // "github" | "gdrive" | "notion" | "dropbox" | "fs"
    pub source_id: String,       // repo/document/page id
    pub path: Option<String>,    // file path / hierarchical location
    pub title: Option<String>,   // document title
    pub content_type: String,    // "code" | "doc" | "page" | "markdown" | "pdf"
    pub tags: Vec<String>,       // ["auth", "orm", "backend"]
    pub metadata: Option<serde_json::Value>, // provider-specific metadata
    pub created_at: i64,
    pub updated_at: i64,
}

/// Content chunk - for embedding pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentChunk {
    pub id: String,              // "doc-id:chunk-0"
    pub document_id: String,     // references DocumentDescriptor
    pub offset: u32,             // character offset
    pub length: u32,             // character length
    pub content_type: String,    // "code" | "text" | "markdown"
    pub language: Option<String>,
    pub text: String,            // actual content
    pub tags: Vec<String>,
}

/// Resource descriptor for MCP resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDescriptor {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub uri: String,
}

/// Resource content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    pub content: String,
    pub mime_type: Option<String>,
}

/// Paginated result wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: Option<u64>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub has_next: bool,
}

impl<T> PaginatedResult<T> {
    pub fn new(items: Vec<T>, total: Option<u64>, page: Option<u32>, per_page: Option<u32>) -> Self {
        let has_next = if let (Some(t), Some(p), Some(pp)) = (total, page, per_page) {
            ((p * pp) as u64) < t
        } else {
            false
        };
        
        Self {
            items,
            total,
            page,
            per_page,
            has_next,
        }
    }
}
