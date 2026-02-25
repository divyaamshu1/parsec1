//! Extension API for Parsec extensions
//!
//! Provides the API that extensions can use to interact with the editor.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use serde::{Serialize, Deserialize};
use tokio::sync::Mutex;
use async_trait::async_trait;

/// Main extension API
pub struct ExtensionAPI {
    /// Extension ID
    extension_id: String,
    /// Editor handle
    editor: Arc<dyn EditorAPI + Send + Sync>,
    /// Workspace handle
    workspace: Arc<dyn WorkspaceAPI + Send + Sync>,
    /// Window handle
    window: Arc<dyn WindowAPI + Send + Sync>,
    /// Commands handle
    commands: Arc<dyn CommandsAPI + Send + Sync>,
    /// Languages handle
    languages: Arc<dyn LanguagesAPI + Send + Sync>,
    /// Configuration handle
    config: Arc<dyn ConfigAPI + Send + Sync>,
    /// Clipboard handle
    clipboard: Arc<dyn ClipboardAPI + Send + Sync>,
    /// Storage handle
    storage: Arc<dyn StorageAPI + Send + Sync>,
}

/// Editor API for extensions
#[async_trait]
pub trait EditorAPI {
    async fn get_text(&self) -> String;
    async fn set_text(&self, text: &str);
    async fn insert(&self, position: Position, text: &str);
    async fn delete(&self, range: Range);
    async fn get_selection(&self) -> Vec<Range>;
    async fn set_selection(&self, ranges: Vec<Range>);
    async fn get_cursor_position(&self) -> Position;
    async fn set_cursor_position(&self, position: Position);
    async fn get_line(&self, line: usize) -> Option<String>;
    async fn get_line_count(&self) -> usize;
    async fn save(&self);
    async fn get_language(&self) -> Option<String>;
    async fn set_language(&self, language: &str);
}

/// Workspace API for extensions
#[async_trait]
pub trait WorkspaceAPI {
    async fn open_file(&self, path: &Path) -> Result<()>;
    async fn close_file(&self, path: &Path) -> Result<()>;
    async fn get_open_files(&self) -> Vec<PathBuf>;
    async fn get_current_file(&self) -> Option<PathBuf>;
    async fn find_files(&self, pattern: &str) -> Result<Vec<PathBuf>>;
    async fn read_file(&self, path: &Path) -> Result<String>;
    async fn write_file(&self, path: &Path, content: &str) -> Result<()>;
    async fn get_workspace_root(&self) -> Option<PathBuf>;
    async fn get_workspace_folders(&self) -> Vec<PathBuf>;
}

/// Window API for extensions
#[async_trait]
pub trait WindowAPI {
    async fn show_info_message(&self, message: &str);
    async fn show_warning_message(&self, message: &str);
    async fn show_error_message(&self, message: &str);
    async fn show_input_box(&self, options: InputBoxOptions) -> Option<String>;
    async fn show_quick_pick(&self, items: Vec<QuickPickItem>) -> Option<QuickPickItem>;
    async fn set_status_bar_message(&self, message: &str);
    async fn create_output_channel(&self, name: &str) -> OutputChannel;
}

/// Commands API for extensions
#[async_trait]
/// Commands API for extensions
pub trait CommandsAPI {
    async fn register_command(&self, command: &str, callback: Arc<dyn Fn(Vec<serde_json::Value>) -> Result<serde_json::Value> + Send + Sync>) -> Result<()>;
    async fn execute_command(&self, command: &str, args: Vec<serde_json::Value>) -> Result<serde_json::Value>;
}

/// Languages API for extensions
#[async_trait]
pub trait LanguagesAPI {
    async fn register_completion_provider(
        &self,
        selector: DocumentSelector,
        provider: Box<dyn CompletionProvider>,
    );
    async fn register_hover_provider(
        &self,
        selector: DocumentSelector,
        provider: Box<dyn HoverProvider>,
    );
    async fn register_definition_provider(
        &self,
        selector: DocumentSelector,
        provider: Box<dyn DefinitionProvider>,
    );
    async fn register_reference_provider(
        &self,
        selector: DocumentSelector,
        provider: Box<dyn ReferenceProvider>,
    );
}

/// Configuration API for extensions
#[async_trait]
/// Configuration API for extensions
#[async_trait]
pub trait ConfigAPI {
    async fn get(&self, key: &str) -> Option<serde_json::Value>;
    async fn set(&self, key: &str, value: serde_json::Value) -> Result<()>;
    async fn has(&self, key: &str) -> bool;
}

/// Clipboard API for extensions
#[async_trait]
pub trait ClipboardAPI {
    async fn read_text(&self) -> String;
    async fn write_text(&self, text: &str);
    async fn read_html(&self) -> Option<String>;
    async fn write_html(&self, html: &str);
}

/// Storage API for extensions
#[async_trait]
pub trait StorageAPI {
    async fn get(&self, key: &str) -> Option<serde_json::Value>;
    async fn set(&self, key: &str, value: serde_json::Value);
    async fn delete(&self, key: &str);
    async fn clear(&self);
}

/// Position in a text document
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// Range in a text document
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// Input box options
#[derive(Debug, Clone)]
pub struct InputBoxOptions {
    pub prompt: Option<String>,
    pub placeholder: Option<String>,
    pub default: Option<String>,
    pub password: bool,
}

/// Quick pick item
#[derive(Debug, Clone)]
pub struct QuickPickItem {
    pub label: String,
    pub description: Option<String>,
    pub detail: Option<String>,
    pub picked: bool,
}

/// Output channel
#[derive(Debug, Clone)]
pub struct OutputChannel {
    name: String,
    buffer: Arc<Mutex<Vec<String>>>,
}

impl OutputChannel {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn append(&self, text: &str) {
        self.buffer.lock().await.push(text.to_string());
    }

    pub async fn append_line(&self, text: &str) {
        self.buffer.lock().await.push(format!("{}\n", text));
    }

    pub async fn clear(&self) {
        self.buffer.lock().await.clear();
    }

    pub async fn show(&self) {
        // Would show in UI
    }
}

/// Document selector for language features
#[derive(Debug, Clone)]
pub struct DocumentSelector {
    pub language: Option<String>,
    pub scheme: Option<String>,
    pub pattern: Option<String>,
}

/// Completion provider trait
#[async_trait]
pub trait CompletionProvider: Send + Sync {
    async fn provide_completion_items(
        &self,
        document: &str,
        position: Position,
    ) -> Vec<CompletionItem>;
}

/// Completion item
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
}

/// Completion item kind
#[derive(Debug, Clone, Copy)]
pub enum CompletionItemKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
}

/// Hover provider trait
#[async_trait]
pub trait HoverProvider: Send + Sync {
    async fn provide_hover(&self, document: &str, position: Position) -> Option<Hover>;
}

/// Hover information
#[derive(Debug, Clone)]
pub struct Hover {
    pub contents: Vec<String>,
    pub range: Option<Range>,
}

/// Definition provider trait
#[async_trait]
pub trait DefinitionProvider: Send + Sync {
    async fn provide_definition(&self, document: &str, position: Position) -> Vec<Location>;
}

/// Reference provider trait
#[async_trait]
pub trait ReferenceProvider: Send + Sync {
    async fn provide_references(&self, document: &str, position: Position) -> Vec<Location>;
}

/// Location in a document
#[derive(Debug, Clone)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

/// Disposable for cleanup
pub trait Disposable: Send + Sync {
    fn dispose(&self);
}

impl<T: Fn() + Send + Sync> Disposable for T {
    fn dispose(&self) {
        self();
    }
}

/// Context for extension activation
pub struct ExtensionContext {
    /// Extension ID
    pub id: String,
    /// Extension API
    pub api: Arc<ExtensionAPI>,
    /// Subscriptions for cleanup
    subscriptions: Arc<Mutex<Vec<Box<dyn Disposable>>>>,
}

impl ExtensionContext {
    pub fn new(id: String, api: Arc<ExtensionAPI>) -> Self {
        Self {
            id,
            api,
            subscriptions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Subscribe a disposable for cleanup
    pub async fn subscribe(&self, disposable: Box<dyn Disposable>) {
        self.subscriptions.lock().await.push(disposable);
    }

    /// Dispose all subscriptions
    pub async fn dispose(&self) {
        let mut subs = self.subscriptions.lock().await;
        for sub in subs.drain(..) {
            sub.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockEditor;

    #[async_trait]
    impl EditorAPI for MockEditor {
        async fn get_text(&self) -> String { String::new() }
        async fn set_text(&self, _text: &str) {}
        async fn insert(&self, _position: Position, _text: &str) {}
        async fn delete(&self, _range: Range) {}
        async fn get_selection(&self) -> Vec<Range> { vec![] }
        async fn set_selection(&self, _ranges: Vec<Range>) {}
        async fn get_cursor_position(&self) -> Position { Position { line: 0, column: 0 } }
        async fn set_cursor_position(&self, _position: Position) {}
        async fn get_line(&self, _line: usize) -> Option<String> { None }
        async fn get_line_count(&self) -> usize { 0 }
        async fn save(&self) {}
        async fn get_language(&self) -> Option<String> { None }
        async fn set_language(&self, _language: &str) {}
    }

    #[tokio::test]
    async fn test_output_channel() {
        let channel = OutputChannel::new("test");
        channel.append_line("Hello, world!").await;
        channel.append("More text").await;
        
        let buffer = channel.buffer.lock().await;
        assert_eq!(buffer.len(), 2);
    }
}