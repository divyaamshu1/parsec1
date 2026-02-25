//! Core engine trait for custom engines

use std::path::Path;

use async_trait::async_trait;

use crate::engine::GameEngine;

/// Custom engine detector trait
#[async_trait]
pub trait CustomEngineDetector: Send + Sync {
    fn name(&self) -> &str;
    async fn detect(&self) -> Option<InstallationInfo>;
}

/// Installation information
#[derive(Debug, Clone)]
pub struct InstallationInfo {
    pub path: PathBuf,
    pub version: String,
    pub capabilities: Vec<String>,
}