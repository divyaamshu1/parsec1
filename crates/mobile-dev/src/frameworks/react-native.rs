//! React Native framework support

use std::path::{Path, PathBuf};
use std::collections::HashMap;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::process::Command;
use tracing::{info, warn, debug};

use super::{MobileFramework, FrameworkType, BuildCommand};

/// React Native framework
#[derive(Debug, Clone)]
pub struct ReactNativeFramework {
    node_path: PathBuf,
    npm_path: PathBuf,
    version: String,
}

#[async_trait]
impl MobileFramework for ReactNativeFramework {
    fn framework_type(&self) -> FrameworkType {
        FrameworkType::ReactNative
    }

    fn name(&self) -> String {
        format!("React Native {}", self.version)
    }

    fn version(&self) -> Option<String> {
        Some(self.version.clone())
    }

    async fn detect() -> Result<Self> {
        // Find node and npm
        let node_path = which::which("node")
            .map_err(|_| anyhow!("Node.js not found"))?;

        let npm_path = which::which("npm")
            .map_err(|_| anyhow!("npm not found"))?;

        // Get version from package.json if available
        let version = "latest".to_string();

        Ok(Self {
            node_path,
            npm_path,
            version,
        })
    }

    fn is_valid_project(&self, path: &Path) -> bool {
        path.join("package.json").exists() && path.join("index.js").exists()
    }

    fn build_commands(&self) -> Vec<BuildCommand> {
        vec![
            BuildCommand {
                name: "android".to_string(),
                command: "cd android && ./gradlew".to_string(),
                args: vec!["assembleRelease".to_string()],
                env: HashMap::new(),
            },
            BuildCommand {
                name: "ios".to_string(),
                command: "xcodebuild".to_string(),
                args: vec!["-workspace".to_string(), "ios/MyApp.xcworkspace".to_string(), "-scheme".to_string(), "MyApp".to_string(), "-configuration".to_string(), "Release".to_string()],
                env: HashMap::new(),
            },
        ]
    }

    fn run_command(&self, platform: &str) -> Vec<String> {
        vec![
            "npx".to_string(),
            "react-native".to_string(),
            "run-".to_string() + platform,
        ]
    }

    fn box_clone(&self) -> Box<dyn MobileFramework> {
        Box::new(self.clone())
    }
}