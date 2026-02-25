//! Native Android framework support

use std::path::{Path, PathBuf};
use std::collections::HashMap;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tracing::{info, warn, debug};

use super::{MobileFramework, FrameworkType, BuildCommand};

/// Native Android framework
#[derive(Debug, Clone)]
pub struct NativeAndroidFramework {
    sdk_path: PathBuf,
    version: String,
}

#[async_trait]
impl MobileFramework for NativeAndroidFramework {
    fn framework_type(&self) -> FrameworkType {
        FrameworkType::NativeAndroid
    }

    fn name(&self) -> String {
        "Native Android".to_string()
    }

    fn version(&self) -> Option<String> {
        Some(self.version.clone())
    }

    async fn detect() -> Result<Self> {
        let sdk_path = std::env::var("ANDROID_HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("ANDROID_SDK_ROOT").map(PathBuf::from))
            .map_err(|_| anyhow!("Android SDK not found"))?;

        Ok(Self {
            sdk_path,
            version: "latest".to_string(),
        })
    }

    fn is_valid_project(&self, path: &Path) -> bool {
        path.join("app/src/main").exists() && path.join("build.gradle").exists()
    }

    fn build_commands(&self) -> Vec<BuildCommand> {
        vec![
            BuildCommand {
                name: "debug".to_string(),
                command: "./gradlew".to_string(),
                args: vec!["assembleDebug".to_string()],
                env: HashMap::new(),
            },
            BuildCommand {
                name: "release".to_string(),
                command: "./gradlew".to_string(),
                args: vec!["assembleRelease".to_string()],
                env: HashMap::new(),
            },
        ]
    }

    fn run_command(&self, _platform: &str) -> Vec<String> {
        vec![]
    }

    fn box_clone(&self) -> Box<dyn MobileFramework> {
        Box::new(self.clone())
    }
}