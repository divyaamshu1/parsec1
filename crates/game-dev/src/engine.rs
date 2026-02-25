//! Game engine detection and management

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use tracing::{info, warn, debug};

/// Game engine type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EngineType {
    Unity,
    Unreal,
    Godot,
    Custom(String),
}

impl std::fmt::Display for EngineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineType::Unity => write!(f, "Unity"),
            EngineType::Unreal => write!(f, "Unreal"),
            EngineType::Godot => write!(f, "Godot"),
            EngineType::Custom(name) => write!(f, "{}", name),
        }
    }
}

impl EngineType {
    pub fn as_str(&self) -> &str {
        match self {
            EngineType::Unity => "unity",
            EngineType::Unreal => "unreal",
            EngineType::Godot => "godot",
            EngineType::Custom(name) => name.as_str(),
        }
    }
}

/// Game engine trait - all engines must implement this
#[async_trait]
pub trait GameEngine: Send + Sync {
    fn engine_type(&self) -> EngineType;
    fn name(&self) -> String;
    fn version(&self) -> Option<String>;
    fn install_path(&self) -> PathBuf;
    
    /// Detect if engine is installed
    async fn detect() -> Result<Self> where Self: Sized;
    
    /// Check if a path is a valid project for this engine
    fn is_valid_project(&self, path: &Path) -> bool;
    
    /// Get language servers needed for this engine
    fn language_servers(&self) -> Vec<LanguageServerConfig>;
    
    /// Get build targets for this engine
    fn build_targets(&self) -> Vec<BuildTarget>;
    
    /// Build project
    async fn build(&self, project: &Project, config: &BuildConfig) -> Result<BuildResult>;
    
    /// Run project
    async fn run(&self, project: &Project, config: &RunConfig) -> Result<()>;
    
    /// Debug project
    async fn debug(&self, project: &Project, config: &DebugConfig) -> Result<DebugSession>;
    
    /// Clone boxed engine
    fn box_clone(&self) -> Box<dyn GameEngine>;
}

/// Language server configuration
#[derive(Debug, Clone)]
pub struct LanguageServerConfig {
    pub language: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

/// Build target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildTarget {
    pub name: String,
    pub platform: String,
    pub arch: String,
    pub config: String,
    pub output_path: PathBuf,
}

/// Run configuration
#[derive(Debug, Clone)]
pub struct RunConfig {
    pub target: BuildTarget,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

/// Engine detector - finds installed engines
pub struct EngineDetector {
    /// Custom engine detectors from SDK
    custom_detectors: Vec<Box<dyn CustomEngineDetector>>,
}

#[async_trait]
pub trait CustomEngineDetector: Send + Sync {
    fn name(&self) -> &str;
    async fn detect(&self) -> Option<Box<dyn GameEngine>>;
}

impl EngineDetector {
    pub fn new() -> Self {
        Self {
            custom_detectors: Vec::new(),
        }
    }

    pub fn register_detector(&mut self, detector: Box<dyn CustomEngineDetector>) {
        self.custom_detectors.push(detector);
    }

    /// Detect all installed engines
    pub async fn detect_all(&self) -> Vec<Box<dyn GameEngine>> {
        let mut engines = Vec::new();

        // Detect Unity
        if let Ok(unity) = UnityEngine::detect().await {
            engines.push(Box::new(unity) as Box<dyn GameEngine>);
        }

        // Detect Unreal
        if let Ok(unreal) = UnrealEngine::detect().await {
            engines.push(Box::new(unreal));
        }

        // Detect Godot
        if let Ok(godot) = GodotEngine::detect().await {
            engines.push(Box::new(godot));
        }

        // Detect custom engines
        for detector in &self.custom_detectors {
            if let Some(engine) = detector.detect().await {
                engines.push(engine);
            }
        }

        engines
    }
}

/// Unity Engine implementation
#[derive(Debug, Clone)]
pub struct UnityEngine {
    path: PathBuf,
    version: String,
    editions: Vec<String>,
}

#[async_trait]
impl GameEngine for UnityEngine {
    fn engine_type(&self) -> EngineType {
        EngineType::Unity
    }

    fn name(&self) -> String {
        format!("Unity {}", self.version)
    }

    fn version(&self) -> Option<String> {
        Some(self.version.clone())
    }

    fn install_path(&self) -> PathBuf {
        self.path.clone()
    }

    async fn detect() -> Result<Self> {
        // Common Unity installation paths
        let paths = if cfg!(windows) {
            vec![
                PathBuf::from("C:\\Program Files\\Unity\\Hub\\Editor"),
                PathBuf::from("C:\\Program Files (x86)\\Unity\\Hub\\Editor"),
            ]
        } else if cfg!(target_os = "macos") {
            vec![
                PathBuf::from("/Applications/Unity/Hub/Editor"),
                PathBuf::from("~/Applications/Unity/Hub/Editor").expand(),
            ]
        } else {
            vec![
                PathBuf::from("~/Unity/Hub/Editor").expand(),
            ]
        };

        for base_path in paths {
            if base_path.exists() {
                if let Ok(entries) = std::fs::read_dir(base_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            if let Some(name) = path.file_name() {
                                let version = name.to_string_lossy().to_string();
                                return Ok(UnityEngine {
                                    path,
                                    version,
                                    editions: vec!["personal".to_string(), "pro".to_string()],
                                });
                            }
                        }
                    }
                }
            }
        }

        Err(anyhow!("Unity not found"))
    }

    fn is_valid_project(&self, path: &Path) -> bool {
        path.join("Assets").exists() && path.join("ProjectSettings").exists()
    }

    fn language_servers(&self) -> Vec<LanguageServerConfig> {
        vec![
            LanguageServerConfig {
                language: "csharp".to_string(),
                command: "dotnet".to_string(),
                args: vec!["OmniSharp".to_string()],
                env: HashMap::new(),
            }
        ]
    }

    fn build_targets(&self) -> Vec<BuildTarget> {
        vec![
            BuildTarget {
                name: "Windows".to_string(),
                platform: "windows".to_string(),
                arch: "x64".to_string(),
                config: "development".to_string(),
                output_path: PathBuf::from("Build/Windows"),
            },
            BuildTarget {
                name: "macOS".to_string(),
                platform: "macos".to_string(),
                arch: "x64".to_string(),
                config: "development".to_string(),
                output_path: PathBuf::from("Build/macOS"),
            },
            BuildTarget {
                name: "Linux".to_string(),
                platform: "linux".to_string(),
                arch: "x64".to_string(),
                config: "development".to_string(),
                output_path: PathBuf::from("Build/Linux"),
            },
            BuildTarget {
                name: "Android".to_string(),
                platform: "android".to_string(),
                arch: "arm64".to_string(),
                config: "development".to_string(),
                output_path: PathBuf::from("Build/Android"),
            },
            BuildTarget {
                name: "iOS".to_string(),
                platform: "ios".to_string(),
                arch: "arm64".to_string(),
                config: "development".to_string(),
                output_path: PathBuf::from("Build/iOS"),
            },
            BuildTarget {
                name: "WebGL".to_string(),
                platform: "webgl".to_string(),
                arch: "wasm".to_string(),
                config: "development".to_string(),
                output_path: PathBuf::from("Build/WebGL"),
            },
        ]
    }

    async fn build(&self, project: &Project, config: &BuildConfig) -> Result<BuildResult> {
        let start = std::time::Instant::now();

        // Construct Unity build command
        let mut cmd = Command::new(self.path.join("Unity.exe"));
        cmd.args(&[
            "-batchmode",
            "-projectPath", project.path().to_str().unwrap(),
            "-executeMethod", "BuildScript.PerformBuild",
            "-buildTarget", &config.target.platform,
            "-logFile", "-",
        ]);

        if let Some(output) = &config.output_path {
            cmd.arg("-buildPath").arg(output);
        }

        let output = cmd.output()?;

        Ok(BuildResult {
            success: output.status.success(),
            duration: start.elapsed(),
            output_path: config.output_path.clone(),
            logs: String::from_utf8_lossy(&output.stdout).to_string(),
            errors: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn run(&self, project: &Project, config: &RunConfig) -> Result<()> {
        let exe_path = config.target.output_path.join(project.name()).with_extension("exe");
        
        let mut cmd = Command::new(exe_path);
        cmd.args(&config.args);
        cmd.envs(&config.env);
        
        cmd.spawn()?;
        Ok(())
    }

    async fn debug(&self, project: &Project, config: &DebugConfig) -> Result<DebugSession> {
        // Use DAP to attach debugger
        let session = DebugSession::new(
            "unity".to_string(),
            config.executable.clone(),
            config.args.clone(),
        )?;
        Ok(session)
    }

    fn box_clone(&self) -> Box<dyn GameEngine> {
        Box::new(self.clone())
    }
}

/// Unreal Engine implementation
#[derive(Debug, Clone)]
pub struct UnrealEngine {
    path: PathBuf,
    version: String,
}

#[async_trait]
impl GameEngine for UnrealEngine {
    fn engine_type(&self) -> EngineType {
        EngineType::Unreal
    }

    fn name(&self) -> String {
        format!("Unreal Engine {}", self.version)
    }

    fn version(&self) -> Option<String> {
        Some(self.version.clone())
    }

    fn install_path(&self) -> PathBuf {
        self.path.clone()
    }

    async fn detect() -> Result<Self> {
        let paths = if cfg!(windows) {
            vec![
                PathBuf::from("C:\\Program Files\\Epic Games\\UE_5.3"),
                PathBuf::from("C:\\Program Files\\Epic Games\\UE_5.2"),
                PathBuf::from("C:\\Program Files\\Epic Games\\UE_5.1"),
                PathBuf::from("C:\\Program Files\\Epic Games\\UE_5.0"),
            ]
        } else {
            vec![
                PathBuf::from("/Applications/Unreal Engine.app"),
            ]
        };

        for path in paths {
            if path.exists() {
                let version = path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                return Ok(UnrealEngine { path, version });
            }
        }

        Err(anyhow!("Unreal Engine not found"))
    }

    fn is_valid_project(&self, path: &Path) -> bool {
        path.join("Source").exists() && 
        path.join("Config").exists() &&
        path.join("Content").exists()
    }

    fn language_servers(&self) -> Vec<LanguageServerConfig> {
        vec![
            LanguageServerConfig {
                language: "cpp".to_string(),
                command: "clangd".to_string(),
                args: vec![],
                env: HashMap::new(),
            },
            LanguageServerConfig {
                language: "blueprint".to_string(),
                command: "unreal-blueprint-lsp".to_string(),
                args: vec![],
                env: HashMap::new(),
            },
        ]
    }

    fn build_targets(&self) -> Vec<BuildTarget> {
        vec![
            BuildTarget {
                name: "Windows".to_string(),
                platform: "Win64".to_string(),
                arch: "x64".to_string(),
                config: "Development".to_string(),
                output_path: PathBuf::from("Binaries/Win64"),
            },
            BuildTarget {
                name: "macOS".to_string(),
                platform: "Mac".to_string(),
                arch: "x64".to_string(),
                config: "Development".to_string(),
                output_path: PathBuf::from("Binaries/Mac"),
            },
            BuildTarget {
                name: "Linux".to_string(),
                platform: "Linux".to_string(),
                arch: "x64".to_string(),
                config: "Development".to_string(),
                output_path: PathBuf::from("Binaries/Linux"),
            },
            BuildTarget {
                name: "Android".to_string(),
                platform: "Android".to_string(),
                arch: "arm64".to_string(),
                config: "Development".to_string(),
                output_path: PathBuf::from("Binaries/Android"),
            },
            BuildTarget {
                name: "iOS".to_string(),
                platform: "IOS".to_string(),
                arch: "arm64".to_string(),
                config: "Development".to_string(),
                output_path: PathBuf::from("Binaries/IOS"),
            },
        ]
    }

    async fn build(&self, project: &Project, config: &BuildConfig) -> Result<BuildResult> {
        let start = std::time::Instant::now();

        let build_tool = self.path.join("Engine/Build/BatchFiles");
        let script = if cfg!(windows) {
            build_tool.join("Build.bat")
        } else {
            build_tool.join("Build.sh")
        };

        let mut cmd = Command::new(script);
        cmd.args(&[
            &config.target.platform,
            &config.target.config,
            project.name(),
            "-Project=", project.path().to_str().unwrap(),
        ]);

        let output = cmd.output()?;

        Ok(BuildResult {
            success: output.status.success(),
            duration: start.elapsed(),
            output_path: config.output_path.clone(),
            logs: String::from_utf8_lossy(&output.stdout).to_string(),
            errors: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn run(&self, project: &Project, config: &RunConfig) -> Result<()> {
        let exe_path = config.target.output_path.join(project.name()).with_extension("exe");
        
        let mut cmd = Command::new(exe_path);
        cmd.args(&config.args);
        cmd.envs(&config.env);
        
        cmd.spawn()?;
        Ok(())
    }

    async fn debug(&self, project: &Project, config: &DebugConfig) -> Result<DebugSession> {
        let session = DebugSession::new(
            "unreal".to_string(),
            config.executable.clone(),
            config.args.clone(),
        )?;
        Ok(session)
    }

    fn box_clone(&self) -> Box<dyn GameEngine> {
        Box::new(self.clone())
    }
}

/// Godot Engine implementation
#[derive(Debug, Clone)]
pub struct GodotEngine {
    path: PathBuf,
    version: String,
}

#[async_trait]
impl GameEngine for GodotEngine {
    fn engine_type(&self) -> EngineType {
        EngineType::Godot
    }

    fn name(&self) -> String {
        format!("Godot Engine {}", self.version)
    }

    fn version(&self) -> Option<String> {
        Some(self.version.clone())
    }

    fn install_path(&self) -> PathBuf {
        self.path.clone()
    }

    async fn detect() -> Result<Self> {
        // Check common installation paths
        let paths = if cfg!(windows) {
            vec![
                PathBuf::from("C:\\Program Files\\Godot"),
                PathBuf::from("C:\\Program Files (x86)\\Godot"),
            ]
        } else if cfg!(target_os = "macos") {
            vec![
                PathBuf::from("/Applications/Godot.app"),
            ]
        } else {
            vec![
                PathBuf::from("/usr/bin/godot"),
                PathBuf::from("/usr/local/bin/godot"),
            ]
        };

        for path in paths {
            if path.exists() {
                // Try to get version
                let output = Command::new(&path).arg("--version").output().ok();
                let version = output
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .unwrap_or_else(|| "unknown".to_string())
                    .trim()
                    .to_string();

                return Ok(GodotEngine { path, version });
            }
        }

        Err(anyhow!("Godot not found"))
    }

    fn is_valid_project(&self, path: &Path) -> bool {
        path.join("project.godot").exists()
    }

    fn language_servers(&self) -> Vec<LanguageServerConfig> {
        vec![
            LanguageServerConfig {
                language: "gdscript".to_string(),
                command: "godot".to_string(),
                args: vec!["--language-server".to_string()],
                env: HashMap::new(),
            },
            LanguageServerConfig {
                language: "csharp".to_string(),
                command: "dotnet".to_string(),
                args: vec!["OmniSharp".to_string()],
                env: HashMap::new(),
            },
        ]
    }

    fn build_targets(&self) -> Vec<BuildTarget> {
        vec![
            BuildTarget {
                name: "Windows Desktop".to_string(),
                platform: "windows".to_string(),
                arch: "x64".to_string(),
                config: "release".to_string(),
                output_path: PathBuf::from("build/windows"),
            },
            BuildTarget {
                name: "macOS".to_string(),
                platform: "macos".to_string(),
                arch: "universal".to_string(),
                config: "release".to_string(),
                output_path: PathBuf::from("build/macos"),
            },
            BuildTarget {
                name: "Linux".to_string(),
                platform: "linux".to_string(),
                arch: "x64".to_string(),
                config: "release".to_string(),
                output_path: PathBuf::from("build/linux"),
            },
            BuildTarget {
                name: "Android".to_string(),
                platform: "android".to_string(),
                arch: "arm64".to_string(),
                config: "release".to_string(),
                output_path: PathBuf::from("build/android"),
            },
            BuildTarget {
                name: "iOS".to_string(),
                platform: "ios".to_string(),
                arch: "arm64".to_string(),
                config: "release".to_string(),
                output_path: PathBuf::from("build/ios"),
            },
            BuildTarget {
                name: "Web".to_string(),
                platform: "web".to_string(),
                arch: "wasm".to_string(),
                config: "release".to_string(),
                output_path: PathBuf::from("build/web"),
            },
        ]
    }

    async fn build(&self, project: &Project, config: &BuildConfig) -> Result<BuildResult> {
        let start = std::time::Instant::now();

        let mut cmd = Command::new(&self.path);
        cmd.args(&[
            "--headless",
            "--path", project.path().to_str().unwrap(),
            "--export", &config.target.name,
            config.output_path.as_ref().unwrap().to_str().unwrap(),
        ]);

        let output = cmd.output()?;

        Ok(BuildResult {
            success: output.status.success(),
            duration: start.elapsed(),
            output_path: config.output_path.clone(),
            logs: String::from_utf8_lossy(&output.stdout).to_string(),
            errors: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn run(&self, project: &Project, config: &RunConfig) -> Result<()> {
        let mut cmd = Command::new(&self.path);
        cmd.arg("--path").arg(project.path());
        cmd.args(&config.args);
        cmd.envs(&config.env);
        
        cmd.spawn()?;
        Ok(())
    }

    async fn debug(&self, project: &Project, config: &DebugConfig) -> Result<DebugSession> {
        let session = DebugSession::new(
            "godot".to_string(),
            self.path.clone(),
            vec!["--path".to_string(), project.path().to_string_lossy().to_string()],
        )?;
        Ok(session)
    }

    fn box_clone(&self) -> Box<dyn GameEngine> {
        Box::new(self.clone())
    }
}