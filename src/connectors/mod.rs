// Connector Architecture - Unified interface for all data sources
pub mod trait_def;
pub mod manager;
pub mod github;
pub mod gitlab;
pub mod bitbucket;
pub mod local_fs;
pub mod google_drive;
pub mod dropbox;
pub mod notion;
pub mod memory;
pub mod embeddings;
pub mod graph;
pub mod context;
pub mod milvus_search;
pub mod blob_retrieval;

pub use trait_def::Connector;
pub use manager::ConnectorManager;
pub use memory::MemoryConnector;
pub use embeddings::EmbeddingsConnector;
pub use graph::GraphConnector;
pub use context::ContextConnector;
pub use milvus_search::MilvusSearchConnector;
pub use blob_retrieval::BlobRetrievalConnector;
