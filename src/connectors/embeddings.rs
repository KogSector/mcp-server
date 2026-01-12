// Embeddings Connector - Direct access to embeddings service
use crate::{context::*, protocol::McpTool, errors::{McpError, McpResult}};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use super::Connector;

pub struct EmbeddingsConnector {
    base_url: String,
    client: reqwest::Client,
}

impl EmbeddingsConnector {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct EmbedRequest {
    text: String,
}

#[derive(Debug, Serialize)]
struct BatchEmbedRequest {
    texts: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
    dimension: u32,
    model: String,
}

#[derive(Debug, Deserialize)]
struct BatchEmbedResponse {
    embeddings: Vec<Vec<f32>>,
    dimension: u32,
    model: String,
}

#[async_trait]
impl Connector for EmbeddingsConnector {
    fn id(&self) -> &'static str {
        "embeddings"
    }
    
    fn list_tools(&self) -> Vec<McpTool> {
        vec![
            McpTool {
                name: "embeddings.embed".to_string(),
                description: "Generate embedding vector for a single text".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "Text to embed"
                        }
                    },
                    "required": ["text"]
                }),
            },
            McpTool {
                name: "embeddings.batch_embed".to_string(),
                description: "Generate embedding vectors for multiple texts".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "texts": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Texts to embed"
                        }
                    },
                    "required": ["texts"]
                }),
            },
            McpTool {
                name: "embeddings.similarity".to_string(),
                description: "Calculate cosine similarity between two texts".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "text1": { "type": "string" },
                        "text2": { "type": "string" }
                    },
                    "required": ["text1", "text2"]
                }),
            },
        ]
    }
    
    async fn call_tool(&self, tool: &str, args: Value) -> McpResult<Value> {
        match tool {
            "embed" => {
                let text = args.get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments("Missing 'text' argument".into()))?;
                
                let response = self.client
                    .post(format!("{}/embed", self.base_url))
                    .json(&EmbedRequest { text: text.to_string() })
                    .send()
                    .await
                    .map_err(|e| McpError::Internal(format!("Embeddings request failed: {}", e)))?;
                
                if !response.status().is_success() {
                    return Err(McpError::Internal(format!(
                        "Embeddings service returned {}", response.status()
                    )));
                }
                
                let result: EmbedResponse = response.json().await
                    .map_err(|e| McpError::Internal(format!("Failed to parse response: {}", e)))?;
                
                Ok(json!({
                    "embedding": result.embedding,
                    "dimension": result.dimension,
                    "model": result.model
                }))
            }
            
            "batch_embed" => {
                let texts: Vec<String> = args.get("texts")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .ok_or_else(|| McpError::InvalidArguments("Missing 'texts' argument".into()))?;
                
                let response = self.client
                    .post(format!("{}/batch/embed", self.base_url))
                    .json(&BatchEmbedRequest { texts })
                    .send()
                    .await
                    .map_err(|e| McpError::Internal(format!("Embeddings request failed: {}", e)))?;
                
                if !response.status().is_success() {
                    return Err(McpError::Internal(format!(
                        "Embeddings service returned {}", response.status()
                    )));
                }
                
                let result: BatchEmbedResponse = response.json().await
                    .map_err(|e| McpError::Internal(format!("Failed to parse response: {}", e)))?;
                
                Ok(json!({
                    "embeddings": result.embeddings,
                    "dimension": result.dimension,
                    "model": result.model,
                    "count": result.embeddings.len()
                }))
            }
            
            "similarity" => {
                let text1 = args.get("text1")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments("Missing 'text1' argument".into()))?;
                let text2 = args.get("text2")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments("Missing 'text2' argument".into()))?;
                
                // Get embeddings for both texts
                let response = self.client
                    .post(format!("{}/batch/embed", self.base_url))
                    .json(&BatchEmbedRequest { 
                        texts: vec![text1.to_string(), text2.to_string()] 
                    })
                    .send()
                    .await
                    .map_err(|e| McpError::Internal(format!("Embeddings request failed: {}", e)))?;
                
                if !response.status().is_success() {
                    return Err(McpError::Internal(format!(
                        "Embeddings service returned {}", response.status()
                    )));
                }
                
                let result: BatchEmbedResponse = response.json().await
                    .map_err(|e| McpError::Internal(format!("Failed to parse response: {}", e)))?;
                
                if result.embeddings.len() != 2 {
                    return Err(McpError::Internal("Expected 2 embeddings".into()));
                }
                
                // Calculate cosine similarity
                let similarity = cosine_similarity(&result.embeddings[0], &result.embeddings[1]);
                
                Ok(json!({
                    "similarity": similarity,
                    "text1": text1,
                    "text2": text2
                }))
            }
            
            _ => Err(McpError::ToolNotFound(format!("Unknown tool: embeddings.{}", tool))),
        }
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }
    
    dot_product / (magnitude_a * magnitude_b)
}
