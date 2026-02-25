//! Platform detection for mobile development

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use tracing::{info, warn, debug};

/// Platform type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlatformType {
    Android,
    IOS,
    Web,
    Desktop,
    Custom(String),
}

impl std::fmt::Display for PlatformType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlatformType::Android => write!(f, "Android"),
            PlatformType::IOS => write!(f, "iOS"),
            PlatformType::Web => write!(f, "Web"),
            PlatformType::Desktop => write!(f, "Desktop"),
            PlatformType::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Platform trait - all platforms must implement this
#[async_trait]
pub trait Platform: Send + Sync {
    fn platform_type(&self) -> PlatformType;
    fn name(&self) -> String;
    fn version(&self) -> Option<String>;
    fn sdk_path(&self) -> Option<PathBuf>;
    
    /// Detect if platform is installed
    async fn detect() -> Result<Self> where Self: Sized;
    
    /// Check if platform is available
    fn is_available(&self) -> bool;
    
    /// Get platform capabilities
    fn capabilities(&self) -> Vec<String>;
    
    /// Clone boxed platform
    fn box_clone(&self) -> Box<dyn Platform>;
}

/// Platform detector
pub struct PlatformDetector {
    custom_detectors: Vec<Box<dyn CustomPlatformDetector>>,
}

#[async_trait]
pub trait CustomPlatformDetector: Send + Sync {
    fn name(&self) -> &str;
    async fn detect(&self) -> Option<Box<dyn Platform>>;
}

impl PlatformDetector {
    pub fn new() -> Self {
        Self {
            custom_detectors: Vec::new(),
        }
    }

    pub fn register_detector(&mut self, detector: Box<dyn CustomPlatformDetector>) {
        self.custom_detectors.push(detector);
    }

    /// Detect all installed platforms
    pub async fn detect_all(&self) -> Vec<Box<dyn Platform>> {
        let mut platforms = Vec::new();

        // Detect Android
        if let Ok(android) = AndroidPlatform::detect().await {
            platforms.push(Box::new(android) as Box<dyn Platform>);
        }

        // Detect iOS (macOS only)
        #[cfg(target_os = "macos")]
        if let Ok(ios) = IOSPlatform::detect().await {
            platforms.push(Box::new(ios));
        }

        // Detect custom platforms
        for detector in &self.custom_detectors {
            if let Some(platform) = detector.detect().await {
                platforms.push(platform);
            }
        }

        platforms
    }
}

/// Android platform implementation
#[derive(Debug, Clone)]
pub struct AndroidPlatform {
    sdk_path: PathBuf,
    ndk_path: Option<PathBuf>,
    java_home: Option<PathBuf>,
    version: String,
}

#[async_trait]
impl Platform for AndroidPlatform {
    fn platform_type(&self) -> PlatformType {
        PlatformType::Android
    }

    fn name(&self) -> String {
        "Android".to_string()
    }

    fn version(&self) -> Option<String> {
        Some(self.version.clone())
    }

    fn sdk_path(&self) -> Option<PathBuf> {
        Some(self.sdk_path.clone())
    }

    async fn detect() -> Result<Self> {
        // Check ANDROID_HOME environment variable
        let sdk_path = if let Ok(path) = std::env::var("ANDROID_HOME") {
            PathBuf::from(path)
        } else if let Ok(path) = std::env::var("ANDROID_SDK_ROOT") {
            PathBuf::from(path)
        } else {
            // Check common locations
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            let candidates = vec![
                home.join("Android/Sdk"),
                PathBuf::from("C:\\Android\\Sdk"),
                PathBuf::from("C:\\Program Files\\Android\\Android Studio"),
                PathBuf::from("/usr/local/android-sdk"),
                PathBuf::from("/opt/android-sdk"),
            ];

            candidates.into_iter().find(|p: &PathBuf| p.exists())
                .ok_or_else(|| anyhow!("Android SDK not found"))?
        };

        // Check NDK
        let ndk_path = std::env::var("ANDROID_NDK").ok().map(PathBuf::from)
            .or_else(|| {
                let ndk_dir = sdk_path.join("ndk");
                if ndk_dir.exists() {
                    std::fs::read_dir(ndk_dir).ok()?
                        .next()?.ok()
                        .map(|e| e.path())
                } else {
                    None
                }
            });

        // Check Java
        let java_home = std::env::var("JAVA_HOME").ok().map(PathBuf::from);

        // Get Android version
        let version = if let Ok(props) = std::fs::read_to_string(sdk_path.join("platforms").join("android.properties")) {
            props.lines()
                .find(|l| l.starts_with("ro.build.version.release"))
                .and_then(|l| l.split('=').nth(1))
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        } else {
            "unknown".to_string()
        };

        Ok(Self {
            sdk_path,
            ndk_path,
            java_home,
            version,
        })
    }

    fn is_available(&self) -> bool {
        self.sdk_path.exists() && self.sdk_path.join("platform-tools/adb").exists()
    }

    fn capabilities(&self) -> Vec<String> {
        let mut caps = vec![
            "build".to_string(),
            "debug".to_string(),
            "emulator".to_string(),
            "device".to_string(),
        ];

        if self.ndk_path.is_some() {
            caps.push("native".to_string());
        }

        caps
    }

    fn box_clone(&self) -> Box<dyn Platform> {
        Box::new(self.clone())
    }
}

/// iOS platform implementation (macOS only)
#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
pub struct IOSPlatform {
    xcode_path: PathBuf,
    simulator_runtime: String,
    version: String,
}

#[cfg(target_os = "macos")]
#[async_trait]
impl Platform for IOSPlatform {
    fn platform_type(&self) -> PlatformType {
        PlatformType::IOS
    }

    fn name(&self) -> String {
        "iOS".to_string()
    }

    fn version(&self) -> Option<String> {
        Some(self.version.clone())
    }

    fn sdk_path(&self) -> Option<PathBuf> {
        Some(self.xcode_path.join("Contents/Developer/Platforms/iPhoneOS.platform/Developer/SDKs"))
    }

    async fn detect() -> Result<Self> {
        // Check for Xcode
        let xcode_path = if let Ok(path) = std::env::var("XCODE_PATH") {
            PathBuf::from(path)
        } else {
            // Check common locations
            let candidates = vec![
                PathBuf::from("/Applications/Xcode.app"),
                PathBuf::from("/Applications/Xcode-beta.app"),
            ];

            candidates.into_iter().find(|p| p.exists())
                .ok_or_else(|| anyhow!("Xcode not found"))?
        };

        // Get iOS version
        let output = std::process::Command::new("xcrun")
            .args(&["simctl", "list", "runtimes", "iOS"])
            .output()?;

        let version = String::from_utf8_lossy(&output.stdout)
            .lines()
            .find(|l| l.contains("iOS"))
            .and_then(|l| l.split_whitespace().find(|w| w.contains('.')))
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(Self {
            xcode_path,
            simulator_runtime: "com.apple.CoreSimulator.SimRuntime.iOS".to_string(),
            version,
        })
    }

    fn is_available(&self) -> bool {
        self.xcode_path.exists()
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "build".to_string(),
            "debug".to_string(),
            "simulator".to_string(),
            "device".to_string(),
        ]
    }

    fn box_clone(&self) -> Box<dyn Platform> {
        Box::new(self.clone())
    }
}

#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone)]
pub struct IOSPlatform;

#[cfg(not(target_os = "macos"))]
#[async_trait]
impl Platform for IOSPlatform {
    fn platform_type(&self) -> PlatformType { PlatformType::IOS }
    fn name(&self) -> String { "iOS (macOS only)".to_string() }
    fn version(&self) -> Option<String> { None }
    fn sdk_path(&self) -> Option<PathBuf> { None }
    async fn detect() -> Result<Self> { Err(anyhow!("iOS development requires macOS")) }
    fn is_available(&self) -> bool { false }
    fn capabilities(&self) -> Vec<String> { vec![] }
    fn box_clone(&self) -> Box<dyn Platform> { Box::new(self.clone()) }
}