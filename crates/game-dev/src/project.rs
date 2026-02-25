//! Game project management

use std::path::{Path, PathBuf};
use std::collections::HashMap;

use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::engine::EngineType;

/// Game project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    name: String,
    path: PathBuf,
    engine: EngineType,
    engine_version: Option<String>,
    created_at: DateTime<Utc>,
    last_opened: DateTime<Utc>,
    config: ProjectConfig,
    assets: Vec<AssetInfo>,
    scenes: Vec<SceneInfo>,
    scripts: Vec<ScriptInfo>,
}

/// Project type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectType {
    Unity,
    Unreal,
    Godot,
    Custom,
}

/// Project configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    pub company: Option<String>,
    pub description: Option<String>,
    pub settings: HashMap<String, serde_json::Value>,
}

/// Asset information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub path: PathBuf,
    pub asset_type: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
}

/// Scene information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneInfo {
    pub name: String,
    pub path: PathBuf,
    pub scene_type: String,
    pub objects: usize,
}

/// Script information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptInfo {
    pub name: String,
    pub path: PathBuf,
    pub language: String,
    pub lines: usize,
}

impl Project {
    /// Open an existing project
    pub async fn open(path: &Path, project_type: ProjectType) -> Result<Self> {
        // Read project configuration based on type
        let config = match project_type {
            ProjectType::Unity => Self::read_unity_config(path).await?,
            ProjectType::Unreal => Self::read_unreal_config(path).await?,
            ProjectType::Godot => Self::read_godot_config(path).await?,
            ProjectType::Custom => Self::read_custom_config(path).await?,
        };

        // Scan assets, scenes, scripts
        let assets = Self::scan_assets(path).await?;
        let scenes = Self::scan_scenes(path, project_type).await?;
        let scripts = Self::scan_scripts(path).await?;

        Ok(Self {
            name: config.name.clone(),
            path: path.to_path_buf(),
            engine: project_type.into(),
            engine_version: None,
            created_at: Utc::now(),
            last_opened: Utc::now(),
            config,
            assets,
            scenes,
            scripts,
        })
    }

    /// Initialize a new project
    pub async fn init(path: PathBuf, engine: EngineType) -> Result<Self> {
        // Create project structure based on engine
        match engine {
            EngineType::Unity => Self::init_unity(&path).await?,
            EngineType::Unreal => Self::init_unreal(&path).await?,
            EngineType::Godot => Self::init_godot(&path).await?,
            EngineType::Custom(name) => Self::init_custom(&path, &name).await?,
        }

        let config = ProjectConfig {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            version: "1.0.0".to_string(),
            company: None,
            description: None,
            settings: HashMap::new(),
        };

        Ok(Self {
            name: config.name.clone(),
            path,
            engine,
            engine_version: None,
            created_at: Utc::now(),
            last_opened: Utc::now(),
            config,
            assets: Vec::new(),
            scenes: Vec::new(),
            scripts: Vec::new(),
        })
    }

    /// Read Unity project configuration
    async fn read_unity_config(path: &Path) -> Result<ProjectConfig> {
        let project_settings = path.join("ProjectSettings").join("ProjectSettings.asset");
        if !project_settings.exists() {
            return Err(anyhow!("Invalid Unity project"));
        }

        // Parse Unity project settings
        let content = tokio::fs::read_to_string(project_settings).await?;
        
        // Simple parsing - in production, use proper YAML parser
        let mut config = ProjectConfig {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            version: "1.0.0".to_string(),
            company: None,
            description: None,
            settings: HashMap::new(),
        };

        for line in content.lines() {
            if line.starts_with("  bundleVersion:") {
                config.version = line.split(':').nth(1).unwrap_or("1.0.0").trim().to_string();
            }
            if line.starts_with("  companyName:") {
                config.company = line.split(':').nth(1).map(|s| s.trim().to_string());
            }
            if line.starts_with("  productName:") {
                config.name = line.split(':').nth(1).unwrap_or("").trim().to_string();
            }
        }

        Ok(config)
    }

    /// Read Unreal project configuration
    async fn read_unreal_config(path: &Path) -> Result<ProjectConfig> {
        let uproject = std::fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .find(|e| e.path().extension().and_then(|e| e.to_str()) == Some("uproject"))
            .ok_or_else(|| anyhow!("No .uproject file found"))?;

        let content = tokio::fs::read_to_string(uproject.path()).await?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        Ok(ProjectConfig {
            name: uproject.path().file_stem().unwrap().to_string_lossy().to_string(),
            version: json.get("EngineAssociation")
                .and_then(|v| v.as_str())
                .unwrap_or("1.0.0")
                .to_string(),
            company: None,
            description: json.get("Description").and_then(|v| v.as_str()).map(|s| s.to_string()),
            settings: json.as_object()
                .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default(),
        })
    }

    /// Read Godot project configuration
    async fn read_godot_config(path: &Path) -> Result<ProjectConfig> {
        let godot_config = path.join("project.godot");
        if !godot_config.exists() {
            return Err(anyhow!("Invalid Godot project"));
        }

        let content = tokio::fs::read_to_string(godot_config).await?;
        let mut config = ProjectConfig {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            version: "1.0.0".to_string(),
            company: None,
            description: None,
            settings: HashMap::new(),
        };

        for line in content.lines() {
            if line.starts_with("config/name=") {
                config.name = line.split('=').nth(1).unwrap_or("").trim_matches('"').to_string();
            }
            if line.starts_with("config/version=") {
                config.version = line.split('=').nth(1).unwrap_or("1.0.0").trim_matches('"').to_string();
            }
            if line.starts_with("config/description=") {
                config.description = line.split('=').nth(1).map(|s| s.trim_matches('"').to_string());
            }
        }

        Ok(config)
    }

    /// Read custom project configuration
    async fn read_custom_config(path: &Path) -> Result<ProjectConfig> {
        // Look for .parsec-project file
        let config_file = path.join(".parsec-project");
        if config_file.exists() {
            let content = tokio::fs::read_to_string(config_file).await?;
            let config: ProjectConfig = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            // Default config
            Ok(ProjectConfig {
                name: path.file_name().unwrap().to_string_lossy().to_string(),
                version: "1.0.0".to_string(),
                company: None,
                description: None,
                settings: HashMap::new(),
            })
        }
    }

    /// Initialize Unity project
    async fn init_unity(path: &Path) -> Result<()> {
        tokio::fs::create_dir_all(path.join("Assets")).await?;
        tokio::fs::create_dir_all(path.join("ProjectSettings")).await?;
        tokio::fs::create_dir_all(path.join("Packages")).await?;

        // Create basic Unity project settings
        let manifest = r#"{
  "dependencies": {
    "com.unity.ide.visualstudio": "2.0.22",
    "com.unity.ide.rider": "3.0.31"
  }
}"#;

        tokio::fs::write(path.join("Packages/manifest.json"), manifest).await?;

        Ok(())
    }

    /// Initialize Unreal project
    async fn init_unreal(path: &Path) -> Result<()> {
        let project_name = path.file_name().unwrap().to_string_lossy();
        
        // Create basic Unreal project file
        let uproject = format!(r#"{{
  "FileVersion": 3,
  "EngineAssociation": "5.3",
  "Category": "",
  "Description": "",
  "Modules": [
    {{
      "Name": "{}",
      "Type": "Runtime",
      "LoadingPhase": "Default"
    }}
  ]
}}"#, project_name);

        tokio::fs::write(path.join(format!("{}.uproject", project_name)), uproject).await?;

        // Create source directory
        tokio::fs::create_dir_all(path.join("Source")).await?;

        Ok(())
    }

    /// Initialize Godot project
    async fn init_godot(path: &Path) -> Result<()> {
        let project_name = path.file_name().unwrap().to_string_lossy();
        
        // Create project.godot file
        let godot_config = format!(r#"; Engine configuration file.
; Designed to be edited only by Godot itself.

[application]
config/name="{}"
config/description=""
run/main_scene=""

[rendering]
renderer/backend="opengl3"
quality/driver/driver_name="GLES3"
"#, project_name);

        tokio::fs::write(path.join("project.godot"), godot_config).await?;

        // Create basic directories
        tokio::fs::create_dir_all(path.join("scenes")).await?;
        tokio::fs::create_dir_all(path.join("scripts")).await?;
        tokio::fs::create_dir_all(path.join("assets")).await?;

        Ok(())
    }

    /// Initialize custom project
    async fn init_custom(path: &Path, engine_name: &str) -> Result<()> {
        let config = ProjectConfig {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            version: "1.0.0".to_string(),
            company: None,
            description: Some(format!("{} project", engine_name)),
            settings: HashMap::new(),
        };

        let content = serde_json::to_string_pretty(&config)?;
        tokio::fs::write(path.join(".parsec-project"), content).await?;

        Ok(())
    }

    /// Scan assets in project
    async fn scan_assets(path: &Path) -> Result<Vec<AssetInfo>> {
        let mut assets = Vec::new();
        let mut stack = vec![path.to_path_buf()];

        while let Some(dir) = stack.pop() {
            let mut read_dir = tokio::fs::read_dir(&dir).await?;
            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    if let Ok(metadata) = entry.metadata().await {
                        let asset_type = path.extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_string();

                        assets.push(AssetInfo {
                            path,
                            asset_type,
                            size: metadata.len(),
                            modified: metadata.modified()
                                .map(|t| DateTime::from(t))
                                .unwrap_or_else(|_| Utc::now()),
                        });
                    }
                }
            }
        }

        Ok(assets)
    }

    /// Scan scenes in project
    async fn scan_scenes(path: &Path, project_type: ProjectType) -> Result<Vec<SceneInfo>> {
        let mut scenes = Vec::new();
        let extensions = match project_type {
            ProjectType::Unity => vec!["unity"],
            ProjectType::Unreal => vec!["umap"],
            ProjectType::Godot => vec!["tscn", "scn"],
            ProjectType::Custom => vec![],
        };

        let mut stack = vec![path.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let mut read_dir = tokio::fs::read_dir(&dir).await?;
            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if extensions.contains(&ext) {
                            scenes.push(SceneInfo {
                                name: path.file_stem().unwrap().to_string_lossy().to_string(),
                                path,
                                scene_type: ext.to_string(),
                                objects: 0, // Would need actual parsing
                            });
                        }
                    }
                }
            }
        }

        Ok(scenes)
    }

    /// Scan scripts in project
    async fn scan_scripts(path: &Path) -> Result<Vec<ScriptInfo>> {
        let mut scripts = Vec::new();
        let mut stack = vec![path.to_path_buf()];

        while let Some(dir) = stack.pop() {
            let mut read_dir = tokio::fs::read_dir(&dir).await?;
            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let language = match ext {
                            "cs" => "csharp",
                            "cpp" | "h" | "hpp" => "cpp",
                            "gd" => "gdscript",
                            "py" => "python",
                            "js" => "javascript",
                            "ts" => "typescript",
                            _ => continue,
                        };

                        let content = tokio::fs::read_to_string(&path).await?;
                        scripts.push(ScriptInfo {
                            name: path.file_stem().unwrap().to_string_lossy().to_string(),
                            path,
                            language: language.to_string(),
                            lines: content.lines().count(),
                        });
                    }
                }
            }
        }

        Ok(scripts)
    }

    /// Save project metadata
    pub fn save_metadata(&self, projects_dir: &Path) -> Result<()> {
        let metadata_path = projects_dir.join(format!("{}.json", self.name));
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(metadata_path, content)?;
        Ok(())
    }

    /// Get project name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get project path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get engine type
    pub fn engine_type(&self) -> EngineType {
        self.engine
    }

    /// Get project configuration
    pub fn config(&self) -> &ProjectConfig {
        &self.config
    }

    /// Get project assets
    pub fn assets(&self) -> &[AssetInfo] {
        &self.assets
    }

    /// Get project scenes
    pub fn scenes(&self) -> &[SceneInfo] {
        &self.scenes
    }

    /// Get project scripts
    pub fn scripts(&self) -> &[ScriptInfo] {
        &self.scripts
    }
}

impl From<ProjectType> for EngineType {
    fn from(pt: ProjectType) -> Self {
        match pt {
            ProjectType::Unity => EngineType::Unity,
            ProjectType::Unreal => EngineType::Unreal,
            ProjectType::Godot => EngineType::Godot,
            ProjectType::Custom => EngineType::Custom("custom".to_string()),
        }
    }
}