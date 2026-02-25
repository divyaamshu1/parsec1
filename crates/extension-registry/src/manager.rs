use anyhow::Result;
use std::path::PathBuf;
use crate::sources::{vsx::VSCodeMarketplace, local::LocalRegistry};
use crate::types::{ExtensionInfo, InstalledExtension};

pub struct RegistryManager {
    pub vsx: VSCodeMarketplace,
    pub local: LocalRegistry,
}

impl RegistryManager {
    pub fn new(extensions_dir: PathBuf, marketplace_url: String) -> Self {
        Self {
            vsx: VSCodeMarketplace::new(marketplace_url, None),
            local: LocalRegistry::new(extensions_dir),
        }
    }
    
    pub async fn search_vsx(&self, query: &str) -> Result<Vec<ExtensionInfo>> {
        let results = self.vsx.search(crate::SearchQuery {
            text: query.to_string(),
            page: 1,
            page_size: 50,
            ..Default::default()
        }).await?;
        
        Ok(results.extensions)
    }
    
    pub async fn list_installed(&self) -> Result<Vec<InstalledExtension>> {
        self.local.list_installed().await
    }
    
    pub async fn install_local(&self, path: &PathBuf) -> Result<String> {
        self.local.install(path).await
    }
}