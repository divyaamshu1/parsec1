//! Asset pipeline for game development

mod models;
mod textures;
mod audio;
mod preview;

pub use models::*;
pub use textures::*;
pub use audio::*;
pub use preview::*;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tracing::{info, warn, debug};

/// Asset type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
    Model,
    Texture,
    Audio,
    Material,
    Shader,
    Animation,
    Prefab,
    Scene,
    Unknown,
}

/// Asset importer trait
#[async_trait]
pub trait AssetImporter: Send + Sync {
    fn supported_extensions(&self) -> Vec<String>;
    fn asset_type(&self) -> AssetType;
    async fn import(&self, source: &Path, destination: &Path) -> Result<()>;
    async fn preview(&self, path: &Path) -> Result<AssetPreview>;
}

/// Asset manager
pub struct AssetManager {
    assets_dir: PathBuf,
    importers: HashMap<String, Box<dyn AssetImporter>>,
    preview_cache: Arc<tokio::sync::Mutex<HashMap<PathBuf, AssetPreview>>>,
}

/// Asset preview
#[derive(Debug, Clone)]
pub enum AssetPreview {
    Image(Vec<u8>),
    Model(Vec<f32>),
    Audio(Vec<f32>),
    Text(String),
    Json(serde_json::Value),
}

impl AssetManager {
    /// Create new asset manager
    pub fn new(assets_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&assets_dir)?;

        let mut manager = Self {
            assets_dir,
            importers: HashMap::new(),
            preview_cache: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        };

        // Register built-in importers
        manager.register_importer(Box::new(models::ModelImporter::new()?));
        manager.register_importer(Box::new(textures::TextureImporter::new()?));
        manager.register_importer(Box::new(audio::AudioImporter::new()?));

        Ok(manager)
    }

    /// Register an asset importer
    pub fn register_importer(&mut self, importer: Box<dyn AssetImporter>) {
        for ext in importer.supported_extensions() {
            self.importers.insert(ext, importer.clone_box());
        }
    }

    /// Import assets into project
    pub async fn import_assets(
        &self,
        project: &crate::Project,
        engine: &dyn crate::GameEngine,
        paths: Vec<PathBuf>,
    ) -> Result<Vec<crate::AssetImportResult>> {
        let mut results = Vec::new();

        for source in paths {
            let ext = source.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string();

            if let Some(importer) = self.importers.get(&ext) {
                let asset_type = importer.asset_type();
                let dest = self.assets_dir.join(project.name()).join(source.file_name().unwrap());

                match importer.import(&source, &dest).await {
                    Ok(()) => {
                        results.push(crate::AssetImportResult {
                            source,
                            destination: dest,
                            asset_type,
                            success: true,
                            error: None,
                        });
                    }
                    Err(e) => {
                        results.push(crate::AssetImportResult {
                            source,
                            destination: dest,
                            asset_type,
                            success: false,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    /// Get asset preview
    pub async fn get_preview(&self, path: &Path) -> Result<Option<AssetPreview>> {
        // Check cache
        {
            let cache = self.preview_cache.lock().await;
            if let Some(preview) = cache.get(path) {
                return Ok(Some(preview.clone()));
            }
        }

        // Generate preview
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        if let Some(importer) = self.importers.get(&ext) {
            let preview = importer.preview(path).await?;
            
            // Cache preview
            let mut cache = self.preview_cache.lock().await;
            cache.insert(path.to_path_buf(), preview.clone());

            Ok(Some(preview))
        } else {
            Ok(None)
        }
    }

    /// Clear preview cache
    pub async fn clear_cache(&self) {
        let mut cache = self.preview_cache.lock().await;
        cache.clear();
    }
}

/// Helper for cloning boxed importers
impl dyn AssetImporter {
    fn clone_box(&self) -> Box<dyn AssetImporter> {
        // This would need to be implemented by each concrete importer
        unimplemented!("Clone not implemented for this importer")
    }
}