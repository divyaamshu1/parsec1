//! Model asset importers

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use gltf::Gltf;

use super::{AssetImporter, AssetType, AssetPreview};

/// Model importer for 3D models
pub struct ModelImporter {
    supported_formats: Vec<String>,
}

impl ModelImporter {
    pub fn new() -> Result<Self> {
        Ok(Self {
            supported_formats: vec![
                "gltf".to_string(),
                "glb".to_string(),
                "fbx".to_string(),
                "obj".to_string(),
                "dae".to_string(),
            ],
        })
    }

    /// Import glTF/GLB model
    async fn import_gltf(&self, source: &Path, destination: &Path) -> Result<()> {
        let gltf = Gltf::open(source)?;
        
        // Process glTF data
        for mesh in gltf.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&gltf.buffers()[buffer.index()]));
                
                if let Some(positions) = reader.read_positions() {
                    // Process vertex positions
                    let vertices: Vec<f32> = positions.flatten().copied().collect();
                    debug!("Loaded {} vertices from {}", vertices.len(), source.display());
                }
            }
        }

        // Copy to destination
        tokio::fs::copy(source, destination).await?;

        Ok(())
    }

    /// Import OBJ model
    async fn import_obj(&self, source: &Path, destination: &Path) -> Result<()> {
        let (models, _materials) = obj::Obj::load(source)?;
        
        for model in models {
            debug!("Loaded {} vertices from {}", model.vertices().len(), source.display());
        }

        tokio::fs::copy(source, destination).await?;
        Ok(())
    }

    /// Generate preview image for model
    async fn generate_preview(&self, path: &Path) -> Result<AssetPreview> {
        // In production, this would render a 3D preview
        // For now, return placeholder
        Ok(AssetPreview::Text(format!("Model preview: {}", path.display())))
    }
}

#[async_trait]
impl AssetImporter for ModelImporter {
    fn supported_extensions(&self) -> Vec<String> {
        self.supported_formats.clone()
    }

    fn asset_type(&self) -> AssetType {
        AssetType::Model
    }

    async fn import(&self, source: &Path, destination: &Path) -> Result<()> {
        let ext = source.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match ext {
            "gltf" | "glb" => self.import_gltf(source, destination).await,
            "obj" => self.import_obj(source, destination).await,
            "fbx" | "dae" => {
                // Would need FBX SDK or similar
                tokio::fs::copy(source, destination).await?;
                Ok(())
            }
            _ => Err(anyhow!("Unsupported model format: {}", ext)),
        }
    }

    async fn preview(&self, path: &Path) -> Result<AssetPreview> {
        self.generate_preview(path).await
    }
}