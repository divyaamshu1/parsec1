//! SDK for creating custom mobile framework extensions

mod platform_trait;
mod build_hooks;
mod debug_hooks;

pub use platform_trait::*;
pub use build_hooks::*;
pub use debug_hooks::*;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::frameworks::FrameworkType;
use crate::platform::Platform;

/// Mobile SDK
pub struct MobileSDK {
    custom_frameworks: Arc<RwLock<HashMap<String, Box<dyn FrameworkExtension>>>>,
    custom_detectors: Arc<RwLock<Vec<Box<dyn CustomFrameworkDetector>>>>,
}

/// Framework extension trait
#[async_trait]
pub trait FrameworkExtension: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn framework_type(&self) -> String;
    async fn detect_installation(&self) -> Option<InstallationInfo>;
    async fn create_framework(&self) -> Box<dyn crate::frameworks::MobileFramework>;
    async fn create_platform(&self) -> Box<dyn Platform>;
}

/// Installation information
#[derive(Debug, Clone)]
pub struct InstallationInfo {
    pub path: PathBuf,
    pub version: String,
    pub capabilities: Vec<String>,
}

/// Custom framework detector trait
#[async_trait]
pub trait CustomFrameworkDetector: Send + Sync {
    fn name(&self) -> &str;
    async fn detect(&self) -> Option<Box<dyn crate::frameworks::MobileFramework>>;
}

impl MobileSDK {
    /// Create new SDK
    pub fn new() -> Result<Self> {
        Ok(Self {
            custom_frameworks: Arc::new(RwLock::new(HashMap::new())),
            custom_detectors: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Register a framework extension
    pub async fn register_extension(&self, extension: Box<dyn FrameworkExtension>) -> Result<()> {
        let name = extension.name().to_string();
        self.custom_frameworks.write().await.insert(name, extension);
        Ok(())
    }

    /// Register a custom framework detector
    pub async fn register_detector(&self, detector: Box<dyn CustomFrameworkDetector>) {
        self.custom_detectors.write().await.push(detector);
    }

    /// Detect custom platforms
    pub async fn detect_custom_platforms(&self) -> Result<HashMap<String, Box<dyn Platform>>> {
        let platforms = HashMap::new();
        let detectors = self.custom_detectors.read().await;

        for detector in detectors.iter() {
            if let Some(framework) = detector.detect().await {
                // Would need to get platform from framework
                // platforms.insert(detector.name().to_string(), platform);
            }
        }

        Ok(platforms)
    }

    /// Detect custom framework from path
    pub async fn detect_custom_framework(&self, path: &Path) -> Result<Option<FrameworkType>> {
        let detectors = self.custom_detectors.read().await;

        for detector in detectors.iter() {
            if let Some(framework) = detector.detect().await {
                return Ok(Some(framework.framework_type()));
            }
        }

        Ok(None)
    }
}