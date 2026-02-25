//! Mobile framework support

mod flutter;
mod ionic;
#[path = "native-android.rs"]
mod native_android;
#[path = "native-ios.rs"]
mod native_ios;
mod custom;
#[path = "react-native.rs"]
mod react_native;

pub use flutter::*;
pub use react_native::*;
pub use ionic::*;
pub use native_android::*;
pub use native_ios::*;
pub use custom::*;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use tracing::{info, warn, debug};

/// Framework type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrameworkType {
    Flutter,
    ReactNative,
    Ionic,
    NativeAndroid,
    NativeIOS,
    Custom(String),
}

impl std::fmt::Display for FrameworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameworkType::Flutter => write!(f, "Flutter"),
            FrameworkType::ReactNative => write!(f, "React Native"),
            FrameworkType::Ionic => write!(f, "Ionic"),
            FrameworkType::NativeAndroid => write!(f, "Native Android"),
            FrameworkType::NativeIOS => write!(f, "Native iOS"),
            FrameworkType::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Mobile framework trait
#[async_trait]
pub trait MobileFramework: Send + Sync {
    fn framework_type(&self) -> FrameworkType;
    fn name(&self) -> String;
    fn version(&self) -> Option<String>;
    
    /// Detect if framework is installed
    async fn detect() -> Result<Self> where Self: Sized;
    
    /// Check if path is a valid project for this framework
    fn is_valid_project(&self, path: &Path) -> bool;
    
    /// Get build commands for this framework
    fn build_commands(&self) -> Vec<BuildCommand>;
    
    /// Get run command
    fn run_command(&self, platform: &str) -> Vec<String>;
    
    /// Clone boxed framework
    fn box_clone(&self) -> Box<dyn MobileFramework>;
}

/// Build command
#[derive(Debug, Clone)]
pub struct BuildCommand {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

/// Framework manager
pub struct FrameworkManager {
    frameworks: HashMap<String, Box<dyn MobileFramework>>,
    custom_detectors: Vec<Box<dyn CustomFrameworkDetector>>,
}

#[async_trait]
pub trait CustomFrameworkDetector: Send + Sync {
    fn name(&self) -> &str;
    async fn detect(&self) -> Option<Box<dyn MobileFramework>>;
}

impl FrameworkManager {
    /// Create new framework manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            frameworks: HashMap::new(),
            custom_detectors: Vec::new(),
        })
    }

    /// Register custom detector
    pub fn register_detector(&mut self, detector: Box<dyn CustomFrameworkDetector>) {
        self.custom_detectors.push(detector);
    }

    /// Detect framework from path
    pub async fn detect_framework(&self, path: &Path) -> Option<FrameworkType> {
        // Check Flutter
        if path.join("pubspec.yaml").exists() {
            return Some(FrameworkType::Flutter);
        }

        // Check React Native
        if path.join("package.json").exists() {
            if let Ok(content) = tokio::fs::read_to_string(path.join("package.json")).await {
                if content.contains("react-native") {
                    return Some(FrameworkType::ReactNative);
                }
            }
        }

        // Check Ionic
        if path.join("ionic.config.json").exists() {
            return Some(FrameworkType::Ionic);
        }

        // Check native Android
        if path.join("app/src/main").exists() && path.join("build.gradle").exists() {
            return Some(FrameworkType::NativeAndroid);
        }

        // Check native iOS
        if path.join("*.xcodeproj").exists() || path.join("*.xcworkspace").exists() {
            return Some(FrameworkType::NativeIOS);
        }

        // Check custom detectors
        for detector in &self.custom_detectors {
            if let Some(framework) = detector.detect().await {
                return Some(framework.framework_type());
            }
        }

        None
    }
}