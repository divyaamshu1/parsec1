//! Sidebar component with file explorer and extensions

use std::path::{Path, PathBuf};
use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use tauri::Window;
use tauri::Emitter;
use notify::{RecursiveMode, RecommendedWatcher};

use parsec_core::git::FileStatus;

/// Sidebar tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SidebarTab {
    Explorer,
    Search,
    SourceControl,
    Extensions,
    Debug,
}

/// File tree item
#[derive(Debug, Clone, Serialize)]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub expanded: bool,
    pub children: Option<Vec<FileTreeNode>>,
    pub icon: String,
    pub git_status: Option<GitStatus>,
    pub depth: usize,
}

/// Git status for file
#[derive(Debug, Clone, Serialize)]
pub struct GitStatus {
    pub status: String,
    pub color: String,
    pub tooltip: String,
}

/// Extension item
#[derive(Debug, Clone, Serialize)]
pub struct ExtensionItem {
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub installed: bool,
    pub icon: Option<String>,
    pub has_update: bool,
}

/// Sidebar state
pub struct Sidebar {
    window: Window,
    current_tab: SidebarTab,
    root_path: Option<PathBuf>,
    expanded_folders: HashMap<String, bool>,
    watcher: Option<RecommendedWatcher>,
}

impl Sidebar {
    pub fn new(window: Window) -> Self {
        Self {
            window,
            current_tab: SidebarTab::Explorer,
            root_path: None,
            expanded_folders: HashMap::new(),
            watcher: None,
        }
    }

    /// Set workspace root
    pub async fn set_root(&mut self, path: PathBuf) -> Result<(), String> {
        self.root_path = Some(path.clone());
        self.refresh_file_tree().await?;
        self.setup_file_watcher(path).await?;
        Ok(())
    }

    /// Refresh file tree
    pub async fn refresh_file_tree(&self) -> Result<(), String> {
        if let Some(root) = &self.root_path {
            let tree = self.build_file_tree(root, 0);
            self.window.emit("sidebar:files", &tree)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Build file tree recursively
    fn build_file_tree(&self, path: &Path, depth: usize) -> Vec<FileTreeNode> {
        if depth > 10 {
            return Vec::new(); // Limit recursion depth
        }

        let mut items = Vec::new();

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files and common ignore patterns
                if name.starts_with('.') && name != ".git" {
                    continue;
                }
                if name == "node_modules" || name == "target" || name == "dist" {
                    continue;
                }

                let is_dir = path.is_dir();
                let path_str = path.to_string_lossy().to_string();
                let expanded = self.expanded_folders.get(&path_str).copied().unwrap_or(false);

                items.push(FileTreeNode {
                    name,
                    path: path_str,
                    is_dir,
                    expanded,
                    children: if is_dir && expanded {
                        Some(self.build_file_tree(&path, depth + 1))
                    } else {
                        None
                    },
                    icon: self.get_file_icon(&path),
                    git_status: None, // Would come from GitManager
                    depth,
                });
            }
        }

        // Sort: directories first, then files alphabetically
        items.sort_by(|a, b| {
            if a.is_dir && !b.is_dir {
                std::cmp::Ordering::Less
            } else if !a.is_dir && b.is_dir {
                std::cmp::Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        });

        items
    }

    /// Get icon for file
    fn get_file_icon(&self, path: &Path) -> String {
        if path.is_dir() {
            return "📁".to_string();
        }

        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => "🦀".to_string(),
            Some("py") => "🐍".to_string(),
            Some("js") => "📜".to_string(),
            Some("jsx") => "⚛️".to_string(),
            Some("ts") => "📘".to_string(),
            Some("tsx") => "⚛️".to_string(),
            Some("html") => "🌐".to_string(),
            Some("css") => "🎨".to_string(),
            Some("scss") => "🎨".to_string(),
            Some("json") => "📋".to_string(),
            Some("toml") => "⚙️".to_string(),
            Some("yaml") | Some("yml") => "⚙️".to_string(),
            Some("md") => "📝".to_string(),
            Some("txt") => "📄".to_string(),
            Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg") => "🖼️".to_string(),
            Some("mp3") | Some("wav") | Some("ogg") => "🎵".to_string(),
            Some("mp4") | Some("avi") | Some("mov") => "🎬".to_string(),
            Some("pdf") => "📕".to_string(),
            Some("zip") | Some("tar") | Some("gz") | Some("7z") => "📦".to_string(),
            Some("exe") | Some("msi") => "⚙️".to_string(),
            Some("sh") | Some("bash") => "🐚".to_string(),
            _ => "📄".to_string(),
        }
    }

    /// Setup file watcher for auto-refresh
    async fn setup_file_watcher(&mut self, path: PathBuf) -> Result<(), String> {
        use notify::{Watcher, RecursiveMode};

        let window = self.window.clone();
        let watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                    let _ = window.emit("sidebar:refresh", ());
                }
            }
        }).map_err(|e| e.to_string())?;

        let mut watcher = watcher;
        watcher.watch(&path, RecursiveMode::Recursive)
            .map_err(|e| e.to_string())?;

        self.watcher = Some(watcher);
        Ok(())
    }

    /// Toggle folder expansion
    pub async fn toggle_folder(&mut self, path: String) -> Result<(), String> {
        let expanded = self.expanded_folders.get(&path).copied().unwrap_or(false);
        self.expanded_folders.insert(path, !expanded);
        self.refresh_file_tree().await?;
        Ok(())
    }

    /// Switch tab
    pub async fn switch_tab(&mut self, tab: SidebarTab) -> Result<(), String> {
        self.current_tab = tab;
        self.window.emit("sidebar:tab", &tab)
            .map_err(|e| e.to_string())
    }

    /// Get current tab
    pub fn current_tab(&self) -> SidebarTab {
        self.current_tab
    }
}

/// Sidebar commands
#[tauri::command]
pub async fn sidebar_get_files(_window: Window) -> Result<Vec<FileTreeNode>, String> {
    // Placeholder implementation for compile-time: returns empty tree
    Ok(Vec::new())
}

#[tauri::command]
pub async fn sidebar_toggle_folder(_path: String, _window: Window) -> Result<(), String> {
    // Placeholder: no-op for compile
    Ok(())
}

#[tauri::command]
pub async fn sidebar_open_file(_path: String) -> Result<(), String> {
    // Placeholder: core open_file not available in this compile-pass
    Ok(())
}

#[tauri::command]
pub async fn sidebar_get_extensions() -> Result<Vec<ExtensionItem>, String> {
    // Placeholder: return empty extension list for compile
    Ok(Vec::new())
}

#[tauri::command]
pub async fn sidebar_install_extension(_id: String) -> Result<(), String> {
    // Placeholder
    Ok(())
}

#[tauri::command]
pub async fn sidebar_enable_extension(_id: String) -> Result<(), String> {
    // Placeholder
    Ok(())
}

#[tauri::command]
pub async fn sidebar_disable_extension(_id: String) -> Result<(), String> {
    // Placeholder
    Ok(())
}