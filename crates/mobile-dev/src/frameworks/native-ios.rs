//! Native iOS framework support

use std::path::{Path, PathBuf};
use std::collections::HashMap;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tracing::{info, warn, debug};

use super::{MobileFramework, FrameworkType, BuildCommand};

/// Native iOS framework
#[derive(Debug, Clone)]
pub struct NativeIOSFramework {
    xcode_path: PathBuf,
    version: String,
}

#[async_trait]
impl MobileFramework for NativeIOSFramework {
    fn framework_type(&self) -> FrameworkType {
        FrameworkType::NativeIOS
    }

    fn name(&self) -> String {
        "Native iOS".to_string()
    }

    fn version(&self) -> Option<String> {
        Some(self.version.clone())
    }

    async fn detect() -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            let xcode_path = PathBuf::from("/Applications/Xcode.app");
            if !xcode_path.exists() {
                return Err(anyhow!("Xcode not found"));
            }

            Ok(Self {
                xcode_path,
                version: "latest".to_string(),
            })
        }

        #[cfg(not(target_os = "macos"))]
        {
            Err(anyhow!("iOS development requires macOS"))
        }
    }

    fn is_valid_project(&self, path: &Path) -> bool {
        path.join("*.xcodeproj").exists() || path.join("*.xcworkspace").exists()
    }

    fn build_commands(&self) -> Vec<BuildCommand> {
        vec![
            BuildCommand {
                name: "debug".to_string(),
                command: "xcodebuild".to_string(),
                args: vec!["-configuration".to_string(), "Debug".to_string()],
                env: HashMap::new(),
            },
            BuildCommand {
                name: "release".to_string(),
                command: "xcodebuild".to_string(),
                args: vec!["-configuration".to_string(), "Release".to_string()],
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