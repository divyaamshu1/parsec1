//! Flutter framework support

use std::path::{Path, PathBuf};
use std::collections::HashMap;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::process::Command;
use tracing::{info, warn, debug};

use super::{MobileFramework, FrameworkType, BuildCommand};

/// Flutter framework
#[derive(Debug, Clone)]
pub struct FlutterFramework {
    flutter_path: PathBuf,
    version: String,
}

#[async_trait]
impl MobileFramework for FlutterFramework {
    fn framework_type(&self) -> FrameworkType {
        FrameworkType::Flutter
    }

    fn name(&self) -> String {
        format!("Flutter {}", self.version)
    }

    fn version(&self) -> Option<String> {
        Some(self.version.clone())
    }

    async fn detect() -> Result<Self> {
        // Find flutter in PATH
        let flutter_path = which::which("flutter")
            .map_err(|_| anyhow!("Flutter not found in PATH"))?;

        // Get version
        let output = Command::new(&flutter_path)
            .arg("--version")
            .output()
            .await?;

        let version = String::from_utf8(output.stdout)?
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .unwrap_or("unknown")
            .to_string();

        Ok(Self {
            flutter_path,
            version,
        })
    }

    fn is_valid_project(&self, path: &Path) -> bool {
        path.join("pubspec.yaml").exists()
    }

    fn build_commands(&self) -> Vec<BuildCommand> {
        vec![
            BuildCommand {
                name: "apk".to_string(),
                command: self.flutter_path.to_string_lossy().to_string(),
                args: vec!["build".to_string(), "apk".to_string()],
                env: HashMap::new(),
            },
            BuildCommand {
                name: "appbundle".to_string(),
                command: self.flutter_path.to_string_lossy().to_string(),
                args: vec!["build".to_string(), "appbundle".to_string()],
                env: HashMap::new(),
            },
            BuildCommand {
                name: "ios".to_string(),
                command: self.flutter_path.to_string_lossy().to_string(),
                args: vec!["build".to_string(), "ios".to_string()],
                env: HashMap::new(),
            },
            BuildCommand {
                name: "web".to_string(),
                command: self.flutter_path.to_string_lossy().to_string(),
                args: vec!["build".to_string(), "web".to_string()],
                env: HashMap::new(),
            },
        ]
    }

    fn run_command(&self, platform: &str) -> Vec<String> {
        let mut args = vec!["run".to_string()];
        match platform {
            "android" => args.push("-d".to_string()),
            "ios" => args.push("-d".to_string()),
            "web" => args.push("-d".to_string()),
            _ => {}
        }
        args
    }

    fn box_clone(&self) -> Box<dyn MobileFramework> {
        Box::new(self.clone())
    }
}