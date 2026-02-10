//! Azure Blob Storage retrieval connector
//!
//! Fetches chunk content from Azure Blob Storage for hybrid retrieval results.

use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub struct BlobRetrievalConnector {
    client: reqwest::Client,
    account_name: String,
    account_key: String,
    container_name: String,
}

impl BlobRetrievalConnector {
    pub fn from_connection_string(connection_string: &str, container_name: &str) -> Result<Self> {
        let mut account_name = String::new();
        let mut account_key = String::new();

        for part in connection_string.split(';') {
            if let Some(val) = part.strip_prefix("AccountName=") {
                account_name = val.to_string();
            } else if let Some(val) = part.strip_prefix("AccountKey=") {
                account_key = val.to_string();
            }
        }

        if account_name.is_empty() || account_key.is_empty() {
            anyhow::bail!("Invalid Azure Blob connection string");
        }

        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()?,
            account_name,
            account_key,
            container_name: container_name.to_string(),
        })
    }

    pub fn from_env() -> Option<Self> {
        let conn_str = std::env::var("AZURE_BLOB_CONNECTION_STRING").ok()?;
        let container = std::env::var("AZURE_BLOB_CONTAINER")
            .unwrap_or_else(|_| "confuse-chunks".to_string());
        Self::from_connection_string(&conn_str, &container).ok()
    }

    /// Download chunk content by blob path
    pub async fn get_chunk_content(&self, blob_path: &str) -> Result<String> {
        let url = format!(
            "https://{}.blob.core.windows.net/{}/{}",
            self.account_name, self.container_name, blob_path
        );
        let version = "2021-08-06";
        let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();

        let string_to_sign = format!(
            "GET\n\n\n\n\n\n\n\n\n\n\n\nx-ms-date:{}\nx-ms-version:{}\n/{}/{}/{}",
            date, version, self.account_name, self.container_name, blob_path,
        );

        let key_bytes = BASE64.decode(&self.account_key)?;
        let mut mac = Hmac::<Sha256>::new_from_slice(&key_bytes)?;
        mac.update(string_to_sign.as_bytes());
        let signature = BASE64.encode(mac.finalize().into_bytes());
        let auth = format!("SharedKey {}:{}", self.account_name, signature);

        let response = self.client
            .get(&url)
            .header("Authorization", &auth)
            .header("x-ms-date", &date)
            .header("x-ms-version", version)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Blob download failed ({}): {}", response.status(), blob_path);
        }

        Ok(response.text().await?)
    }

    /// Batch download multiple chunks
    pub async fn get_chunks_content(&self, blob_paths: &[String]) -> Vec<(String, Result<String>)> {
        let mut results = Vec::with_capacity(blob_paths.len());
        for path in blob_paths {
            let content = self.get_chunk_content(path).await;
            results.push((path.clone(), content));
        }
        results
    }

    pub async fn health_check(&self) -> Result<()> {
        let url = format!(
            "https://{}.blob.core.windows.net/{}?restype=container",
            self.account_name, self.container_name
        );
        let response = self.client.head(&url).send().await?;
        if response.status().is_server_error() {
            anyhow::bail!("Blob storage unreachable");
        }
        Ok(())
    }
}
