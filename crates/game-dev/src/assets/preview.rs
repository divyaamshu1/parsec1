//! Asset preview generation

use std::path::Path;
use anyhow::Result;

use super::AssetPreview;

/// Asset preview generator
pub struct PreviewGenerator;

impl PreviewGenerator {
    /// Generate preview for any asset
    pub async fn generate_preview(&self, path: &Path) -> Result<Option<AssetPreview>> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "tga" => self.image_preview(path).await,
            "gltf" | "glb" | "obj" => self.model_preview(path).await,
            "wav" | "mp3" | "ogg" => self.audio_preview(path).await,
            "txt" | "json" | "xml" | "yaml" => self.text_preview(path).await,
            _ => Ok(None),
        }
    }

    async fn image_preview(&self, path: &Path) -> Result<Option<AssetPreview>> {
        let img = image::open(path)?;
        let thumbnail = img.thumbnail(256, 256);
        
        let mut bytes = Vec::new();
        thumbnail.write_to(&mut bytes, image::ImageFormat::Png)?;

        Ok(Some(AssetPreview::Image(bytes)))
    }

    async fn model_preview(&self, path: &Path) -> Result<Option<AssetPreview>> {
        // In production, would generate 3D preview
        Ok(Some(AssetPreview::Text(format!("3D Model: {}", path.display()))))
    }

    async fn audio_preview(&self, path: &Path) -> Result<Option<AssetPreview>> {
        // Generate waveform
        Ok(Some(AssetPreview::Text(format!("Audio: {}", path.display()))))
    }

    async fn text_preview(&self, path: &Path) -> Result<Option<AssetPreview>> {
        let content = tokio::fs::read_to_string(path).await?;
        let preview = if content.len() > 1000 {
            format!("{}...", &content[..1000])
        } else {
            content
        };
        Ok(Some(AssetPreview::Text(preview)))
    }
}