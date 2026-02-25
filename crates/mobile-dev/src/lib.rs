//! Unified Mobile Development System for Parsec IDE
//!
//! This crate provides a complete mobile development environment supporting
//! Android, iOS, Flutter, React Native, and allowing custom frameworks
//! via the SDK.

#![allow(dead_code, unused_imports, unused_variables)]

pub mod platform;
pub mod device;
pub mod build;
pub mod debug;
pub mod frameworks;
pub mod templates;
pub mod sdk;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::sync::{RwLock, Mutex};
use tracing::{info, warn, debug};

pub use platform::{Platform, PlatformType, PlatformDetector};
pub use device::{DeviceManager, DeviceType, DeviceInfo};
pub use build::{BuildSystem, BuildConfig, BuildResult, BuildTarget, BuildConfiguration};
pub use debug::{Debugger, DebugSession, LayoutInspector, PerformanceProfiler, NetworkMonitor};
pub use frameworks::{MobileFramework, FrameworkType, FrameworkManager};
pub use templates::{TemplateManager, ProjectTemplate};
pub use sdk::{MobileSDK, FrameworkExtension};

/// Main mobile development manager
pub struct MobileDevManager {
    /// Detected platforms
    platforms: Arc<RwLock<HashMap<String, Box<dyn Platform>>>>,
    /// Device manager
    device_manager: Arc<DeviceManager>,
    /// Build system
    build_system: Arc<BuildSystem>,
    /// Debugger
    debugger: Arc<Debugger>,
    /// Framework manager
    framework_manager: Arc<FrameworkManager>,
    /// Template manager
    template_manager: Arc<TemplateManager>,
    /// SDK for extensions
    sdk: Arc<MobileSDK>,
    /// Configuration
    config: MobileDevConfig,
    /// Current project
    current_project: Arc<RwLock<Option<MobileProject>>>,
}

/// Mobile development configuration
#[derive(Debug, Clone)]
pub struct MobileDevConfig {
    pub android_sdk_path: Option<PathBuf>,
    pub android_ndk_path: Option<PathBuf>,
    pub java_home: Option<PathBuf>,
    pub xcode_path: Option<PathBuf>,
    pub flutter_sdk_path: Option<PathBuf>,
    pub react_native_path: Option<PathBuf>,
    pub projects_dir: PathBuf,
    pub build_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub auto_detect_devices: bool,
    pub max_concurrent_builds: usize,
}

impl Default for MobileDevConfig {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("parsec/mobile-dev");

        Self {
            android_sdk_path: std::env::var("ANDROID_HOME").ok().map(PathBuf::from),
            android_ndk_path: std::env::var("ANDROID_NDK").ok().map(PathBuf::from),
            java_home: std::env::var("JAVA_HOME").ok().map(PathBuf::from),
            xcode_path: None,
            flutter_sdk_path: std::env::var("FLUTTER_HOME").ok().map(PathBuf::from),
            react_native_path: None,
            projects_dir: data_dir.join("projects"),
            build_dir: data_dir.join("build"),
            cache_dir: data_dir.join("cache"),
            auto_detect_devices: true,
            max_concurrent_builds: 4,
        }
    }
}

/// Mobile project
#[derive(Debug, Clone)]
pub struct MobileProject {
    pub name: String,
    pub path: PathBuf,
    pub framework: FrameworkType,
    pub platform_targets: Vec<PlatformType>,
    pub config: ProjectConfig,
}

/// Project configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectConfig {
    pub package_name: String,
    pub version: String,
    pub build_number: i32,
    pub min_sdk: Option<String>,
    pub target_sdk: Option<String>,
    pub signing_config: Option<SigningConfig>,
}

/// Signing configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SigningConfig {
    pub keystore_path: PathBuf,
    pub keystore_password: String,
    pub key_alias: String,
    pub key_password: Option<String>,
}

impl MobileDevManager {
    /// Create a new mobile development manager
    pub fn new(config: MobileDevConfig) -> Result<Self> {
        // Create directories
        std::fs::create_dir_all(&config.projects_dir)?;
        std::fs::create_dir_all(&config.build_dir)?;
        std::fs::create_dir_all(&config.cache_dir)?;

        Ok(Self {
            platforms: Arc::new(RwLock::new(HashMap::new())),
            device_manager: Arc::new(DeviceManager::new()?),
            build_system: Arc::new(BuildSystem::new(config.build_dir.clone())?),
            debugger: Arc::new(Debugger::new()?),
            framework_manager: Arc::new(FrameworkManager::new()?),
            template_manager: Arc::new(TemplateManager::new()?),
            sdk: Arc::new(MobileSDK::new()?),
            config,
            current_project: Arc::new(RwLock::new(None)),
        })
    }

    /// Detect all platforms
    pub async fn detect_platforms(&self) -> Result<Vec<String>> {
        let mut platforms = self.platforms.write().await;

        // Detect Android
        if let Ok(android) = platform::AndroidPlatform::detect().await {
            platforms.insert("android".to_string(), Box::new(android));
        }

        // Detect iOS (macOS only)
        #[cfg(target_os = "macos")]
        if let Ok(ios) = platform::IOSPlatform::detect().await {
            platforms.insert("ios".to_string(), Box::new(ios));
        }

        // Detect custom platforms via SDK
        if let Ok(custom) = self.sdk.detect_custom_platforms().await {
            for (name, platform) in custom {
                platforms.insert(name, platform);
            }
        }

        Ok(platforms.keys().cloned().collect())
    }

    /// Open a mobile project
    pub async fn open_project<P: AsRef<Path>>(&self, path: P) -> Result<MobileProject> {
        let path = path.as_ref();
        
        // Detect project framework
        let framework = self.detect_framework(path).await?;
        
        // Load project configuration
        let config = self.load_project_config(path, &framework).await?;
        
        let project = MobileProject {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            path: path.to_path_buf(),
            framework: framework.clone(),
            platform_targets: self.get_target_platforms(&framework).await?,
            config,
        };

        *self.current_project.write().await = Some(project.clone());
        info!("Opened mobile project: {} ({:?})", project.name, project.framework);
        
        Ok(project)
    }

    /// Create a new project from template
    pub async fn create_project(
        &self,
        name: &str,
        framework: FrameworkType,
        template: &str,
        target_dir: PathBuf,
    ) -> Result<MobileProject> {
        let template_path = self.template_manager.get_template(framework.clone(), template)?;
        
        // Copy template to target directory
        self.template_manager.copy_template(&template_path, &target_dir)?;
        
        // Create project configuration
        let config = self.create_project_config(name, &framework).await?;
        
        let project = MobileProject {
            name: name.to_string(),
            path: target_dir,
            framework: framework.clone(),
            platform_targets: self.get_target_platforms(&framework).await?,
            config,
        };

        // Save project metadata
        self.save_project_metadata(&project).await?;

        info!("Created new {} project: {}", framework, name);
        Ok(project)
    }

    /// Detect project framework
    async fn detect_framework(&self, path: &Path) -> Result<FrameworkType> {
        // Check Flutter
        if path.join("pubspec.yaml").exists() {
            return Ok(FrameworkType::Flutter);
        }

        // Check React Native
        if path.join("package.json").exists() {
            let content = tokio::fs::read_to_string(path.join("package.json")).await?;
            if content.contains("react-native") {
                return Ok(FrameworkType::ReactNative);
            }
        }

        // Check Ionic
        if path.join("ionic.config.json").exists() {
            return Ok(FrameworkType::Ionic);
        }

        // Check native Android
        if path.join("app/src/main").exists() && path.join("build.gradle").exists() {
            return Ok(FrameworkType::NativeAndroid);
        }

        // Check native iOS
        if path.join("*.xcodeproj").exists() || path.join("*.xcworkspace").exists() {
            return Ok(FrameworkType::NativeIOS);
        }

        // Check custom frameworks via SDK
        if let Some(framework) = self.sdk.detect_custom_framework(path).await? {
            return Ok(framework);
        }

        Err(anyhow!("Unknown project framework"))
    }

    /// Load project configuration
    async fn load_project_config(&self, path: &Path, framework: &FrameworkType) -> Result<ProjectConfig> {
        match framework {
            FrameworkType::Flutter => self.load_flutter_config(path).await,
            FrameworkType::ReactNative => self.load_react_native_config(path).await,
            FrameworkType::Ionic => self.load_ionic_config(path).await,
            FrameworkType::NativeAndroid => self.load_android_config(path).await,
            FrameworkType::NativeIOS => self.load_ios_config(path).await,
            FrameworkType::Custom(name) => self.load_custom_config(path, name).await,
        }
    }

    /// Load Flutter project configuration
    async fn load_flutter_config(&self, path: &Path) -> Result<ProjectConfig> {
        let pubspec_path = path.join("pubspec.yaml");
        let content = tokio::fs::read_to_string(pubspec_path).await?;
        
        // Parse YAML (simplified)
        let mut config = ProjectConfig {
            package_name: path.file_name().unwrap().to_string_lossy().to_string(),
            version: "1.0.0".to_string(),
            build_number: 1,
            min_sdk: None,
            target_sdk: None,
            signing_config: None,
        };

        for line in content.lines() {
            if line.starts_with("name:") {
                config.package_name = line.split(':').nth(1).unwrap().trim().to_string();
            }
            if line.starts_with("version:") {
                let version_str = line.split(':').nth(1).unwrap().trim();
                if let Some((ver, build)) = version_str.split_once('+') {
                    config.version = ver.to_string();
                    config.build_number = build.parse().unwrap_or(1);
                } else {
                    config.version = version_str.to_string();
                }
            }
        }

        Ok(config)
    }

    /// Load React Native project configuration
    async fn load_react_native_config(&self, path: &Path) -> Result<ProjectConfig> {
        let package_path = path.join("package.json");
        let content = tokio::fs::read_to_string(package_path).await?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        Ok(ProjectConfig {
            package_name: json["name"].as_str().unwrap_or("").to_string(),
            version: json["version"].as_str().unwrap_or("1.0.0").to_string(),
            build_number: 1,
            min_sdk: None,
            target_sdk: None,
            signing_config: None,
        })
    }

    /// Load Ionic project configuration
    async fn load_ionic_config(&self, path: &Path) -> Result<ProjectConfig> {
        let package_path = path.join("package.json");
        let content = tokio::fs::read_to_string(package_path).await?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        Ok(ProjectConfig {
            package_name: json["name"].as_str().unwrap_or("").to_string(),
            version: json["version"].as_str().unwrap_or("1.0.0").to_string(),
            build_number: 1,
            min_sdk: None,
            target_sdk: None,
            signing_config: None,
        })
    }

    /// Load Android project configuration
    async fn load_android_config(&self, path: &Path) -> Result<ProjectConfig> {
        let gradle_path = path.join("app/build.gradle");
        let content = tokio::fs::read_to_string(gradle_path).await?;
        
        let mut config = ProjectConfig {
            package_name: "com.example.app".to_string(),
            version: "1.0.0".to_string(),
            build_number: 1,
            min_sdk: None,
            target_sdk: None,
            signing_config: None,
        };

        for line in content.lines() {
            if line.contains("applicationId") {
                config.package_name = line.split('"').nth(1).unwrap_or("").to_string();
            }
            if line.contains("versionName") {
                config.version = line.split('"').nth(1).unwrap_or("1.0.0").to_string();
            }
            if line.contains("versionCode") {
                config.build_number = line.split_whitespace().last()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1);
            }
            if line.contains("minSdkVersion") {
                config.min_sdk = line.split_whitespace().last().map(|s| s.to_string());
            }
            if line.contains("targetSdkVersion") {
                config.target_sdk = line.split_whitespace().last().map(|s| s.to_string());
            }
        }

        Ok(config)
    }

    /// Load iOS project configuration
    async fn load_ios_config(&self, path: &Path) -> Result<ProjectConfig> {
        // Find Info.plist
        let info_plist = self.find_info_plist(path).await?;
        let content = tokio::fs::read_to_string(info_plist).await?;
        
        // Parse Info.plist (simplified)
        let mut config = ProjectConfig {
            package_name: "com.example.app".to_string(),
            version: "1.0.0".to_string(),
            build_number: 1,
            min_sdk: None,
            target_sdk: None,
            signing_config: None,
        };

        for line in content.lines() {
            if line.contains("CFBundleIdentifier") {
                if let Some(next) = content.lines().skip_while(|l| !l.contains("CFBundleIdentifier")).nth(1) {
                    config.package_name = next.trim().trim_matches('<').trim_matches('>').to_string();
                }
            }
            if line.contains("CFBundleShortVersionString") {
                if let Some(next) = content.lines().skip_while(|l| !l.contains("CFBundleShortVersionString")).nth(1) {
                    config.version = next.trim().trim_matches('<').trim_matches('>').to_string();
                }
            }
            if line.contains("CFBundleVersion") {
                if let Some(next) = content.lines().skip_while(|l| !l.contains("CFBundleVersion")).nth(1) {
                    config.build_number = next.trim().trim_matches('<').trim_matches('>').parse().unwrap_or(1);
                }
            }
        }

        Ok(config)
    }

    /// Load custom project configuration
    async fn load_custom_config(&self, path: &Path, _name: &str) -> Result<ProjectConfig> {
        // Look for .parsec-mobile file
        let config_file = path.join(".parsec-mobile");
        if config_file.exists() {
            let content = tokio::fs::read_to_string(config_file).await?;
            let config: ProjectConfig = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(ProjectConfig {
                package_name: path.file_name().unwrap().to_string_lossy().to_string(),
                version: "1.0.0".to_string(),
                build_number: 1,
                min_sdk: None,
                target_sdk: None,
                signing_config: None,
            })
        }
    }

    /// Find Info.plist in iOS project
    async fn find_info_plist(&self, path: &Path) -> Result<PathBuf> {
        let mut stack = vec![path.to_path_buf()];
        
        while let Some(dir) = stack.pop() {
            let mut read_dir = tokio::fs::read_dir(&dir).await?;
            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.file_name().and_then(|n| n.to_str()) == Some("Info.plist") {
                    return Ok(path);
                }
            }
        }

        Err(anyhow!("Info.plist not found"))
    }

    /// Create project configuration
    async fn create_project_config(&self, name: &str, framework: &FrameworkType) -> Result<ProjectConfig> {
        let package_name = format!("com.{}.{}", framework.to_string().to_lowercase(), name.to_lowercase());

        Ok(ProjectConfig {
            package_name,
            version: "1.0.0".to_string(),
            build_number: 1,
            min_sdk: match framework {
                FrameworkType::Flutter => Some("21".to_string()),
                FrameworkType::ReactNative => Some("21".to_string()),
                FrameworkType::NativeAndroid => Some("21".to_string()),
                _ => None,
            },
            target_sdk: match framework {
                FrameworkType::Flutter => Some("33".to_string()),
                FrameworkType::ReactNative => Some("33".to_string()),
                FrameworkType::NativeAndroid => Some("33".to_string()),
                _ => None,
            },
            signing_config: None,
        })
    }

    /// Get target platforms for framework
    async fn get_target_platforms(&self, framework: &FrameworkType) -> Result<Vec<PlatformType>> {
        Ok(match framework {
            FrameworkType::Flutter => vec![PlatformType::Android, PlatformType::IOS],
            FrameworkType::ReactNative => vec![PlatformType::Android, PlatformType::IOS],
            FrameworkType::Ionic => vec![PlatformType::Android, PlatformType::IOS, PlatformType::Web],
            FrameworkType::NativeAndroid => vec![PlatformType::Android],
            FrameworkType::NativeIOS => vec![PlatformType::IOS],
            FrameworkType::Custom(_) => vec![], // Custom frameworks define their own
        })
    }

    /// Save project metadata
    async fn save_project_metadata(&self, project: &MobileProject) -> Result<()> {
        let metadata_path = self.config.projects_dir.join(format!("{}.json", project.name));
        let metadata = serde_json::json!({
            "name": project.name,
            "path": project.path,
            "framework": format!("{:?}", project.framework),
            "config": project.config,
        });
        tokio::fs::write(metadata_path, serde_json::to_string_pretty(&metadata)?).await?;
        Ok(())
    }

    /// Build current project
    pub async fn build_project(&self, target: PlatformType, config: BuildConfig) -> Result<BuildResult> {
        let project = self.current_project.read().await.clone()
            .ok_or_else(|| anyhow!("No project open"))?;

        self.build_system.build(&project, target, config).await
    }

    /// Run on device
    pub async fn run_on_device(&self, device_id: &str, target: PlatformType) -> Result<()> {
        let project = self.current_project.read().await.clone()
            .ok_or_else(|| anyhow!("No project open"))?;

        // Build first
        let build_config = BuildConfig {
            target: target.clone(),
            configuration: BuildConfiguration::Debug,
            output_path: Some(self.config.build_dir.join(&project.name)),
            clean_before_build: true,
            env_vars: HashMap::new(),
            extra_args: vec![],
        };

        let build_result = self.build_system.build(&project, target.clone(), build_config).await?;
        
        if !build_result.success {
            return Err(anyhow!("Build failed"));
        }

        // Install and run on device
        self.device_manager.install_app(device_id, &build_result.output_path.unwrap()).await?;
        self.device_manager.run_app(device_id, &project.config.package_name).await?;

        Ok(())
    }

    /// Start debugging session
    pub async fn start_debug_session(&self, device_id: &str) -> Result<DebugSession> {
        let project = self.current_project.read().await.clone()
            .ok_or_else(|| anyhow!("No project open"))?;

        self.debugger.start_session(&project, device_id).await
    }

    /// List connected devices
    pub async fn list_devices(&self) -> Vec<DeviceInfo> {
        self.device_manager.list_devices().await
    }

    /// Refresh device list
    pub async fn refresh_devices(&self) -> Result<()> {
        self.device_manager.refresh().await
    }

    /// Get project templates
    pub async fn list_templates(&self, framework: Option<FrameworkType>) -> Vec<TemplateInfo> {
        self.template_manager.list_templates(framework)
            .into_iter()
            .map(|t| TemplateInfo {
                name: t.name,
                description: t.description,
                framework: t.framework,
                version: t.version,
                icon: t.icon,
            })
            .collect()
    }

    /// Register a custom framework extension
    pub async fn register_framework_extension(&self, extension: Box<dyn FrameworkExtension>) -> Result<()> {
        self.sdk.register_extension(extension).await
    }
}

/// Template information
#[derive(Debug, Clone)]
pub struct TemplateInfo {
    pub name: String,
    pub description: String,
    pub framework: FrameworkType,
    pub version: String,
    pub icon: Option<PathBuf>,
}