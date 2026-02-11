// Search and Retrieval Module
pub mod blob;
pub mod chromadb;
pub mod embeddings;
pub mod graph;
pub mod hybrid;
pub mod memory;
pub mod schema;
pub mod service_trait;
pub mod manager;

pub use service_trait::SearchService;
pub use manager::SearchManager;
pub use schema::*;

// Re-export all search services
pub use blob::BlobRetrievalService;
pub use chromadb::ChromaSearchService;
pub use embeddings::EmbeddingsService;
pub use graph::GraphSearchService;
pub use hybrid::HybridSearchService;
pub use memory::MemoryService;
