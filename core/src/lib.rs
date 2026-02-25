//! Parsec IDE Core Engine
//!
//! This crate provides the core editing, terminal, git, syntax highlighting,
//! and process management functionality for the Parsec IDE.

#![allow(dead_code, unused_imports, unused_variables, unused_mut, ambiguous_glob_reexports, mismatched_lifetime_syntaxes)]

pub mod editor;
pub mod terminal;
pub mod git;
pub mod syntax;
pub mod process;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

/// The main core engine, aggregating all components.
pub struct ParsecCore {
    pub editor: Arc<Mutex<editor::Editor>>,
    pub terminal: Arc<terminal::Terminal>,
    pub git: Arc<git::GitManager>,
    pub syntax: Arc<syntax::SyntaxSystem>,
    pub process: Arc<process::ProcessManager>,
}

impl ParsecCore {
    /// Create a new instance of the Parsec core engine with default components.
    pub fn new() -> Self {
        Self {
            editor: Arc::new(Mutex::new(editor::Editor::new())),
            terminal: Arc::new(terminal::Terminal::new(
                "term-1".to_string(),
                "Terminal".to_string(),
                terminal::TerminalConfig::default(),
            )),
            git: Arc::new(git::GitManager::new(git::GitConfig::default())),
            syntax: Arc::new(syntax::SyntaxSystem::new(syntax::SyntaxConfig::default())),
            process: Arc::new(process::ProcessManager::new()),
        }
    }

    /// Open a file in the editor.
    pub async fn open_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        self.editor.lock().await.open_file(path).await?;
        Ok(())
    }

    /// Save the currently open file.
    pub async fn save_current(&self) -> Result<()> {
        self.editor.lock().await.save_current().await?;
        Ok(())
    }

    /// Get the current content of the active buffer.
    pub async fn get_content(&self) -> String {
        self.editor.lock().await.get_content()
    }

    /// Insert text at the current cursor position.
    pub async fn insert(&self, text: &str) {
        self.editor.lock().await.insert(text);
    }
}

impl Default for ParsecCore {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export commonly used types for convenience
pub use editor::{Editor, Position, Range, Selection, EditorMode};
pub use terminal::{Terminal, TerminalSize, TerminalEvent};
pub use git::{GitManager, Repository, Branch, Commit, FileStatus};
pub use syntax::{SyntaxSystem, SyntaxTheme, HighlightStyle};
pub use process::{ProcessManager, OutputStream};
// Note: CommandHandle is not yet defined; add it when needed.