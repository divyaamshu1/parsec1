//! Community/self-hosted extension registry support

use anyhow::Result;
use reqwest::Client;
use std::path::{Path, PathBuf};

use crate::types::{ExtensionInfo, RegistryError};

/// Community registry client
pub struct CommunityRegistry {
    /// Registry base URL
    url: String,
    /// HTTP client
    client: Client,
    /// Registry name (for display)
    name: String,
}

impl CommunityRegistry {
    /// Create a new community registry
    pub fn new(url: String, name: Option<String>) -> Self {
        let name = name.unwrap_or_else(|| {
            url.replace("https://", "")
               .replace("http://", "")
               .split('/')
               .next()
               .unwrap_or(&url)
               .to_string()
        });

        Self {
            url,
            client: Client::new(),
            name,
        }
    }

    /// Get registry name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get registry URL
    pub fn url(&self) -> &str {
        &self.url
    }

    /// List all available extensions from this registry
    pub async fn list_extensions(&self) -> Result<Vec<ExtensionInfo>, RegistryError> {
        let url = format!("{}/api/extensions", self.url);
        
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RegistryError::Network(format!(
                "Failed to list extensions: HTTP {}",
                response.status()
            )));
        }

        let extensions: Vec<ExtensionInfo> = response.json()
            .await
            .map_err(|e| RegistryError::InvalidManifest(e.to_string()))?;

        Ok(extensions)
    }

    /// Search extensions in this registry
    pub async fn search(&self, query: &str) -> Result<Vec<ExtensionInfo>, RegistryError> {
        let url = format!("{}/api/search?q={}", self.url, query);
        
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RegistryError::Network(format!(
                "Search failed: HTTP {}",
                response.status()
            )));
        }

        let extensions: Vec<ExtensionInfo> = response.json()
            .await
            .map_err(|e| RegistryError::InvalidManifest(e.to_string()))?;

        Ok(extensions)
    }

    /// Get extension details
    pub async fn get_extension(&self, id: &str) -> Result<ExtensionInfo, RegistryError> {
        let url = format!("{}/api/extensions/{}", self.url, id);
        
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RegistryError::NotFound(id.to_string()));
        }

        let ext: ExtensionInfo = response.json()
            .await
            .map_err(|e| RegistryError::InvalidManifest(e.to_string()))?;

        Ok(ext)
    }

    /// Download an extension
    pub async fn download(&self, id: &str, version: &str, target_dir: &Path) -> Result<PathBuf, RegistryError> {
        let url = format!("{}/api/extensions/{}/{}/download", self.url, id, version);
        
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RegistryError::Network(format!(
                "Download failed: HTTP {}",
                response.status()
            )));
        }

        // Create filename
        let filename = format!("{}-{}.parc", id.replace(".", "-"), version);
        let filepath = target_dir.join(filename);

        // Download and save
        let bytes = response.bytes()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        tokio::fs::write(&filepath, bytes)
            .await
            .map_err(|e| RegistryError::Io(e))?;

        Ok(filepath)
    }

    /// Check if registry is reachable
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/api/health", self.url);
        self.client.get(&url).send().await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_community_registry_creation() {
        let registry = CommunityRegistry::new(
            "https://extensions.example.com".to_string(),
            Some("Example".to_string()),
        );
        assert_eq!(registry.name(), "Example");
        assert_eq!(registry.url(), "https://extensions.example.com");
    }
}