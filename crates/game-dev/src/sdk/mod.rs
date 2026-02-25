//! SDK for creating custom game engine extensions

mod engine_trait;
mod build_hooks;
mod asset_importers;

pub use engine_trait::*;
pub use build_hooks::*;
pub use asset_importers::*;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::sync::RwLock;

/// Game Engine SDK
pub struct GameEngineSDK {
    custom_engines: Arc<RwLock<HashMap<String, Box<dyn EngineExtension>>>>,
    custom_detectors: Arc<RwLock<Vec<Box<dyn CustomEngineDetector>>>>,
}

/// Engine extension trait
#[async_trait]
pub trait EngineExtension: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn engine_type(&self) -> String;
    async fn detect_installation(&self) -> Option<InstallationInfo>;
    async fn create_engine(&self) -> Box<dyn crate::GameEngine>;
}

/// Installation information
#[derive(Debug, Clone)]
pub struct InstallationInfo {
    pub path: PathBuf,
    pub version: String,
    pub capabilities: Vec<String>,
}

impl GameEngineSDK {
    /// Create new SDK
    pub fn new() -> Result<Self> {
        Ok(Self {
            custom_engines: Arc::new(RwLock::new(HashMap::new())),
            custom_detectors: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Register an engine extension
    pub async fn register_extension(&self, extension: Box<dyn EngineExtension>) -> Result<()> {
        let name = extension.name().to_string();
        self.custom_engines.write().await.insert(name, extension);
        Ok(())
    }

    /// Register a custom engine detector
    pub async fn register_detector(&self, detector: Box<dyn CustomEngineDetector>) {
        self.custom_detectors.write().await.push(detector);
    }

    /// Detect custom engines
    pub async fn detect_custom_engines(&self) -> Result<HashMap<String, Box<dyn crate::GameEngine>>> {
        let mut engines = HashMap::new();
        let detectors = self.custom_detectors.read().await;

        for detector in detectors.iter() {
            if let Some(installation) = detector.detect().await {
                if let Some(extension) = self.custom_engines.read().await.get(detector.name()) {
                    let engine = extension.create_engine().await;
                    engines.insert(detector.name().to_string(), engine);
                }
            }
        }

        Ok(engines)
    }

    /// Detect custom project
    pub async fn detect_custom_project(&self, path: &Path) -> Result<Option<crate::ProjectType>> {
        let extensions = self.custom_engines.read().await;

        for (name, extension) in extensions.iter() {
            // Check if this project belongs to the custom engine
            // This would need engine-specific detection logic
            if path.join(format!("{}.project", name)).exists() {
                return Ok(Some(crate::ProjectType::Custom));
            }
        }

        Ok(None)
    }
}