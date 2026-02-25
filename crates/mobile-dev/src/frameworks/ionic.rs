//! Ionic framework support

use std::path::{Path, PathBuf};
use std::collections::HashMap;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::process::Command;
use tracing::{info, warn, debug};

use super::{MobileFramework, FrameworkType, BuildCommand};

/// Ionic framework
#[derive(Debug, Clone)]
pub struct IonicFramework {
    ionic_path: PathBuf,
    version: String,
}

#[async_trait]
impl MobileFramework for IonicFramework {
    fn framework_type(&self) -> FrameworkType {
        FrameworkType::Ionic
    }

    fn name(&self) -> String {
        format!("Ionic {}", self.version)
    }

    fn version(&self) -> Option<String> {
        Some(self.version.clone())
    }

    async fn detect() -> Result<Self> {
        let ionic_path = which::which("ionic")
            .map_err(|_| anyhow!("Ionic CLI not found"))?;

        let output = Command::new(&ionic_path)
            .arg("--version")
            .output()
            .await?;

        let version = String::from_utf8(output.stdout)?.trim().to_string();

        Ok(Self {
            ionic_path,
            version,
        })
    }

    fn is_valid_project(&self, path: &Path) -> bool {
        path.join("ionic.config.json").exists()
    }

    fn build_commands(&self) -> Vec<BuildCommand> {
        vec![
            BuildCommand {
                name: "android".to_string(),
                command: self.ionic_path.to_string_lossy().to_string(),
                args: vec!["build".to_string(), "android".to_string()],
                env: HashMap::new(),
            },
            BuildCommand {
                name: "ios".to_string(),
                command: self.ionic_path.to_string_lossy().to_string(),
                args: vec!["build".to_string(), "ios".to_string()],
                env: HashMap::new(),
            },
        ]
    }

    fn run_command(&self, platform: &str) -> Vec<String> {
        vec![
            self.ionic_path.to_string_lossy().to_string(),
            "serve".to_string(),
        ]
    }

    fn box_clone(&self) -> Box<dyn MobileFramework> {
        Box::new(self.clone())
    }
}