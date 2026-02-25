//! VS Code Extension Compatibility Layer
//!
//! This crate provides a compatibility layer for running VS Code extensions
//! in Parsec IDE. It implements the vscode.* API and handles extension
//! loading from the VS Code marketplace.
#![allow(dead_code, unused_imports, unused_variables, unused_mut, ambiguous_glob_reexports, mismatched_lifetime_syntaxes)]

pub mod api;
pub mod runtime;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use tokio::sync::{RwLock, mpsc};
use tracing::{info, warn, error};
use serde::{Serialize, Deserialize};

use parsec_core::editor::Editor;
use parsec_core::terminal::{Terminal, TerminalConfig};
use parsec_core::git::{GitManager, GitConfig};
use crate::runtime::Runtime;

// Use extension-registry for marketplace
use extension_registry::sources::vsx::VSCodeMarketplace;
use extension_registry::types::{ExtensionInfo, SearchQuery, SearchResult};

/// Main VS Code compatibility manager
pub struct VSCodeCompat {
    /// Loaded extensions
    extensions: Arc<RwLock<HashMap<String, LoadedExtension>>>,
    /// JavaScript runtime for pure JS extensions
    js_runtime: Option<runtime::JSRuntime>,
    /// Node.js runtime for extensions that need Node
    node_runtime: Option<runtime::NodeRuntime>,
    /// Marketplace client
    marketplace: VSCodeMarketplace,
    /// API implementation
    api: Arc<api::VSCodeAPI>,
    /// Configuration
    config: VSCodeConfig,
    /// Event sender
    event_tx: mpsc::UnboundedSender<VSCodeEvent>,
}

/// VS Code compatibility configuration
#[derive(Debug, Clone)]
pub struct VSCodeConfig {
    pub extensions_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub enable_js_runtime: bool,
    pub enable_node_runtime: bool,
    pub enable_marketplace: bool,
    pub marketplace_url: String,
    pub api_key: Option<String>,
    pub verify_signatures: bool,
}

impl Default for VSCodeConfig {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("parsec");
        
        Self {
            extensions_dir: data_dir.join("vscode-extensions"),
            cache_dir: data_dir.join("vscode-cache"),
            enable_js_runtime: true,
            enable_node_runtime: true,
            enable_marketplace: true,
            marketplace_url: "https://marketplace.visualstudio.com/_apis/public/gallery".to_string(),
            api_key: None,
            verify_signatures: false,
        }
    }
}

/// Loaded extension information
#[derive(Debug, Clone)]
pub struct LoadedExtension {
    pub id: String,
    pub publisher: String,
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub main: Option<String>,
    pub browser: Option<String>,
    pub extension_kind: Vec<String>,
    pub activation_events: Vec<String>,
    pub contributes: serde_json::Value,
    pub runtime: ExtensionRuntime,
    pub enabled: bool,
    pub activation_time: Option<std::time::Instant>,
}

/// Extension runtime type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionRuntime {
    PureJS,
    NodeJS,
    WebAssembly,
    Unknown,
}

/// VS Code compatibility events
#[derive(Debug, Clone)]
pub enum VSCodeEvent {
    ExtensionLoaded(String),
    ExtensionActivated(String),
    ExtensionFailed(String, String),
    ExtensionUnloaded(String),
    MarketplaceSearchComplete(Vec<ExtensionInfo>),
    Error(String),
}

impl VSCodeCompat {
    /// Create a new VS Code compatibility manager
    pub async fn new(config: VSCodeConfig, editor: Arc<Editor>, terminal: Arc<Terminal>, git: Arc<GitManager>) -> Result<Self> {
        // Create directories
        tokio::fs::create_dir_all(&config.extensions_dir).await?;
        tokio::fs::create_dir_all(&config.cache_dir).await?;

        let (event_tx, _) = mpsc::unbounded_channel();

        // Initialize JS runtime if enabled
        let js_runtime = if config.enable_js_runtime {
            match runtime::JSRuntime::new() {
                Ok(runtime) => Some(runtime),
                Err(e) => {
                    warn!("Failed to initialize JS runtime: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Initialize Node runtime if enabled
        let node_runtime = if config.enable_node_runtime {
            match runtime::NodeRuntime::new() {
                Ok(runtime) => Some(runtime),
                Err(e) => {
                    warn!("Failed to initialize Node runtime: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Create API
        let api = Arc::new(api::VSCodeAPI::new(editor, terminal, git));

        Ok(Self {
            extensions: Arc::new(RwLock::new(HashMap::new())),
            js_runtime,
            node_runtime,
            marketplace: VSCodeMarketplace::new(config.marketplace_url.clone(), config.api_key.clone()),
            api,
            config,
            event_tx,
        })
    }

    /// Install an extension from VS Code marketplace
    pub async fn install_extension(&self, id: &str, version: Option<&str>) -> Result<LoadedExtension> {
        info!("Installing extension: {}", id);

        // Check if already installed
        if self.extensions.read().await.contains_key(id) {
            return Err(anyhow!("Extension already installed: {}", id));
        }

        // Get extension info from marketplace (id is expected as "publisher.name")
        let parts: Vec<&str> = id.split('.').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid extension id, expected 'publisher.name'"));
        }
        let publisher = parts[0];
        let name = parts[1];
        let info = self.marketplace.get_extension(publisher, name).await?;

        // Download VSIX
        let vsix_path = self.download_extension(&info).await?;

        // Extract VSIX
        let ext_dir = self.extract_vsix(&vsix_path, &info).await?;

        // Read package.json
        let package_json = self.read_package_json(&ext_dir).await?;

        // Detect runtime type
        let runtime = self.detect_runtime(&package_json);

        // Create loaded extension
        let extension = LoadedExtension {
            id: id.to_string(),
            publisher: info.publisher.clone(),
            name: info.name.clone(),
            version: info.version,
            path: ext_dir,
            main: package_json.get("main").and_then(|v| v.as_str()).map(|s| s.to_string()),
            browser: package_json.get("browser").and_then(|v| v.as_str()).map(|s| s.to_string()),
            extension_kind: package_json
                .get("extensionKind")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            activation_events: package_json
                .get("activationEvents")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            contributes: package_json.get("contributes").cloned().unwrap_or(serde_json::Value::Null),
            runtime,
            enabled: true,
            activation_time: None,
        };

        // Store extension
        self.extensions.write().await.insert(id.to_string(), extension.clone());

        // Send event
        self.event_tx.send(VSCodeEvent::ExtensionLoaded(id.to_string())).ok();

        Ok(extension)
    }

    /// Activate an extension
    pub async fn activate_extension(&self, id: &str) -> Result<()> {
        let extensions = self.extensions.read().await;
        let extension = extensions.get(id).ok_or_else(|| anyhow!("Extension not found: {}", id))?;

        if !extension.enabled {
            return Err(anyhow!("Extension is disabled: {}", id));
        }

        info!("Activating extension: {}", id);

        match extension.runtime {
            ExtensionRuntime::PureJS => {
                if let Some(runtime) = &self.js_runtime {
                    runtime.execute_extension(extension)?;
                } else {
                    return Err(anyhow!("JS runtime not available"));
                }
            }
            ExtensionRuntime::NodeJS => {
                if let Some(runtime) = &self.node_runtime {
                    runtime.execute_extension(extension)?;
                } else {
                    return Err(anyhow!("Node runtime not available"));
                }
            }
            ExtensionRuntime::WebAssembly => {
                return Err(anyhow!("WebAssembly extensions not yet supported"));
            }
            ExtensionRuntime::Unknown => {
                return Err(anyhow!("Unknown extension type"));
            }
        }

        // Update activation time
        drop(extensions);
        let mut extensions = self.extensions.write().await;
        if let Some(ext) = extensions.get_mut(id) {
            ext.activation_time = Some(std::time::Instant::now());
        }

        self.event_tx.send(VSCodeEvent::ExtensionActivated(id.to_string())).ok();

        Ok(())
    }

    /// Download extension from marketplace
    async fn download_extension(&self, info: &ExtensionInfo) -> Result<PathBuf> {
        let url = format!(
            "{}/publisher/{}/vsextensions/{}/{}/vspackage",
            self.config.marketplace_url, info.publisher, info.name, info.version
        );

        let response = reqwest::get(&url).await?;
        if !response.status().is_success() {
            return Err(anyhow!("Failed to download extension: HTTP {}", response.status()));
        }

        let filename = format!("{}.{}.vsix", info.name, info.version);
        let path = self.config.cache_dir.join(filename);

        let bytes = response.bytes().await?;
        tokio::fs::write(&path, bytes).await?;

        Ok(path)
    }

    /// Extract VSIX file
    async fn extract_vsix(&self, vsix_path: &Path, info: &ExtensionInfo) -> Result<PathBuf> {
        let ext_dir = self.config.extensions_dir.join(&info.publisher).join(&info.name).join(&info.version);
        tokio::fs::create_dir_all(&ext_dir).await?;

        let file = tokio::fs::File::open(vsix_path).await?;
        let mut archive = zip::ZipArchive::new(file.into_std().await)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = ext_dir.join(file.name());

            if file.name().ends_with('/') {
                tokio::fs::create_dir_all(&outpath).await?;
            } else {
                if let Some(parent) = outpath.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                let mut outfile = tokio::fs::File::create(&outpath).await?;
                std::io::copy(&mut file, &mut outfile.into_std().await)?;
            }
        }

        Ok(ext_dir)
    }

    /// Read package.json from extension directory
    async fn read_package_json(&self, ext_dir: &Path) -> Result<serde_json::Value> {
        let package_path = ext_dir.join("extension").join("package.json");
        if !package_path.exists() {
            return Err(anyhow!("package.json not found in extension"));
        }

        let content = tokio::fs::read_to_string(package_path).await?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        Ok(json)
    }

    /// Detect runtime type from package.json
    fn detect_runtime(&self, package_json: &serde_json::Value) -> ExtensionRuntime {
        // Check for native modules
        if let Some(dependencies) = package_json.get("dependencies").and_then(|d| d.as_object()) {
            for (name, _) in dependencies {
                if name.starts_with("node-") || name == "fs" || name == "path" || name == "child_process" {
                    return ExtensionRuntime::NodeJS;
                }
            }
        }

        // Check for browser entry point
        if package_json.get("browser").is_some() {
            return ExtensionRuntime::PureJS;
        }

        // Check for main entry point
        if let Some(main) = package_json.get("main").and_then(|m| m.as_str()) {
            if main.ends_with(".wasm") {
                return ExtensionRuntime::WebAssembly;
            }
        }

        ExtensionRuntime::PureJS
    }

    /// Uninstall an extension
    pub async fn uninstall_extension(&self, id: &str) -> Result<()> {
        let mut extensions = self.extensions.write().await;
        if let Some(extension) = extensions.remove(id) {
            // Delete extension directory
            if extension.path.exists() {
                tokio::fs::remove_dir_all(&extension.path).await?;
            }
            self.event_tx.send(VSCodeEvent::ExtensionUnloaded(id.to_string())).ok();
        }
        Ok(())
    }

    /// Enable an extension
    pub async fn enable_extension(&self, id: &str) -> Result<()> {
        let mut extensions = self.extensions.write().await;
        if let Some(extension) = extensions.get_mut(id) {
            extension.enabled = true;
        }
        Ok(())
    }

    /// Disable an extension
    pub async fn disable_extension(&self, id: &str) -> Result<()> {
        let mut extensions = self.extensions.write().await;
        if let Some(extension) = extensions.get_mut(id) {
            extension.enabled = false;
        }
        Ok(())
    }

    /// List installed extensions
    pub async fn list_extensions(&self) -> Vec<LoadedExtension> {
        self.extensions.read().await.values().cloned().collect()
    }

    /// Get extension by ID
    pub async fn get_extension(&self, id: &str) -> Option<LoadedExtension> {
        self.extensions.read().await.get(id).cloned()
    }

    /// Search marketplace for extensions
    pub async fn search_marketplace(&self, query: &str) -> Result<Vec<ExtensionInfo>> {
        if !self.config.enable_marketplace {
            return Err(anyhow!("Marketplace access is disabled"));
        }

        // Build a SearchQuery from the provided text
        let search_query = SearchQuery { 
            text: query.to_string(), 
            page: 1,
            page_size: 50,
            ..Default::default() 
        };
        
        let results = self.marketplace.search(search_query).await?;
        
        // Notify subscribers with the extensions vector
        self.event_tx.send(VSCodeEvent::MarketplaceSearchComplete(results.extensions.clone())).ok();
        
        Ok(results.extensions)
    }

    /// Get API instance (for extensions)
    pub fn api(&self) -> Arc<api::VSCodeAPI> {
        self.api.clone()
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> mpsc::UnboundedReceiver<VSCodeEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        // Forward events from main channel to this subscriber
        // This is simplified - in production, use broadcast
        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_vscode_compat_creation() -> Result<()> {
        let dir = tempdir()?;
        let config = VSCodeConfig {
            extensions_dir: dir.path().to_path_buf(),
            cache_dir: dir.path().join("cache"),
            ..Default::default()
        };

        let editor = Arc::new(Editor::new());
        let terminal = Arc::new(Terminal::new(
            "test-term".to_string(),
            "Test Terminal".to_string(),
            TerminalConfig::default(),
        ));
        let git = Arc::new(GitManager::new(GitConfig::default()));

        let compat = VSCodeCompat::new(config, editor, terminal, git).await?;
        assert!(compat.list_extensions().await.is_empty());

        Ok(())
    }
}