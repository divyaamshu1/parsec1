//! Text editor core module
//!
//! Provides the main editor functionality including buffer management,
//! cursor control, selections, undo/redo history, and text manipulation.

mod buffer;
mod cursor;
mod selection;
mod history;
mod position;
mod movement;
mod edit;

pub use buffer::*;
pub use cursor::*;
pub use selection::*;
pub use history::*;
pub use position::*;
pub use movement::*;

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Main editor struct managing buffers, cursors, and editor state
pub struct Editor {
    /// List of open buffers
    buffers: Vec<Buffer>,
    /// Index of currently active buffer
    active_buffer: usize,
    /// Global cursor manager (supports multi-cursor)
    cursors: CursorManager,
    /// Undo/redo history
    history: EditHistory,
    /// Editor configuration
    config: EditorConfig,
    /// Clipboard content
    clipboard: Option<String>,
    /// File change watcher
    watcher: Option<notify::RecommendedWatcher>,
    /// Current editor mode
    mode: EditorMode,
}

/// Editor configuration settings
#[derive(Debug, Clone)]
pub struct EditorConfig {
    /// Tab width in spaces
    pub tab_width: usize,
    /// Whether to expand tabs to spaces
    pub expand_tab: bool,
    /// Show line numbers
    pub line_numbers: bool,
    /// Highlight current line
    pub highlight_current_line: bool,
    /// Enable syntax highlighting
    pub syntax_highlighting: bool,
    /// Auto-pair brackets and quotes
    pub auto_pair: bool,
    /// Show whitespace characters
    pub show_whitespace: bool,
    /// Maximum file size to open (bytes)
    pub max_file_size: usize,
    /// Auto-save on focus loss
    pub auto_save: bool,
    /// End of line sequence
    pub eol: EOL,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_width: 4,
            expand_tab: true,
            line_numbers: true,
            highlight_current_line: true,
            syntax_highlighting: true,
            auto_pair: true,
            show_whitespace: false,
            max_file_size: 10 * 1024 * 1024, // 10MB
            auto_save: false,
            eol: if cfg!(windows) { EOL::CRLF } else { EOL::LF },
        }
    }
}

/// End of line sequence
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EOL {
    LF,
    CRLF,
    CR,
}

/// Editor mode (Vim-style)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorMode {
    #[default]
    Normal,
    Insert,
    Visual,
    VisualLine,
    Command,
    Search,
}

impl Editor {
    /// Create a new editor instance
    pub fn new() -> Self {
        let mut editor = Self {
            buffers: Vec::new(),
            active_buffer: 0,
            cursors: CursorManager::new(),
            history: EditHistory::new(1000),
            config: EditorConfig::default(),
            clipboard: None,
            watcher: None,
            mode: EditorMode::Normal,
        };
        
        // Create an empty buffer
        editor.create_new_buffer();
        
        editor
    }

    /// Create a new empty buffer
    pub fn create_new_buffer(&mut self) -> usize {
        let buffer = Buffer::new(None);
        self.buffers.push(buffer);
        self.active_buffer = self.buffers.len() - 1;
        self.active_buffer
    }

    /// Open a file in a new buffer
    pub async fn open_file<P: AsRef<Path>>(&mut self, path: P) -> Result<usize> {
        let path = path.as_ref();
        
        // Check if file is already open
        if let Some(idx) = self.buffers.iter().position(|b| b.path() == Some(path)) {
            self.active_buffer = idx;
            return Ok(idx);
        }
        
        // Check file size
        let metadata = tokio::fs::metadata(path).await?;
        if metadata.len() > self.config.max_file_size as u64 {
            anyhow::bail!("File too large (max: {} bytes)", self.config.max_file_size);
        }
        
        let buffer = Buffer::from_file(path).await?;
        self.buffers.push(buffer);
        self.active_buffer = self.buffers.len() - 1;
        
        Ok(self.active_buffer)
    }

    /// Save the current buffer
    pub async fn save_current(&self) -> Result<()> {
        if let Some(buffer) = self.buffers.get(self.active_buffer) {
            buffer.save().await?;
        }
        Ok(())
    }

    /// Get content of active buffer
    pub fn get_content(&self) -> String {
        self.buffers
            .get(self.active_buffer)
            .map(|b| b.text())
            .unwrap_or_default()
    }

    /// Insert text at current cursor position
    pub fn insert(&mut self, text: &str) {
        if let Some(buffer) = self.buffers.get_mut(self.active_buffer) {
            let pos = self.cursors.primary();
            let idx = buffer.position_to_index(pos);
            buffer.insert(idx, text);
            
            // Move cursor forward
            let new_pos = Position {
                line: pos.line,
                column: pos.column + text.len(),
            };
            self.cursors.update_position(0, new_pos);
            
            self.mode = EditorMode::Insert;
        }
    }

    /// Delete character before cursor (backspace)
    pub fn backspace(&mut self) {
        if let Some(buffer) = self.buffers.get_mut(self.active_buffer) {
            let pos = self.cursors.primary();
            if pos.column > 0 {
                let idx = buffer.position_to_index(pos);
                buffer.delete(idx - 1..idx);
                
                let new_pos = Position {
                    line: pos.line,
                    column: pos.column - 1,
                };
                self.cursors.update_position(0, new_pos);
            } else if pos.line > 0 {
                // Join with previous line
                let prev_line_len = buffer.line_length(pos.line - 1);
                let idx = buffer.position_to_index(pos);
                buffer.delete(idx - 1..idx); // Remove newline
                
                let new_pos = Position {
                    line: pos.line - 1,
                    column: prev_line_len,
                };
                self.cursors.update_position(0, new_pos);
            }
        }
    }

    /// Delete character at cursor (delete)
    pub fn delete(&mut self) {
        if let Some(buffer) = self.buffers.get_mut(self.active_buffer) {
            let pos = self.cursors.primary();
            let idx = buffer.position_to_index(pos);
            
            if idx < buffer.len() {
                buffer.delete(idx..idx + 1);
            }
        }
    }

    /// Insert new line at cursor
    pub fn new_line(&mut self) {
        self.insert("\n");
    }

    /// Move cursor up one line
    pub fn move_up(&mut self) {
        let pos = self.cursors.primary();
        if pos.line > 0 {
            let new_pos = Position {
                line: pos.line - 1,
                column: pos.column,
            };
            self.cursors.update_position(0, new_pos);
        }
    }

    /// Move cursor down one line
    pub fn move_down(&mut self) {
        if let Some(buffer) = self.buffers.get(self.active_buffer) {
            let pos = self.cursors.primary();
            if pos.line + 1 < buffer.line_count() {
                let new_pos = Position {
                    line: pos.line + 1,
                    column: pos.column,
                };
                self.cursors.update_position(0, new_pos);
            }
        }
    }

    /// Move cursor left one character
    pub fn move_left(&mut self) {
        let pos = self.cursors.primary();
        if pos.column > 0 {
            let new_pos = Position {
                line: pos.line,
                column: pos.column - 1,
            };
            self.cursors.update_position(0, new_pos);
        } else if pos.line > 0 {
            if let Some(buffer) = self.buffers.get(self.active_buffer) {
                let prev_line_len = buffer.line_length(pos.line - 1);
                let new_pos = Position {
                    line: pos.line - 1,
                    column: prev_line_len,
                };
                self.cursors.update_position(0, new_pos);
            }
        }
    }

    /// Move cursor right one character
    pub fn move_right(&mut self) {
        if let Some(buffer) = self.buffers.get(self.active_buffer) {
            let pos = self.cursors.primary();
            let line_len = buffer.line_length(pos.line);
            
            if pos.column < line_len {
                let new_pos = Position {
                    line: pos.line,
                    column: pos.column + 1,
                };
                self.cursors.update_position(0, new_pos);
            } else if pos.line + 1 < buffer.line_count() {
                let new_pos = Position {
                    line: pos.line + 1,
                    column: 0,
                };
                self.cursors.update_position(0, new_pos);
            }
        }
    }

    /// Undo last operation
    pub fn undo(&mut self) {
        if let Some(op) = self.history.undo() {
            self.apply_edit_operation_inverse(&op);
        }
    }

    /// Redo last undone operation
    pub fn redo(&mut self) {
        if let Some(op) = self.history.redo() {
            self.apply_edit_operation(&op);
        }
    }

    /// Apply an edit operation
    fn apply_edit_operation(&mut self, op: &EditOperation) {
        if let Some(buffer) = self.buffers.get_mut(self.active_buffer) {
            match op {
                EditOperation::Insert { position, text } => {
                    let idx = buffer.position_to_index(*position);
                    buffer.insert(idx, text);
                }
                EditOperation::Delete { position, text } => {
                    let start = buffer.position_to_index(*position);
                    let end = start + text.len();
                    buffer.delete(start..end);
                }
                EditOperation::Replace { range, text } => {
                    let start = buffer.position_to_index(range.start);
                    let end = buffer.position_to_index(range.end);
                    buffer.delete(start..end);
                    buffer.insert(start, text);
                }
            }
        }
    }

    /// Apply inverse of an edit operation (for undo)
    fn apply_edit_operation_inverse(&mut self, op: &EditOperation) {
        if let Some(buffer) = self.buffers.get_mut(self.active_buffer) {
            match op {
                EditOperation::Insert { position, text } => {
                    let start = buffer.position_to_index(*position);
                    let end = start + text.len();
                    buffer.delete(start..end);
                }
                EditOperation::Delete { position, text } => {
                    let idx = buffer.position_to_index(*position);
                    buffer.insert(idx, text);
                }
                EditOperation::Replace { range, text } => {
                    let start = buffer.position_to_index(range.start);
                    let end = buffer.position_to_index(range.end);
                    buffer.delete(start..end);
                    buffer.insert(start, text);
                }
            }
        }
    }

    /// Search for text in current buffer
    pub fn search(&self, query: &str, case_sensitive: bool) -> Vec<Range> {
        self.buffers
            .get(self.active_buffer)
            .map(|b| b.search(query, case_sensitive))
            .unwrap_or_default()
    }

    /// Replace all occurrences of text
    pub fn replace_all(&mut self, query: &str, replace: &str, case_sensitive: bool) -> usize {
        if let Some(buffer) = self.buffers.get_mut(self.active_buffer) {
            buffer.replace_all(query, replace, case_sensitive)
        } else {
            0
        }
    }

    /// Get editor statistics
    pub fn statistics(&self) -> EditorStats {
        self.buffers
            .get(self.active_buffer)
            .map(|b| EditorStats {
                lines: b.line_count(),
                characters: b.len(),
                words: b.word_count(),
                bytes: b.bytes(),
                cursor_line: self.cursors.primary().line,
                cursor_column: self.cursors.primary().column,
                selections: self.cursors.count(),
                mode: self.mode,
                modified: b.is_modified(),
                path: b.path().map(|p| p.to_path_buf()),
            })
            .unwrap_or_default()
    }

    /// Set editor mode
    pub fn set_mode(&mut self, mode: EditorMode) {
        self.mode = mode;
    }

    /// Get current mode
    pub fn mode(&self) -> EditorMode {
        self.mode
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for EOL {
    fn default() -> Self {
        if cfg!(windows) {
            EOL::CRLF
        } else {
            EOL::LF
        }
    }
}

/// Editor statistics for status bar
#[derive(Debug, Clone, Default)]
pub struct EditorStats {
    pub lines: usize,
    pub characters: usize,
    pub words: usize,
    pub bytes: usize,
    pub cursor_line: usize,
    pub cursor_column: usize,
    pub selections: usize,
    pub mode: EditorMode,
    pub modified: bool,
    pub path: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_new() {
        let editor = Editor::new();
        assert_eq!(editor.buffers.len(), 1);
        assert_eq!(editor.mode, EditorMode::Normal);
    }

    #[tokio::test]
    async fn test_open_file() -> Result<()> {
        let mut editor = Editor::new();
        let path = std::env::temp_dir().join("test.txt");
        tokio::fs::write(&path, "Hello, world!").await?;
        
        let idx = editor.open_file(&path).await?;
        assert_eq!(idx, 1); // New buffer index
        assert_eq!(editor.get_content(), "Hello, world!");
        
        tokio::fs::remove_file(path).await?;
        Ok(())
    }

    #[test]
    fn test_insert() {
        let mut editor = Editor::new();
        editor.insert("Hello");
        assert_eq!(editor.get_content(), "Hello");
    }

    #[test]
    fn test_cursor_movement() {
        let mut editor = Editor::new();
        editor.insert("Hello\nWorld");
        
        assert_eq!(editor.cursors.primary().line, 1);
        assert_eq!(editor.cursors.primary().column, 5);
        
        editor.move_up();
        assert_eq!(editor.cursors.primary().line, 0);
        assert_eq!(editor.cursors.primary().column, 5); // Preferred column preserved
    }

    #[test]
    fn test_undo_redo() {
        let mut editor = Editor::new();
        editor.insert("Hello");
        assert_eq!(editor.get_content(), "Hello");
        
        editor.undo();
        assert_eq!(editor.get_content(), "");
        
        editor.redo();
        assert_eq!(editor.get_content(), "Hello");
    }
}