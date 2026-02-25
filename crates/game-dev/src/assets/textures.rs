//! Texture asset importers

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use image::{ImageFormat, DynamicImage};

use super::{AssetImporter, AssetType, AssetPreview};

/// Texture importer for images
pub struct TextureImporter {
    supported_formats: Vec<String>,
}

impl TextureImporter {
    pub fn new() -> Result<Self> {
        Ok(Self {
            supported_formats: vec![
                "png".to_string(),
                "jpg".to_string(),
                "jpeg".to_string(),
                "tga".to_string(),
                "bmp".to_string(),
                "psd".to_string(),
                "exr".to_string(),
                "hdr".to_string(),
            ],
        })
    }

    /// Import texture
    async fn import_texture(&self, source: &Path, destination: &Path) -> Result<()> {
        // Load image
        let img = image::open(source)?;

        // Convert to engine-specific format
        // For now, just copy
        tokio::fs::copy(source, destination).await?;

        Ok(())
    }

    /// Generate thumbnail
    async fn generate_thumbnail(&self, path: &Path) -> Result<Vec<u8>> {
        let img = image::open(path)?;
        
        // Resize to thumbnail
        let thumbnail = img.thumbnail(128, 128);
        
        let mut bytes: Vec<u8> = Vec::new();
        thumbnail.write_to(&mut bytes, ImageFormat::Png)?;

        Ok(bytes)
    }
}

#[async_trait]
impl AssetImporter for TextureImporter {
    fn supported_extensions(&self) -> Vec<String> {
        self.supported_formats.clone()
    }

    fn asset_type(&self) -> AssetType {
        AssetType::Texture
    }

    async fn import(&self, source: &Path, destination: &Path) -> Result<()> {
        self.import_texture(source, destination).await
    }

    async fn preview(&self, path: &Path) -> Result<AssetPreview> {
        let thumbnail = self.generate_thumbnail(path).await?;
        Ok(AssetPreview::Image(thumbnail))
    }
}