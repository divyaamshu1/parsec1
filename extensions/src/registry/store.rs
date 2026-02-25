//! Persistent storage for extensions on disk

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use tokio::fs;
use serde::{Serialize, Deserialize};

use crate::Extension;

/// Simple file-based storage for extensions
pub struct ExtensionStore {
    /// Root directory for extensions
    root: PathBuf,
}

impl ExtensionStore {
    /// Create a new extension store
    pub async fn new<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root).await?;
        Ok(Self { root })
    }

    /// Save an extension to disk
    pub async fn save_extension(&self, ext: &Extension) -> Result<()> {
        let ext_dir = self.root.join(&ext.id);
        fs::create_dir_all(&ext_dir).await?;

        // Save manifest
        let manifest_path = ext_dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(&ext.manifest)?;
        fs::write(manifest_path, manifest_json).await?;

        // Save metadata
        let meta_path = ext_dir.join("metadata.json");
        let metadata = ExtensionMetadata::from(ext);
        let meta_json = serde_json::to_string_pretty(&metadata)?;
        fs::write(meta_path, meta_json).await?;

        Ok(())
    }

    /// Load an extension from disk
    pub async fn load_extension(&self, id: &str) -> Result<Extension> {
        let ext_dir = self.root.join(id);
        
        if !ext_dir.exists() {
            return Err(anyhow!("Extension not found: {}", id));
        }

        // Load manifest
        let manifest_path = ext_dir.join("manifest.json");
        let manifest_json = fs::read_to_string(manifest_path).await?;
        let manifest = serde_json::from_str(&manifest_json)?;

        // Load metadata
        let meta_path = ext_dir.join("metadata.json");
        let meta_json = fs::read_to_string(meta_path).await?;
        let metadata: ExtensionMetadata = serde_json::from_str(&meta_json)?;

        Ok(Extension {
            id: id.to_string(),
            manifest,
            path: ext_dir,
            enabled: metadata.enabled,
            installed_at: metadata.installed_at,
            updated_at: metadata.updated_at,
            size: metadata.size,
            permissions: metadata.permissions,
        })
    }

    /// Delete an extension from disk
    pub async fn delete_extension(&self, id: &str) -> Result<()> {
        let ext_dir = self.root.join(id);
        if ext_dir.exists() {
            fs::remove_dir_all(ext_dir).await?;
        }
        Ok(())
    }

    /// Check if an extension exists
    pub async fn has_extension(&self, id: &str) -> bool {
        self.root.join(id).exists()
    }

    /// List all extension IDs
    pub async fn list_extensions(&self) -> Result<Vec<String>> {
        let mut read_dir = fs::read_dir(&self.root).await?;
        let mut ids = Vec::new();

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    ids.push(name.to_string());
                }
            }
        }

        Ok(ids)
    }

    /// Set extension enabled state
    pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        let meta_path = self.root.join(id).join("metadata.json");
        if !meta_path.exists() {
            return Err(anyhow!("Extension not found: {}", id));
        }

        let mut metadata: ExtensionMetadata = serde_json::from_str(&fs::read_to_string(&meta_path).await?)?;
        metadata.enabled = enabled;
        fs::write(meta_path, serde_json::to_string_pretty(&metadata)?).await?;
        Ok(())
    }

    /// Get extension size on disk
    pub async fn get_extension_size(&self, id: &str) -> Result<u64> {
        let ext_dir = self.root.join(id);
        if !ext_dir.exists() {
            return Err(anyhow!("Extension not found: {}", id));
        }

        let mut total = 0;
        let mut read_dir = fs::read_dir(ext_dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            total += entry.metadata().await?.len();
        }
        Ok(total)
    }
}

/// Extension metadata stored on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMetadata {
    pub id: String,
    pub enabled: bool,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub size: u64,
    pub permissions: Vec<String>,
}

impl From<&Extension> for ExtensionMetadata {
    fn from(ext: &Extension) -> Self {
        Self {
            id: ext.id.clone(),
            enabled: ext.enabled,
            installed_at: ext.installed_at,
            updated_at: ext.updated_at,
            size: ext.size,
            permissions: ext.permissions.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;
    use crate::{ExtensionManifest, ExtensionContributes};

    fn create_test_extension(id: &str) -> Extension {
        Extension {
            id: id.to_string(),
            manifest: ExtensionManifest {
                name: id.to_string(),
                version: "1.0.0".to_string(),
                publisher: "test".to_string(),
                description: Some("Test".to_string()),
                entry: "test.wasm".to_string(),
                engines: HashMap::new(),
                categories: vec![],
                tags: vec![],
                repository: None,
                homepage: None,
                license: None,
                icon: None,
                activation_events: vec![],
                contributes: ExtensionContributes::default(),
                capabilities: vec![],
                permissions: vec![],
                dependencies: vec![],
                dev_dependencies: vec![],
            },
            path: PathBuf::new(),
            enabled: true,
            installed_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            size: 1024,
            permissions: vec![],
        }
    }

    #[tokio::test]
    async fn test_save_load() {
        let dir = tempdir().unwrap();
        let store = ExtensionStore::new(dir.path()).await.unwrap();

        let ext = create_test_extension("test.ext");
        store.save_extension(&ext).await.unwrap();

        let loaded = store.load_extension("test.ext").await.unwrap();
        assert_eq!(loaded.id, "test.ext");
        assert_eq!(loaded.manifest.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_delete() {
        let dir = tempdir().unwrap();
        let store = ExtensionStore::new(dir.path()).await.unwrap();

        let ext = create_test_extension("test.ext");
        store.save_extension(&ext).await.unwrap();
        assert!(store.has_extension("test.ext").await);

        store.delete_extension("test.ext").await.unwrap();
        assert!(!store.has_extension("test.ext").await);
    }

    #[tokio::test]
    async fn test_list() {
        let dir = tempdir().unwrap();
        let store = ExtensionStore::new(dir.path()).await.unwrap();

        for i in 0..3 {
            let ext = create_test_extension(&format!("test.{}", i));
            store.save_extension(&ext).await.unwrap();
        }

        let list = store.list_extensions().await.unwrap();
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let dir = tempdir().unwrap();
        let store = ExtensionStore::new(dir.path()).await.unwrap();

        let ext = create_test_extension("test.ext");
        store.save_extension(&ext).await.unwrap();

        store.set_enabled("test.ext", false).await.unwrap();
        let loaded = store.load_extension("test.ext").await.unwrap();
        assert!(!loaded.enabled);
    }
}