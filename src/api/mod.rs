//! REST API endpoints for knowledge search
//!
//! This module provides HTTP endpoints for semantic and hybrid search
//! using FalcorDB vector storage.

pub mod search;

pub use search::{search_routes, SearchRequest, SearchResponse};
