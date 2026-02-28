//! Workspace layouts management

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::fs;
use serde::{Serialize, Deserialize};

use crate::{Result, CustomizationError, CustomizationConfig};

/// Split direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// Panel location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelLocation {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

/// Editor layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorLayout {
    pub split_direction: SplitDirection,
    pub split_sizes: Vec<f32>,
    pub active_editor: Option<usize>,
    pub editors: Vec<EditorInfo>,
}

/// Editor info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorInfo {
    pub path: Option<String>,
    pub language: Option<String>,
    pub modifications: bool,
    pub view_state: Option<EditorViewState>,
}

/// Editor view state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorViewState {
    pub scroll_x: u32,
    pub scroll_y: u32,
    pub cursor_line: usize,
    pub cursor_column: usize,
    pub selections: Vec<SelectionRange>,
    pub folding: Vec<FoldingRange>,
}

/// Selection range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionRange {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

/// Folding range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoldingRange {
    pub start_line: usize,
    pub end_line: usize,
    pub collapsed: bool,
}

/// Panel layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelLayout {
    pub location: PanelLocation,
    pub size: u32,
    pub visible: bool,
    pub active_tab: Option<String>,
    pub tabs: Vec<PanelTab>,
}

/// Panel tab
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelTab {
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
    pub content_type: String,
}

/// Workspace layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceLayout {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub author: Option<String>,
    pub editor_layout: EditorLayout,
    pub panels: Vec<PanelLayout>,
    pub sidebar_visible: bool,
    pub sidebar_width: u32,
    pub terminal_visible: bool,
    pub terminal_height: u32,
    pub status_bar_visible: bool,
    pub menu_bar_visible: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub parent: Option<String>,
}

/// Layout profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutProfile {
    pub name: String,
    pub active_layout: String,
    pub overrides: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Layout manager
pub struct LayoutManager {
    /// Available layouts
    layouts: Arc<RwLock<HashMap<String, WorkspaceLayout>>>,
    /// Active layout name
    active_layout: Arc<RwLock<Option<String>>>,
    /// Layout profiles
    profiles: Arc<RwLock<HashMap<String, LayoutProfile>>>,
    /// Active profile
    active_profile: Arc<RwLock<String>>,
    /// Configuration
    config: CustomizationConfig,
}

impl LayoutManager {
    /// Create new layout manager
    pub async fn new(config: CustomizationConfig) -> Result<Self> {
        let manager = Self {
            layouts: Arc::new(RwLock::new(HashMap::new())),
            active_layout: Arc::new(RwLock::new(Some("default".to_string()))),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            active_profile: Arc::new(RwLock::new("default".to_string())),
            config: config.clone(),
        };

        // Load default layout
        manager.load_default_layout().await?;

        // Scan layouts directory
        manager.scan_layouts().await?;

        Ok(manager)
    }

    /// Load default layout
    async fn load_default_layout(&self) -> Result<()> {
        let layout = WorkspaceLayout {
            name: "default".to_string(),
            description: Some("Default workspace layout".to_string()),
            version: env!("CARGO_PKG_VERSION").to_string(),
            author: Some("Parsec Team".to_string()),
            editor_layout: EditorLayout {
                split_direction: SplitDirection::Horizontal,
                split_sizes: vec![1.0],
                active_editor: Some(0),
                editors: Vec::new(),
            },
            panels: vec![
                PanelLayout {
                    location: PanelLocation::Left,
                    size: 250,
                    visible: true,
                    active_tab: Some("explorer".to_string()),
                    tabs: vec![
                        PanelTab {
                            id: "explorer".to_string(),
                            title: "Explorer".to_string(),
                            icon: Some("folder".to_string()),
                            content_type: "explorer".to_string(),
                        },
                        PanelTab {
                            id: "search".to_string(),
                            title: "Search".to_string(),
                            icon: Some("search".to_string()),
                            content_type: "search".to_string(),
                        },
                        PanelTab {
                            id: "source-control".to_string(),
                            title: "Source Control".to_string(),
                            icon: Some("git".to_string()),
                            content_type: "source-control".to_string(),
                        },
                        PanelTab {
                            id: "extensions".to_string(),
                            title: "Extensions".to_string(),
                            icon: Some("extensions".to_string()),
                            content_type: "extensions".to_string(),
                        },
                    ],
                },
                PanelLayout {
                    location: PanelLocation::Bottom,
                    size: 200,
                    visible: true,
                    active_tab: Some("terminal".to_string()),
                    tabs: vec![
                        PanelTab {
                            id: "terminal".to_string(),
                            title: "Terminal".to_string(),
                            icon: Some("terminal".to_string()),
                            content_type: "terminal".to_string(),
                        },
                        PanelTab {
                            id: "problems".to_string(),
                            title: "Problems".to_string(),
                            icon: Some("warning".to_string()),
                            content_type: "problems".to_string(),
                        },
                        PanelTab {
                            id: "output".to_string(),
                            title: "Output".to_string(),
                            icon: Some("output".to_string()),
                            content_type: "output".to_string(),
                        },
                        PanelTab {
                            id: "debug".to_string(),
                            title: "Debug".to_string(),
                            icon: Some("debug".to_string()),
                            content_type: "debug".to_string(),
                        },
                    ],
                },
            ],
            sidebar_visible: true,
            sidebar_width: 250,
            terminal_visible: true,
            terminal_height: 200,
            status_bar_visible: true,
            menu_bar_visible: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            parent: None,
        };

        self.layouts.write().await.insert("default".to_string(), layout);
        Ok(())
    }

    /// Scan layouts directory
    async fn scan_layouts(&self) -> Result<()> {
        let mut read_dir = fs::read_dir(&self.config.layouts_dir).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(layout) = serde_json::from_str::<WorkspaceLayout>(&content) {
                        self.layouts.write().await.insert(layout.name.clone(), layout);
                    }
                }
            }
        }

        Ok(())
    }

    /// Add layout
    pub async fn add_layout(&self, layout: WorkspaceLayout) -> Result<()> {
        let name = layout.name.clone();
        self.layouts.write().await.insert(name.clone(), layout);
        
        // Save to disk
        let path = self.config.layouts_dir.join(format!("{}.json", name));
        let json = serde_json::to_string_pretty(&self.layouts.read().await.get(&name).unwrap())?;
        fs::write(path, json).await?;

        Ok(())
    }

    /// Remove layout
    pub async fn remove_layout(&self, name: &str) -> Result<()> {
        self.layouts.write().await.remove(name);
        
        let path = self.config.layouts_dir.join(format!("{}.json", name));
        if path.exists() {
            fs::remove_file(path).await?;
        }

        Ok(())
    }

    /// Set active layout
    pub async fn set_active_layout(&self, name: &str) -> Result<()> {
        if !self.layouts.read().await.contains_key(name) {
            return Err(CustomizationError::LayoutError(format!("Layout not found: {}", name)));
        }

        *self.active_layout.write().await = Some(name.to_string());
        Ok(())
    }

    /// Get active layout
    pub async fn active_layout(&self) -> Option<WorkspaceLayout> {
        let name = self.active_layout.read().await.clone()?;
        self.layouts.read().await.get(&name).cloned()
    }

    /// List layouts
    pub async fn list_layouts(&self) -> Vec<WorkspaceLayout> {
        self.layouts.read().await.values().cloned().collect()
    }

    /// Get layout by name
    pub async fn get_layout(&self, name: &str) -> Option<WorkspaceLayout> {
        self.layouts.read().await.get(name).cloned()
    }

    /// Create profile
    pub async fn create_profile(&self, name: &str, layout_name: &str) -> Result<LayoutProfile> {
        let profile = LayoutProfile {
            name: name.to_string(),
            active_layout: layout_name.to_string(),
            overrides: HashMap::new(),
            created_at: chrono::Utc::now(),
        };

        self.profiles.write().await.insert(name.to_string(), profile.clone());
        Ok(profile)
    }

    /// Set active profile
    pub async fn set_active_profile(&self, name: &str) -> Result<()> {
        let profiles = self.profiles.read().await;
        if let Some(profile) = profiles.get(name) {
            self.set_active_layout(&profile.active_layout).await?;
            *self.active_profile.write().await = name.to_string();
            Ok(())
        } else {
            Err(CustomizationError::LayoutError(format!("Profile not found: {}", name)))
        }
    }

    /// Override layout setting in profile
    pub async fn override_setting(&self, key: &str, value: serde_json::Value) -> Result<()> {
        let profile_name = self.active_profile.read().await.clone();
        let mut profiles = self.profiles.write().await;
        
        if let Some(profile) = profiles.get_mut(&profile_name) {
            profile.overrides.insert(key.to_string(), value);
        }

        Ok(())
    }

    /// Apply layout to workspace
    pub async fn apply_layout(&self, layout: &WorkspaceLayout) -> Result<()> {
        // This would communicate with the UI to apply the layout
        // For now, just log
        tracing::info!("Applying layout: {}", layout.name);
        Ok(())
    }

    /// Save current workspace as layout
    pub async fn save_current(&self, name: &str, description: Option<String>) -> Result<()> {
        // This would capture current workspace state
        // For now, create a default layout
        let layout = WorkspaceLayout {
            name: name.to_string(),
            description,
            version: env!("CARGO_PKG_VERSION").to_string(),
            author: Some("User".to_string()),
            editor_layout: EditorLayout {
                split_direction: SplitDirection::Horizontal,
                split_sizes: vec![1.0],
                active_editor: Some(0),
                editors: Vec::new(),
            },
            panels: vec![],
            sidebar_visible: true,
            sidebar_width: 250,
            terminal_visible: true,
            terminal_height: 200,
            status_bar_visible: true,
            menu_bar_visible: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            parent: None,
        };

        self.add_layout(layout).await?;
        Ok(())
    }

    /// Export layout
    pub async fn export_layout(&self, name: &str, format: &str) -> Result<String> {
        let layout = self.layouts.read().await.get(name)
            .ok_or_else(|| CustomizationError::LayoutError(format!("Layout not found: {}", name)))?
            .clone();

        match format {
            "json" => Ok(serde_json::to_string_pretty(&layout)?),
            "yaml" => Ok(serde_yaml::to_string(&layout)?),
            _ => Err(CustomizationError::LayoutError(format!("Unsupported format: {}", format))),
        }
    }

    /// Import layout
    pub async fn import_layout(&self, data: &str, format: &str) -> Result<()> {
        let layout = match format {
            "json" => serde_json::from_str(data)?,
            "yaml" => serde_yaml::from_str(data)?,
            _ => return Err(CustomizationError::LayoutError(format!("Unsupported format: {}", format))),
        };

        self.add_layout(layout).await?;
        Ok(())
    }
}