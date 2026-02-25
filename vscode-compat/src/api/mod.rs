//! VS Code API Implementation
//!
//! This module implements the vscode.* API that extensions expect.
//! It maps VS Code API calls to Parsec's core functionality.

pub mod window;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock, Mutex};
use async_trait::async_trait;

use parsec_core::editor::{Editor, Position, Range};
use parsec_core::terminal::Terminal;
use parsec_core::git::GitManager;

/// Main VS Code API implementation
pub struct VSCodeAPI {
    // Core components
    editor: Arc<Editor>,
    terminal: Arc<Terminal>,
    git: Arc<GitManager>,

    // API modules
    pub window: window::WindowAPI,

    // Extension context
    extensions: Arc<RwLock<HashMap<String, ExtensionContext>>>,
}

/// Extension context (activation context)
pub struct ExtensionContext {
    pub id: String,
    pub subscriptions: Vec<Box<dyn Disposable>>,
    pub workspace_state: serde_json::Value,
    pub global_state: serde_json::Value,
}

/// Disposable resource (for cleanup)
pub trait Disposable: Send + Sync {
    fn dispose(&self);
}

impl<T: Fn() + Send + Sync> Disposable for T {
    fn dispose(&self) {
        self();
    }
}

/// VS Code URI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Uri {
    pub scheme: String,
    pub path: String,
    pub query: Option<String>,
    pub fragment: Option<String>,
}

impl Uri {
    pub fn parse(s: &str) -> Result<Self> {
        // Simple URI parsing
        let parts: Vec<&str> = s.split("://").collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid URI format"));
        }

        let scheme = parts[0].to_string();
        let rest = parts[1];
        
        let (path, query_fragment) = match rest.find('?') {
            Some(pos) => (&rest[..pos], Some(&rest[pos..])),
            None => (rest, None),
        };

        let (query, fragment) = if let Some(qf) = query_fragment {
            match qf.find('#') {
                Some(pos) => (Some(qf[..pos].to_string()), Some(qf[pos+1..].to_string())),
                None => (Some(qf.to_string()), None),
            }
        } else {
            (None, None)
        };

        Ok(Self {
            scheme,
            path: path.to_string(),
            query,
            fragment,
        })
    }

    pub fn to_string(&self) -> String {
        let mut s = format!("{}://{}", self.scheme, self.path);
        if let Some(query) = &self.query {
            s.push('?');
            s.push_str(query);
        }
        if let Some(fragment) = &self.fragment {
            s.push('#');
            s.push_str(fragment);
        }
        s
    }
}

/// VS Code Position (1-based)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct VSCodePosition {
    pub line: u32,
    pub character: u32,
}

impl From<Position> for VSCodePosition {
    fn from(p: Position) -> Self {
        Self {
            line: (p.line + 1) as u32,
            character: p.column as u32,
        }
    }
}

impl From<VSCodePosition> for Position {
    fn from(p: VSCodePosition) -> Self {
        Self {
            line: (p.line - 1) as usize,
            column: p.character as usize,
        }
    }
}

/// VS Code Range
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct VSCodeRange {
    pub start: VSCodePosition,
    pub end: VSCodePosition,
}

impl From<Range> for VSCodeRange {
    fn from(r: Range) -> Self {
        Self {
            start: r.start.into(),
            end: r.end.into(),
        }
    }
}

impl From<VSCodeRange> for Range {
    fn from(r: VSCodeRange) -> Self {
        Self::new(r.start.into(), r.end.into())
    }
}

/// VS Code Text Document
#[derive(Debug, Clone)]
pub struct TextDocument {
    pub uri: Uri,
    pub file_name: String,
    pub language_id: String,
    pub version: i32,
    pub is_dirty: bool,
    pub is_closed: bool,
    pub line_count: usize,
}

/// VS Code Text Editor
#[derive(Debug, Clone)]
pub struct TextEditor {
    pub document: TextDocument,
    pub selections: Vec<VSCodeRange>,
    pub visible_ranges: Vec<VSCodeRange>,
    pub view_column: Option<usize>,
}

/// VS Code Workspace Edit
#[derive(Debug, Clone)]
pub struct WorkspaceEdit {
    pub entries: HashMap<String, Vec<TextEdit>>,
}

/// VS Code Text Edit
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub range: VSCodeRange,
    pub new_text: String,
}

/// VS Code Completion Item
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub sort_text: Option<String>,
    pub filter_text: Option<String>,
    pub insert_text: Option<String>,
    pub range: Option<VSCodeRange>,
    pub commit_characters: Option<Vec<String>>,
    pub command: Option<Command>,
}

/// Completion Item Kind
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
    EnumMember,
    Constant,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

/// VS Code Command
#[derive(Debug, Clone)]
pub struct Command {
    pub title: String,
    pub command: String,
    pub arguments: Option<Vec<serde_json::Value>>,
}

impl VSCodeAPI {
    pub fn new(editor: Arc<Editor>, terminal: Arc<Terminal>, git: Arc<GitManager>) -> Self {
        Self {
            editor,
            terminal,
            git,
            window: window::WindowAPI::new(),
            extensions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ==================== commands API ====================

    pub async fn register_command<F>(&self, command: &str, callback: F)
    where
        F: Fn(Vec<serde_json::Value>) -> Result<serde_json::Value> + Send + Sync + 'static,
    {
        // Store in command registry
        // This would be implemented with a command registry
    }

    pub async fn execute_command(&self, command: &str, args: Vec<serde_json::Value>) -> Result<serde_json::Value> {
        // Look up and execute command
        Err(anyhow::anyhow!("Command not implemented: {}", command))
    }

    // ==================== workspace API ====================

    pub async fn workspace_open_text_document(&self, uri: &str) -> Result<TextDocument> {
        // Parse URI and return a TextDocument. Opening the file in the editor
        // requires mutable access to the `Editor`, which isn't available here,
        // so we return the document metadata without mutating editor state.
        let uri = Uri::parse(uri)?;
        let file_name = uri.path.clone();

        Ok(TextDocument {
            uri: uri.clone(),
            file_name,
            language_id: "plaintext".to_string(),
            version: 1,
            is_dirty: false,
            is_closed: false,
            line_count: self.editor.statistics().lines,
        })
    }

    pub async fn workspace_apply_edit(&self, edit: WorkspaceEdit) -> Result<bool> {
        // Apply edits to documents
        // This would modify the editor buffers
        Ok(true)
    }

    pub async fn workspace_find_files(&self, pattern: &str, exclude: Option<&str>) -> Result<Vec<Uri>> {
        // Find files matching pattern
        // This would use git or file system
        Ok(Vec::new())
    }

    // ==================== window API ====================

    pub fn window(&self) -> &window::WindowAPI {
        &self.window
    }

    // ==================== languages API ====================

    pub async fn languages_register_completion_item_provider(
        &self,
        selector: Vec<String>,
        provider: impl CompletionProvider + Send + Sync + 'static,
    ) {
        // Register completion provider
    }

    pub async fn languages_register_hover_provider(
        &self,
        selector: Vec<String>,
        provider: impl HoverProvider + Send + Sync + 'static,
    ) {
        // Register hover provider
    }

    // ==================== extensions API ====================

    pub async fn create_extension_context(&self, id: &str) -> ExtensionContext {
        ExtensionContext {
            id: id.to_string(),
            subscriptions: Vec::new(),
            workspace_state: serde_json::Value::Null,
            global_state: serde_json::Value::Null,
        }
    }
}

/// Completion provider trait
#[async_trait]
pub trait CompletionProvider: Send + Sync {
    async fn provide_completion_items(
        &self,
        document: &TextDocument,
        position: VSCodePosition,
    ) -> Result<Vec<CompletionItem>>;
}

/// Hover provider trait
#[async_trait]
pub trait HoverProvider: Send + Sync {
    async fn provide_hover(
        &self,
        document: &TextDocument,
        position: VSCodePosition,
    ) -> Result<Option<Hover>>;
}

/// Hover result
#[derive(Debug, Clone)]
pub struct Hover {
    pub contents: Vec<MarkdownString>,
    pub range: Option<VSCodeRange>,
}

/// Markdown string
#[derive(Debug, Clone)]
pub struct MarkdownString {
    pub value: String,
    pub is_trusted: bool,
}

impl MarkdownString {
    pub fn new(value: String) -> Self {
        Self {
            value,
            is_trusted: false,
        }
    }
}