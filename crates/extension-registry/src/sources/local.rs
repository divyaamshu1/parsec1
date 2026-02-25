use anyhow::Result;
use std::path::{Path, PathBuf};
use crate::types::{InstalledExtension, ExtensionManifest};

pub struct LocalRegistry {
    extensions_dir: PathBuf,
}

impl LocalRegistry {
    pub fn new(extensions_dir: PathBuf) -> Self {
        Self { extensions_dir }
    }
    
    pub async fn list_installed(&self) -> Result<Vec<InstalledExtension>> {
        let mut extensions = Vec::new();
        
        if !self.extensions_dir.exists() {
            return Ok(extensions);
        }
        
        let mut read_dir = tokio::fs::read_dir(&self.extensions_dir).await?;
        
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join("manifest.json");
                if manifest_path.exists() {
                    let content = tokio::fs::read_to_string(manifest_path).await?;
                    let manifest: ExtensionManifest = serde_json::from_str(&content)?;
                    
                    extensions.push(InstalledExtension {
                        id: manifest.id.clone(),
                        path,
                        manifest,
                        enabled: true,
                    });
                }
            }
        }
        
        Ok(extensions)
    }
    
    pub async fn install(&self, path: &Path) -> Result<String> {
        let content = tokio::fs::read_to_string(path.join("manifest.json")).await?;
        let manifest: ExtensionManifest = serde_json::from_str(&content)?;
        
        let dest = self.extensions_dir.join(&manifest.id);
        tokio::fs::create_dir_all(&dest).await?;
        
        // Copy all files
        let mut entries = tokio::fs::read_dir(path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name();
            let dest_path = dest.join(name);
            tokio::fs::copy(entry.path(), dest_path).await?;
        }
        
        Ok(manifest.id)
    }
    
    pub async fn uninstall(&self, id: &str) -> Result<()> {
        let path = self.extensions_dir.join(id);
        if path.exists() {
            tokio::fs::remove_dir_all(path).await?;
        }
        Ok(())
    }
}