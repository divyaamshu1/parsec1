//! Unified Game Development System for Parsec IDE
//!
//! This crate provides a complete game development environment supporting
//! all major engines (Unity, Unreal, Godot) and allowing custom engines
//! via the SDK.

#![allow(dead_code, unused_imports, unused_variables)]

pub mod engine;
pub mod project;
pub mod build;
pub mod debug;
pub mod assets;
pub mod lsp;
pub mod blueprint;
pub mod templates;
pub mod sdk;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::sync::{RwLock, Mutex};
use tracing::{info, warn, debug};

pub use engine::{GameEngine, EngineType, EngineDetector};
pub use project::{Project, ProjectType, ProjectConfig};
pub use build::{BuildSystem, BuildConfig, BuildResult, BuildTarget};
pub use debug::{Debugger, DebugSession, Breakpoint, DAPAdapter};
pub use assets::{AssetManager, AssetType, AssetImporter, AssetPreview};
pub use lsp::{LanguageServer, LanguageServerManager};
pub use blueprint::{BlueprintViewer, BlueprintConverter};
pub use templates::{TemplateManager, ProjectTemplate};
pub use sdk::{GameEngineSDK, EngineExtension};

/// Main game development manager
pub struct GameDevManager {
    /// Detected game engines
    engines: Arc<RwLock<HashMap<String, Box<dyn GameEngine>>>>,
    /// Current project
    current_project: Arc<RwLock<Option<Project>>>,
    /// Asset manager
    asset_manager: Arc<AssetManager>,
    /// Build system
    build_system: Arc<BuildSystem>,
    /// Debugger
    debugger: Arc<Debugger>,
    /// Language server manager
    lsp_manager: Arc<LanguageServerManager>,
    /// Template manager
    template_manager: Arc<TemplateManager>,
    /// SDK for extensions
    sdk: Arc<GameEngineSDK>,
    /// Configuration
    config: GameDevConfig,
}

/// Game development configuration
#[derive(Debug, Clone)]
pub struct GameDevConfig {
    pub engines_dir: PathBuf,
    pub projects_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub build_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub enable_lsp: bool,
    pub enable_debug: bool,
    pub enable_blueprint: bool,
    pub max_concurrent_builds: usize,
    pub auto_detect_engines: bool,
}

impl Default for GameDevConfig {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("parsec/game-dev");

        Self {
            engines_dir: data_dir.join("engines"),
            projects_dir: data_dir.join("projects"),
            assets_dir: data_dir.join("assets"),
            build_dir: data_dir.join("build"),
            cache_dir: data_dir.join("cache"),
            enable_lsp: true,
            enable_debug: true,
            enable_blueprint: true,
            max_concurrent_builds: 4,
            auto_detect_engines: true,
        }
    }
}

impl GameDevManager {
    /// Create a new game development manager
    pub fn new(config: GameDevConfig) -> Result<Self> {
        // Create directories
        std::fs::create_dir_all(&config.engines_dir)?;
        std::fs::create_dir_all(&config.projects_dir)?;
        std::fs::create_dir_all(&config.assets_dir)?;
        std::fs::create_dir_all(&config.build_dir)?;
        std::fs::create_dir_all(&config.cache_dir)?;

        Ok(Self {
            engines: Arc::new(RwLock::new(HashMap::new())),
            current_project: Arc::new(RwLock::new(None)),
            asset_manager: Arc::new(AssetManager::new(config.assets_dir.clone())?),
            build_system: Arc::new(BuildSystem::new(config.build_dir.clone())?),
            debugger: Arc::new(Debugger::new()?),
            lsp_manager: Arc::new(LanguageServerManager::new()?),
            template_manager: Arc::new(TemplateManager::new()?),
            sdk: Arc::new(GameEngineSDK::new()?),
            config,
        })
    }

    /// Detect all installed game engines
    pub async fn detect_engines(&self) -> Result<Vec<String>> {
        let mut engines = self.engines.write().await;
        
        // Detect Unity
        if let Ok(unity) = engine::UnityEngine::detect().await {
            engines.insert("unity".to_string(), Box::new(unity));
        }

        // Detect Unreal
        if let Ok(unreal) = engine::UnrealEngine::detect().await {
            engines.insert("unreal".to_string(), Box::new(unreal));
        }

        // Detect Godot
        if let Ok(godot) = engine::GodotEngine::detect().await {
            engines.insert("godot".to_string(), Box::new(godot));
        }

        // Detect custom engines via SDK
        if let Ok(custom) = self.sdk.detect_custom_engines().await {
            for (name, engine) in custom {
                engines.insert(name, engine);
            }
        }

        Ok(engines.keys().cloned().collect())
    }

    /// Open a game project
    pub async fn open_project<P: AsRef<Path>>(&self, path: P) -> Result<Project> {
        let path = path.as_ref();
        
        // Detect project type
        let project_type = self.detect_project_type(path).await?;
        
        // Create project
        let project = Project::open(path, project_type).await?;
        
        // Set as current
        *self.current_project.write().await = Some(project.clone());

        // Initialize LSP for project languages
        if self.config.enable_lsp {
            self.lsp_manager.init_for_project(&project).await?;
        }

        info!("Opened game project: {} ({:?})", project.name(), project_type);
        Ok(project)
    }

    /// Create a new project from template
    pub async fn create_project(
        &self,
        name: &str,
        engine: EngineType,
        template: &str,
        target_dir: PathBuf,
    ) -> Result<Project> {
        let template_path = self.template_manager.get_template(engine, template)?;
        
        // Copy template to target directory
        self.template_manager.copy_template(&template_path, &target_dir)?;
        
        // Initialize project
        let project = Project::init(target_dir, engine).await?;
        
        // Save to projects list
        project.save_metadata(&self.config.projects_dir)?;

        info!("Created new {} project: {}", engine, name);
        Ok(project)
    }

    /// Build current project
    pub async fn build_project(&self, config: BuildConfig) -> Result<BuildResult> {
        let project = self.current_project.read().await.clone()
            .ok_or_else(|| anyhow!("No project open"))?;

        let engine = self.get_engine_for_project(&project).await?;
        self.build_system.build(&project, engine.as_ref(), config).await
    }

    /// Start debugging session
    pub async fn start_debug_session(&self, config: DebugConfig) -> Result<DebugSession> {
        let project = self.current_project.read().await.clone()
            .ok_or_else(|| anyhow!("No project open"))?;

        let engine = self.get_engine_for_project(&project).await?;
        self.debugger.start_session(&project, engine.as_ref(), config).await
    }

    /// Get engine for project
    async fn get_engine_for_project(&self, project: &Project) -> Result<Box<dyn GameEngine>> {
        let engines = self.engines.read().await;
        let engine = engines.get(project.engine_type().as_str())
            .ok_or_else(|| anyhow!("Engine not found: {}", project.engine_type()))?;
        Ok(engine.box_clone())
    }

    /// Detect project type from path
    async fn detect_project_type(&self, path: &Path) -> Result<ProjectType> {
        // Check Unity
        if path.join("Assets").exists() && path.join("ProjectSettings").exists() {
            return Ok(ProjectType::Unity);
        }

        // Check Unreal
        if path.join(".uprojectdirs").exists() || path.join("Source").exists() {
            return Ok(ProjectType::Unreal);
        }

        // Check Godot
        if path.join("project.godot").exists() {
            return Ok(ProjectType::Godot);
        }

        // Check custom engines via SDK
        if let Some(engine_type) = self.sdk.detect_custom_project(path).await? {
            return Ok(engine_type);
        }

        Err(anyhow!("Unknown project type"))
    }

    /// Import assets into project
    pub async fn import_assets(&self, paths: Vec<PathBuf>) -> Result<Vec<AssetImportResult>> {
        let project = self.current_project.read().await.clone()
            .ok_or_else(|| anyhow!("No project open"))?;

        let engine = self.get_engine_for_project(&project).await?;
        self.asset_manager.import_assets(&project, engine.as_ref(), paths).await
    }

    /// Get project templates
    pub async fn list_templates(&self, engine: Option<EngineType>) -> Vec<TemplateInfo> {
        self.template_manager.list_templates(engine)
    }

    /// Register a custom engine extension
    pub async fn register_engine_extension(&self, extension: Box<dyn EngineExtension>) -> Result<()> {
        self.sdk.register_extension(extension).await
    }
}

/// Debug configuration
#[derive(Debug, Clone)]
pub struct DebugConfig {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: HashMap<String, String>,
    pub breakpoints: Vec<Breakpoint>,
}

/// Asset import result
#[derive(Debug, Clone)]
pub struct AssetImportResult {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub asset_type: AssetType,
    pub success: bool,
    pub error: Option<String>,
}

/// Template information
#[derive(Debug, Clone)]
pub struct TemplateInfo {
    pub name: String,
    pub description: String,
    pub engine: EngineType,
    pub version: String,
    pub icon: Option<PathBuf>,
}