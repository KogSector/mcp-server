// Security Client - Interface to security microservice or DB
use anyhow::Result;
use conhub_database::Database;
use uuid::Uuid;

pub struct SecurityClient {
    db: Database,
}

impl SecurityClient {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
    
    /// Get encrypted secret (API token) for a user and provider
    pub async fn get_user_token(&self, user_id: &Uuid, provider: &str, key_name: &str) -> Result<Option<String>> {
        // Use security repository to fetch encrypted secret
        let security_repo = conhub_database::repositories::SecurityRepository::new(self.db.pool().clone());
        
        if let Some(secret) = security_repo.get_encrypted_secret(user_id, key_name).await? {
            // In production, decrypt the value here
            // For now, assume it's stored in a retrievable format
            // TODO: Implement actual decryption
            Ok(Some(String::from_utf8_lossy(&secret.encrypted_value).to_string()))
        } else {
            Ok(None)
        }
    }
    
    /// Check rate limit for a user/endpoint
    pub async fn check_rate_limit(&self, identifier: &str, endpoint: &str) -> Result<bool> {
        let security_repo = conhub_database::repositories::SecurityRepository::new(self.db.pool().clone());
        security_repo.check_rate_limit(identifier, endpoint, 60, 60).await
    }
    
    /// Log security event
    pub async fn log_event(&self, user_id: &Uuid, event_type: &str, severity: &str, details: serde_json::Value) -> Result<()> {
        let security_repo = conhub_database::repositories::SecurityRepository::new(self.db.pool().clone());
        
        let input = conhub_database::models::CreateSecurityEventInput {
            user_id: Some(*user_id),
            event_type: event_type.to_string(),
            severity: severity.to_string(),
            ip_address: None,
            user_agent: None,
            details: Some(details),
        };
        
        security_repo.log_security_event(&input).await?;
        Ok(())
    }
}
