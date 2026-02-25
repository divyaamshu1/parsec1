//! API that extensions can use to interact with Parsec

use async_trait::async_trait;
use serde_json::Value;

use crate::types::{Position, Range, Selection};

/// Main extension trait — all extensions must implement this
#[async_trait]
pub trait Extension: Send + Sync {
    /// Extension metadata
    fn manifest(&self) -> crate::Manifest;
    
    /// Called when extension is activated
    async fn activate(&self, ctx: &mut ExtensionContext) -> Result<(), ExtensionError>;
    
    /// Called when extension is deactivated
    async fn deactivate(&self, _ctx: &ExtensionContext) -> Result<(), ExtensionError> {
        Ok(())
    }
    
    /// Handle a command
    async fn handle_command(
        &self, 
        _ctx: &ExtensionContext,
        command: &str, 
        _args: Vec<Value>
    ) -> Result<Value, ExtensionError> {
        Err(ExtensionError::CommandNotFound(command.to_string()))
    }
}

/// Context provided to extensions
pub struct ExtensionContext {
    /// Extension ID
    pub id: String,
    /// Workspace root
    pub workspace_root: Option<String>,
    /// Extension storage path
    pub storage_path: String,
    /// Logger
    pub logger: Logger,
    /// API handles
    editor: EditorHandle,
    terminal: TerminalHandle,
    git: GitHandle,
    ai: AIHandle,
}

impl ExtensionContext {
    pub fn new(id: String, storage_path: String) -> Self {
        Self {
            logger: Logger::new(&id),
            id,
            workspace_root: None,
            storage_path,
            editor: EditorHandle::new(),
            terminal: TerminalHandle::new(),
            git: GitHandle::new(),
            ai: AIHandle::new(),
        }
    }
    
    /// Editor API
    pub fn editor(&self) -> &EditorHandle {
        &self.editor
    }
    
    /// Terminal API
    pub fn terminal(&self) -> &TerminalHandle {
        &self.terminal
    }
    
    /// Git API
    pub fn git(&self) -> &GitHandle {
        &self.git
    }
    
    /// AI API
    pub fn ai(&self) -> &AIHandle {
        &self.ai
    }
}

/// Editor handle
pub struct EditorHandle { /* ... */ }

impl EditorHandle {
    fn new() -> Self { Self {} }
    
    /// Get current text content
    pub async fn get_text(&self) -> Result<String, ExtensionError> {
        // This will call back to Parsec host
        todo!("Implement IPC call")
    }
    
    /// Insert text at position
    pub async fn insert(&self, _position: Position, _text: &str) -> Result<(), ExtensionError> {
        todo!("Implement IPC call")
    }
    
    /// Delete range
    pub async fn delete(&self, _range: Range) -> Result<(), ExtensionError> {
        todo!("Implement IPC call")
    }
    
    /// Get current selection
    pub async fn get_selection(&self) -> Result<Vec<Selection>, ExtensionError> {
        todo!("Implement IPC call")
    }
    
    /// Set selection
    pub async fn set_selection(&self, _selections: Vec<Selection>) -> Result<(), ExtensionError> {
        todo!("Implement IPC call")
    }
}

/// Terminal handle
pub struct TerminalHandle { /* ... */ }

impl TerminalHandle {
    fn new() -> Self { Self {} }
    
    /// Create new terminal
    pub async fn create(&self, _name: &str) -> Result<String, ExtensionError> {
        todo!("Implement IPC call")
    }
    
    /// Write to terminal
    pub async fn write(&self, _id: &str, _data: &[u8]) -> Result<(), ExtensionError> {
        todo!("Implement IPC call")
    }
}

/// Git handle
pub struct GitHandle { /* ... */ }

impl GitHandle {
    fn new() -> Self { Self {} }
    
    /// Get git status
    pub async fn status(&self) -> Result<Vec<GitFile>, ExtensionError> {
        todo!("Implement IPC call")
    }
    
    /// Stage files
    pub async fn stage(&self, _files: &[String]) -> Result<(), ExtensionError> {
        todo!("Implement IPC call")
    }
    
    /// Commit
    pub async fn commit(&self, _message: &str) -> Result<String, ExtensionError> {
        todo!("Implement IPC call")
    }
}

/// AI handle
pub struct AIHandle { /* ... */ }

impl AIHandle {
    fn new() -> Self { Self {} }
    
    /// Complete text
    pub async fn complete(&self, _prompt: &str) -> Result<String, ExtensionError> {
        todo!("Implement IPC call")
    }
    
    /// Chat with AI
    pub async fn chat(&self, _messages: Vec<ChatMessage>) -> Result<String, ExtensionError> {
        todo!("Implement IPC call")
    }
}

#[derive(Debug, Clone)]
pub struct GitFile {
    pub path: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,  // "user", "assistant", "system"
    pub content: String,
}

/// Simple logger for extensions
pub struct Logger {
    id: String,
}

impl Logger {
    fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }
    
    pub fn info(&self, msg: &str) {
        println!("[{} INFO] {}", self.id, msg);
    }
    
    pub fn warn(&self, msg: &str) {
        eprintln!("[{} WARN] {}", self.id, msg);
    }
    
    pub fn error(&self, msg: &str) {
        eprintln!("[{} ERROR] {}", self.id, msg);
    }
}

/// Extension error types
#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    #[error("Command not found: {0}")]
    CommandNotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("IPC error: {0}")]
    IpcError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}