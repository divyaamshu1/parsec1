//! Audio asset importers

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use audio::{AudioFile, AudioFormat};

use super::{AssetImporter, AssetType, AssetPreview};

/// Audio importer for sound files
pub struct AudioImporter {
    supported_formats: Vec<String>,
}

impl AudioImporter {
    pub fn new() -> Result<Self> {
        Ok(Self {
            supported_formats: vec![
                "wav".to_string(),
                "mp3".to_string(),
                "ogg".to_string(),
                "flac".to_string(),
                "aiff".to_string(),
            ],
        })
    }

    /// Import audio
    async fn import_audio(&self, source: &Path, destination: &Path) -> Result<()> {
        // Load audio file
        let audio = AudioFile::open(source)?;

        // Convert to engine-specific format
        // For now, just copy
        tokio::fs::copy(source, destination).await?;

        Ok(())
    }

    /// Generate waveform preview
    async fn generate_waveform(&self, path: &Path) -> Result<Vec<f32>> {
        let audio = AudioFile::open(path)?;
        
        // Generate waveform data (simplified)
        let samples = audio.samples()?;
        let waveform: Vec<f32> = samples.iter()
            .step_by(samples.len() / 100)
            .map(|s| *s as f32)
            .collect();

        Ok(waveform)
    }
}

#[async_trait]
impl AssetImporter for AudioImporter {
    fn supported_extensions(&self) -> Vec<String> {
        self.supported_formats.clone()
    }

    fn asset_type(&self) -> AssetType {
        AssetType::Audio
    }

    async fn import(&self, source: &Path, destination: &Path) -> Result<()> {
        self.import_audio(source, destination).await
    }

    async fn preview(&self, path: &Path) -> Result<AssetPreview> {
        let waveform = self.generate_waveform(path).await?;
        Ok(AssetPreview::Audio(waveform))
    }
}