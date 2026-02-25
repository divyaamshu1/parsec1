//! Blueprint visual scripting support for Unreal Engine

mod viewer;
mod converter;

pub use viewer::*;
pub use converter::*;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};

/// Blueprint asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub name: String,
    pub path: PathBuf,
    pub parent_class: String,
    pub nodes: Vec<BlueprintNode>,
    pub graphs: Vec<BlueprintGraph>,
}

/// Blueprint node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintNode {
    pub id: usize,
    pub name: String,
    pub position: (f32, f32),
    pub pins: Vec<BlueprintPin>,
}

/// Blueprint pin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintPin {
    pub id: usize,
    pub name: String,
    pub direction: PinDirection,
    pub connected_to: Vec<usize>,
}

/// Pin direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PinDirection {
    Input,
    Output,
}

/// Blueprint graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintGraph {
    pub name: String,
    pub nodes: Vec<usize>,
}

/// Blueprint viewer
pub struct BlueprintViewer {
    blueprints: Arc<tokio::sync::Mutex<HashMap<String, Blueprint>>>,
}

impl BlueprintViewer {
    pub fn new() -> Self {
        Self {
            blueprints: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Load a blueprint file
    pub async fn load_blueprint(&self, path: &Path) -> Result<Blueprint> {
        // Parse Unreal blueprint file
        // This is simplified - real implementation would parse the actual binary format
        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let blueprint = Blueprint {
            name,
            path: path.to_path_buf(),
            parent_class: "Actor".to_string(),
            nodes: vec![],
            graphs: vec![],
        };

        self.blueprints.lock().await.insert(blueprint.name.clone(), blueprint.clone());

        Ok(blueprint)
    }

    /// Get blueprint by name
    pub async fn get_blueprint(&self, name: &str) -> Option<Blueprint> {
        self.blueprints.lock().await.get(name).cloned()
    }

    /// Render blueprint as SVG
    pub async fn render_as_svg(&self, name: &str) -> Result<String> {
        let blueprint = self.get_blueprint(name).await
            .ok_or_else(|| anyhow!("Blueprint not found: {}", name))?;

        // Generate SVG from blueprint graph
        let mut svg = String::new();
        svg.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 600">"#);

        // Render nodes
        for node in blueprint.nodes {
            svg.push_str(&format!(
                r#"<rect x="{}" y="{}" width="150" height="80" fill="#2d2d2d" stroke="#666" />"#,
                node.position.0, node.position.1
            ));
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" fill="#fff">{}</text>"#,
                node.position.0 + 10, node.position.1 + 25, node.name
            ));
        }

        svg.push_str("</svg>");
        Ok(svg)
    }
}

/// Blueprint converter
pub struct BlueprintConverter;

impl BlueprintConverter {
    /// Convert blueprint to JSON
    pub fn to_json(blueprint: &Blueprint) -> Result<String> {
        serde_json::to_string_pretty(blueprint).map_err(Into::into)
    }

    /// Convert blueprint to YAML
    pub fn to_yaml(blueprint: &Blueprint) -> Result<String> {
        serde_yaml::to_string(blueprint).map_err(Into::into)
    }

    /// Export blueprint as image
    pub async fn to_image(&self, blueprint: &Blueprint) -> Result<Vec<u8>> {
        // Would render to PNG
        Ok(Vec::new())
    }
}