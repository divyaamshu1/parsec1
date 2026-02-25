//! VS Code Workspace API Implementation
//!
//! Implements vscode.workspace.* API for workspace and file operations.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use tokio::sync::{RwLock, watch};
use serde_json::Value;

use crate::api::{Uri, TextDocument, WorkspaceEdit, Disposable};
use parsec_core::editor::Editor;

/// Workspace API implementation
pub struct WorkspaceAPI {
    /// Workspace folders
    folders: Arc<RwLock<Vec<WorkspaceFolder>>>,
    /// Text documents
    documents: Arc<RwLock<HashMap<String, TextDocument>>>,
    /// File system watcher
    watcher: Option<notify::RecommendedWatcher>,
    /// Configuration
    configuration: Arc<RwLock<Configuration>>,
    /// Editor reference
    editor: Arc<Editor>,
    /// Change event sender
    change_tx: watch::Sender<WorkspaceChangeEvent>,
    /// Change event receiver
    change_rx: watch::Receiver<WorkspaceChangeEvent>,
}

/// Workspace folder
#[derive(Debug, Clone)]
pub struct WorkspaceFolder {
    pub uri: Uri,
    pub name: String,
    pub index: usize,
}

/// Configuration (workspace and user settings)
#[derive(Debug, Clone, Default)]
pub struct Configuration {
    pub user: HashMap<String, Value>,
    pub workspace: HashMap<String, Value>,
    pub folder: HashMap<String, HashMap<String, Value>>,
}

/// Workspace change event
#[derive(Debug, Clone)]
pub enum WorkspaceChangeEvent {
    FolderAdded(WorkspaceFolder),
    FolderRemoved(WorkspaceFolder),
    DocumentOpened(TextDocument),
    DocumentClosed(String),
    DocumentChanged(String),
    ConfigurationChanged(String),
    FileCreated(String),
    FileChanged(String),
    FileDeleted(String),
}

impl WorkspaceAPI {
    /// Create a new workspace API
    pub fn new(editor: Arc<Editor>) -> Self {
        let (change_tx, change_rx) = watch::channel(WorkspaceChangeEvent::DocumentChanged("init".to_string()));

        Self {
            folders: Arc::new(RwLock::new(Vec::new())),
            documents: Arc::new(RwLock::new(HashMap::new())),
            watcher: None,
            configuration: Arc::new(RwLock::new(Configuration::default())),
            editor,
            change_tx,
            change_rx,
        }
    }

    // ==================== Workspace Folders ====================

    /// Get workspace folders
    pub async fn workspace_folders(&self) -> Vec<WorkspaceFolder> {
        self.folders.read().await.clone()
    }

    /// Add a workspace folder
    pub async fn add_workspace_folder(&self, uri: &str) -> Result<WorkspaceFolder> {
        let uri = Uri::parse(uri)?;
        let name = Path::new(&uri.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace")
            .to_string();

        let mut folders = self.folders.write().await;
        let index = folders.len();
        let folder = WorkspaceFolder { uri, name, index };
        folders.push(folder.clone());

        self.change_tx.send(WorkspaceChangeEvent::FolderAdded(folder.clone())).ok();
        Ok(folder)
    }

    /// Remove a workspace folder
    pub async fn remove_workspace_folder(&self, index: usize) -> Result<()> {
        let mut folders = self.folders.write().await;
        if index < folders.len() {
            let folder = folders.remove(index);
            self.change_tx.send(WorkspaceChangeEvent::FolderRemoved(folder)).ok();
        }
        Ok(())
    }

    /// Get the root path of the workspace
    pub async fn workspace_root(&self) -> Option<PathBuf> {
        let folders = self.folders.read().await;
        folders.first().map(|f| PathBuf::from(&f.uri.path))
    }

    // ==================== Text Documents ====================

    /// Open a text document
    pub async fn open_text_document(&self, uri: &str) -> Result<TextDocument> {
        let uri = Uri::parse(uri)?;

        // Check if already open
        if let Some(doc) = self.documents.read().await.get(&uri.to_string()) {
            return Ok(doc.clone());
        }

        // Open in editor
        self.editor.open_file(&uri.path).await?;

        let document = TextDocument {
            uri: uri.clone(),
            file_name: Path::new(&uri.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string(),
            language_id: self.detect_language(&uri.path).await,
            version: 1,
            is_dirty: false,
            is_closed: false,
            line_count: self.editor.buffer().read().await.line_count(),
        };

        self.documents.write().await.insert(uri.to_string(), document.clone());
        self.change_tx.send(WorkspaceChangeEvent::DocumentOpened(document.clone())).ok();

        Ok(document)
    }

    /// Get all text documents
    pub async fn text_documents(&self) -> Vec<TextDocument> {
        self.documents.read().await.values().cloned().collect()
    }

    /// Get a text document by URI
    pub async fn get_text_document(&self, uri: &str) -> Option<TextDocument> {
        self.documents.read().await.get(uri).cloned()
    }

    /// Close a text document
    pub async fn close_text_document(&self, uri: &str) -> Result<()> {
        self.documents.write().await.remove(uri);
        self.change_tx.send(WorkspaceChangeEvent::DocumentClosed(uri.to_string())).ok();
        Ok(())
    }

    // ==================== File Operations ====================

    /// Read a file
    pub async fn read_file(&self, uri: &str) -> Result<String> {
        let uri = Uri::parse(uri)?;
        Ok(tokio::fs::read_to_string(&uri.path).await?)
    }

    /// Write a file
    pub async fn write_file(&self, uri: &str, content: &str) -> Result<()> {
        let uri = Uri::parse(uri)?;
        Ok(tokio::fs::write(&uri.path, content).await?)
    }

    /// Delete a file
    pub async fn delete_file(&self, uri: &str) -> Result<()> {
        let uri = Uri::parse(uri)?;
        Ok(tokio::fs::remove_file(&uri.path).await?)
    }

    /// Create a directory
    pub async fn create_directory(&self, uri: &str) -> Result<()> {
        let uri = Uri::parse(uri)?;
        Ok(tokio::fs::create_dir_all(&uri.path).await?)
    }

    /// List files in a directory
    pub async fn read_directory(&self, uri: &str) -> Result<Vec<Uri>> {
        let uri = Uri::parse(uri)?;
        let mut entries = Vec::new();
        let mut read_dir = tokio::fs::read_dir(&uri.path).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            let file_uri = Uri::parse(&format!("file://{}", path.display()))?;
            entries.push(file_uri);
        }

        Ok(entries)
    }

    /// Find files matching pattern
    pub async fn find_files(&self, pattern: &str, exclude: Option<&str>, max_results: Option<usize>) -> Result<Vec<Uri>> {
        let root = self.workspace_root().await.ok_or_else(|| anyhow!("No workspace root"))?;
        let mut results = Vec::new();

        // Simple glob walker
        let walker = ignore::WalkBuilder::new(&root)
            .git_ignore(true)
            .build();

        for entry in walker {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() {
                    if let Some(path_str) = path.to_str() {
                        if path_str.contains(pattern.trim_matches('*')) {
                            results.push(Uri::parse(&format!("file://{}", path_str))?);
                            if let Some(max) = max_results {
                                if results.len() >= max {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    // ==================== Configuration ====================

    /// Get configuration value
    pub async fn get_configuration<T>(&self, section: &str) -> Option<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let config = self.configuration.read().await;

        // Check workspace first, then user
        if let Some(value) = config.workspace.get(section) {
            if let Ok(v) = serde_json::from_value(value.clone()) {
                return Some(v);
            }
        }

        if let Some(value) = config.user.get(section) {
            if let Ok(v) = serde_json::from_value(value.clone()) {
                return Some(v);
            }
        }

        None
    }

    /// Update configuration
    pub async fn update_configuration(&self, section: &str, value: Value, target: ConfigurationTarget) -> Result<()> {
        let mut config = self.configuration.write().await;

        match target {
            ConfigurationTarget::User => {
                config.user.insert(section.to_string(), value);
            }
            ConfigurationTarget::Workspace => {
                config.workspace.insert(section.to_string(), value);
            }
            ConfigurationTarget::WorkspaceFolder => {
                // For simplicity, use workspace
                config.workspace.insert(section.to_string(), value);
            }
        }

        self.change_tx.send(WorkspaceChangeEvent::ConfigurationChanged(section.to_string())).ok();
        Ok(())
    }

    /// Get all configuration
    pub async fn inspect_configuration(&self) -> Configuration {
        self.configuration.read().await.clone()
    }

    // ==================== File Watching ====================

    /// Create a file system watcher
    pub async fn create_file_system_watcher(
        &self,
        pattern: &str,
        handler: impl Fn(FileChangeEvent) -> Result<()> + Send + Sync + 'static,
    ) -> Result<impl Disposable> {
        use notify::{Watcher, RecursiveMode};

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        })?;

        if let Some(root) = self.workspace_root().await {
            watcher.watch(&root, RecursiveMode::Recursive)?;
        }

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                for path in event.paths {
                    let change_event = match event.kind {
                        notify::EventKind::Create(_) => FileChangeEvent::Created(path.to_string_lossy().to_string()),
                        notify::EventKind::Modify(_) => FileChangeEvent::Changed(path.to_string_lossy().to_string()),
                        notify::EventKind::Remove(_) => FileChangeEvent::Deleted(path.to_string_lossy().to_string()),
                        _ => continue,
                    };
                    let _ = handler(change_event);
                }
            }
        });

        struct WatcherDisposable {
            watcher: Option<notify::RecommendedWatcher>,
        }

        impl Disposable for WatcherDisposable {
            fn dispose(&self) {
                // Watcher will be dropped
            }
        }

        Ok(WatcherDisposable { watcher: Some(watcher) })
    }

    // ==================== Events ====================

    /// Subscribe to workspace changes
    pub fn on_did_change(&self) -> watch::Receiver<WorkspaceChangeEvent> {
        self.change_rx.clone()
    }

    // ==================== Utility ====================

    /// Detect language from file extension
    async fn detect_language(&self, path: &str) -> String {
        match Path::new(path).extension().and_then(|e| e.to_str()) {
            Some("rs") => "rust".to_string(),
            Some("py") => "python".to_string(),
            Some("js") => "javascript".to_string(),
            Some("ts") => "typescript".to_string(),
            Some("html") => "html".to_string(),
            Some("css") => "css".to_string(),
            Some("json") => "json".to_string(),
            Some("md") => "markdown".to_string(),
            Some("toml") => "toml".to_string(),
            Some("yaml") | Some("yml") => "yaml".to_string(),
            _ => "plaintext".to_string(),
        }
    }
}

/// Configuration target
#[derive(Debug, Clone, Copy)]
pub enum ConfigurationTarget {
    User,
    Workspace,
    WorkspaceFolder,
}

/// File change event
#[derive(Debug, Clone)]
pub enum FileChangeEvent {
    Created(String),
    Changed(String),
    Deleted(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_workspace_folders() {
        let editor = Arc::new(Editor::new());
        let api = WorkspaceAPI::new(editor);

        let folder = api.add_workspace_folder("file:///tmp/test").await.unwrap();
        assert_eq!(folder.name, "test");

        let folders = api.workspace_folders().await;
        assert_eq!(folders.len(), 1);
    }

    #[tokio::test]
    async fn test_configuration() {
        let editor = Arc::new(Editor::new());
        let api = WorkspaceAPI::new(editor);

        api.update_configuration("editor.fontSize", Value::Number(14.into()), ConfigurationTarget::User).await.unwrap();

        let size: Option<i32> = api.get_configuration("editor.fontSize").await;
        assert_eq!(size, Some(14));
    }
}