//! Project templates for game engines

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};

use crate::engine::EngineType;

/// Template manager
pub struct TemplateManager {
    templates_dir: PathBuf,
    templates: HashMap<String, ProjectTemplate>,
}

/// Project template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTemplate {
    pub name: String,
    pub description: String,
    pub engine: EngineType,
    pub version: String,
    pub author: Option<String>,
    pub icon: Option<PathBuf>,
    pub path: PathBuf,
}

impl TemplateManager {
    /// Create new template manager
    pub fn new() -> Result<Self> {
        let templates_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("parsec/game-templates");

        std::fs::create_dir_all(&templates_dir)?;

        Ok(Self {
            templates_dir,
            templates: HashMap::new(),
        })
    }

    /// Get template path
    pub fn get_template(&self, engine: EngineType, name: &str) -> Result<PathBuf> {
        let template_path = self.templates_dir
            .join(engine.as_str())
            .join(name);

        if template_path.exists() {
            Ok(template_path)
        } else {
            // Fallback to built-in template
            self.get_builtin_template(engine, name)
        }
    }

    /// Get built-in template
    fn get_builtin_template(&self, engine: EngineType, name: &str) -> Result<PathBuf> {
        let template_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("templates")
            .join(engine.as_str())
            .join(name);

        if template_path.exists() {
            Ok(template_path)
        } else {
            Err(anyhow!("Template not found: {} for {:?}", name, engine))
        }
    }

    /// Copy template to target directory
    pub fn copy_template(&self, source: &Path, target: &Path) -> Result<()> {
        if !source.exists() {
            return Err(anyhow!("Template source does not exist: {}", source.display()));
        }

        // Create target directory
        std::fs::create_dir_all(target)?;

        // Copy all files recursively
        self.copy_dir(source, target)?;

        Ok(())
    }

    /// Copy directory recursively
    fn copy_dir(&self, src: &Path, dst: &Path) -> Result<()> {
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let dest_path = dst.join(entry.file_name());

            if path.is_dir() {
                std::fs::create_dir_all(&dest_path)?;
                self.copy_dir(&path, &dest_path)?;
            } else {
                std::fs::copy(&path, &dest_path)?;
            }
        }

        Ok(())
    }

    /// List available templates
    pub fn list_templates(&self, engine: Option<EngineType>) -> Vec<TemplateInfo> {
        let mut templates = Vec::new();

        if let Some(engine) = engine {
            // List templates for specific engine
            let engine_dir = self.templates_dir.join(engine.as_str());
            if engine_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(engine_dir) {
                    for entry in entries.flatten() {
                        if let Some(info) = self.load_template_info(entry.path()) {
                            templates.push(info);
                        }
                    }
                }
            }
        } else {
            // List all templates
            if let Ok(entries) = std::fs::read_dir(&self.templates_dir) {
                for engine_entry in entries.flatten() {
                    let engine_path = engine_entry.path();
                    if engine_path.is_dir() {
                        if let Ok(template_entries) = std::fs::read_dir(engine_path) {
                            for template_entry in template_entries.flatten() {
                                if let Some(info) = self.load_template_info(template_entry.path()) {
                                    templates.push(info);
                                }
                            }
                        }
                    }
                }
            }
        }

        templates
    }

    /// Load template info from directory
    fn load_template_info(&self, path: PathBuf) -> Option<TemplateInfo> {
        let template_file = path.join("template.json");
        if template_file.exists() {
            if let Ok(content) = std::fs::read_to_string(template_file) {
                if let Ok(info) = serde_json::from_str::<TemplateInfo>(&content) {
                    return Some(info);
                }
            }
        }

        // Create default info
        Some(TemplateInfo {
            name: path.file_name()?.to_string_lossy().to_string(),
            description: "No description".to_string(),
            engine: EngineType::Custom("unknown".to_string()),
            version: "1.0.0".to_string(),
            icon: None,
        })
    }

    /// Install a template
    pub fn install_template(&self, source: &Path, engine: EngineType, name: &str) -> Result<()> {
        let target = self.templates_dir.join(engine.as_str()).join(name);
        self.copy_dir(source, &target)?;

        Ok(())
    }
}

/// Template information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub name: String,
    pub description: String,
    pub engine: EngineType,
    pub version: String,
    pub icon: Option<PathBuf>,
}