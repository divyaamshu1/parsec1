//! Custom framework support

use std::path::{Path, PathBuf};
use std::collections::HashMap;

use anyhow::{Result, anyhow};
use async_trait::async_trait;

use super::{MobileFramework, FrameworkType, BuildCommand};

/// Custom framework
#[derive(Debug, Clone)]
pub struct CustomFramework {
    name: String,
    version: String,
}

impl CustomFramework {
    pub fn new(name: String, version: String) -> Self {
        Self { name, version }
    }
}

#[async_trait]
impl MobileFramework for CustomFramework {
    fn framework_type(&self) -> FrameworkType {
        FrameworkType::Custom(self.name.clone())
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn version(&self) -> Option<String> {
        Some(self.version.clone())
    }

    async fn detect() -> Result<Self> {
        Err(anyhow!("Custom frameworks cannot be auto-detected"))
    }

    fn is_valid_project(&self, path: &Path) -> bool {
        path.join(".parsec-mobile").exists()
    }

    fn build_commands(&self) -> Vec<BuildCommand> {
        vec![]
    }

    fn run_command(&self, _platform: &str) -> Vec<String> {
        vec![]
    }

    fn box_clone(&self) -> Box<dyn MobileFramework> {
        Box::new(self.clone())
    }
}