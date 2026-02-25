//! Editor view component for the GUI

use std::sync::Arc;

use serde::{Serialize, Deserialize};
use tauri::{Window, Emitter};
use tokio::sync::Mutex;

use parsec_core::editor::{Editor, Position, Range, EditorMode};

/// Editor view configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorViewConfig {
    pub font_size: u32,
    pub font_family: String,
    pub line_height: f32,
    pub tab_size: usize,
    pub word_wrap: bool,
    pub line_numbers: bool,
    pub minimap: bool,
    pub render_whitespace: bool,
    pub bracket_pair_colorization: bool,
    pub glow_intensity: f32,
    pub theme: String,
}

impl Default for EditorViewConfig {
    fn default() -> Self {
        Self {
            font_size: 14,
            font_family: "Cascadia Code, Fira Code, monospace".to_string(),
            line_height: 1.5,
            tab_size: 4,
            word_wrap: false,
            line_numbers: true,
            minimap: true,
            render_whitespace: false,
            bracket_pair_colorization: true,
            glow_intensity: 0.3,
            theme: "dark".to_string(),
        }
    }
}

/// Editor cursor information
#[derive(Debug, Clone, Serialize)]
pub struct CursorInfo {
    pub line: usize,
    pub column: usize,
    pub selections: Vec<Range>,
    pub primary: usize,
}

/// Editor content change
#[derive(Debug, Clone, Serialize)]
pub struct ContentChange {
    pub range: Range,
    pub text: String,
    pub version: usize,
}

/// Editor view state
pub struct EditorView {
    editor: Arc<Mutex<Editor>>,
    window: Window,
    config: EditorViewConfig,
    version: usize,
    dirty: bool,
}

impl EditorView {
    pub fn new(editor: Arc<Mutex<Editor>>, window: Window) -> Self {
        Self {
            editor,
            window,
            config: EditorViewConfig::default(),
            version: 0,
            dirty: false,
        }
    }

    /// Initialize the editor view
    pub async fn init(&self) -> Result<(), String> {
        let content = self.editor.lock().await.get_content();

        self.window.emit("editor:content", &content)
            .map_err(|e: tauri::Error| e.to_string())?;

        self.window.emit("editor:config", &self.config)
            .map_err(|e: tauri::Error| e.to_string())?;
        
        Ok(())
    }

    /// Update editor configuration
    pub async fn set_config(&mut self, config: EditorViewConfig) -> Result<(), String> {
        self.config = config;

        self.window.emit("editor:config", &self.config)
            .map_err(|e: tauri::Error| e.to_string())?;
        
        Ok(())
    }

    /// Handle cursor position change from frontend
    pub async fn on_cursor_moved(&self, _line: usize, _column: usize) {
        let mut _editor = self.editor.lock().await;
        // Update cursor position in editor
        // This would need proper cursor API
    }

    /// Handle text input from frontend
    pub async fn on_text_input(&mut self, text: String, _position: Position) -> Result<(), String> {
        let mut editor = self.editor.lock().await;
        editor.insert(&text);
        self.version += 1;
        self.dirty = true;
        
        // Emit content change to other clients if needed
        Ok(())
    }

    /// Handle selection change from frontend
    pub async fn on_selection_changed(&self, _selections: Vec<Range>) {
        // Update selections in editor
    }

    /// Get current cursor info
    pub async fn cursor_info(&self) -> CursorInfo {
        let editor = self.editor.lock().await;
        let stats = editor.statistics();
        
        CursorInfo {
            line: stats.cursor_line,
            column: stats.cursor_column,
            selections: Vec::new(), // Would get from editor
            primary: 0,
        }
    }

    /// Scroll to position
    pub async fn scroll_to(&self, line: usize, column: usize) -> Result<(), String> {
        self.window.emit("editor:scroll", &(line, column))
            .map_err(|e: tauri::Error| e.to_string())
    }

    /// Highlight range
    pub async fn highlight_range(&self, range: Range) -> Result<(), String> {
        self.window.emit("editor:highlight", &range)
            .map_err(|e: tauri::Error| e.to_string())
    }

    /// Set editor mode
    pub async fn set_mode(&self, mode: EditorMode) -> Result<(), String> {
        self.window.emit("editor:mode", &format!("{:?}", mode))
            .map_err(|e: tauri::Error| e.to_string())
    }

    /// Check if editor is dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Get current version
    pub fn version(&self) -> usize {
        self.version
    }
}

/// Editor commands for frontend
#[tauri::command]
#[allow(non_snake_case)]
pub fn editor_insert(
    text: String,
    line: usize,
    column: usize,
    _state: tauri::State<'_, crate::AppState>,
    _window: Window,
) -> Result<(), String> {
    // Note: Async operations with AppState containing unsync traits are restricted.
    // This would require redesigning the state management to be Send/Sync safe.
    Ok(())
}

#[tauri::command]
pub async fn editor_delete(
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    let editor = state.core.editor.clone();
    // Would need to delete range
    Ok(())
}

#[tauri::command]
pub async fn editor_undo(
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    let mut editor = state.core.editor.lock().await;
    editor.undo();
    Ok(())
}

#[tauri::command]
pub async fn editor_redo(
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    let mut editor = state.core.editor.lock().await;
    editor.redo();
    Ok(())
}

#[tauri::command]
pub async fn editor_copy(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Option<String>, String> {
    // Get selected text
    Ok(None)
}

#[tauri::command]
pub async fn editor_cut(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Option<String>, String> {
    // Cut selected text
    Ok(None)
}

#[tauri::command]
pub async fn editor_paste(
    text: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    let mut editor = state.core.editor.lock().await;
    editor.insert(&text);
    Ok(())
}

#[tauri::command]
pub async fn editor_find(
    query: String,
    case_sensitive: bool,
    state: tauri::State<'_, crate::AppState>,
    window: Window,
) -> Result<Vec<Range>, String> {
    let editor = state.core.editor.lock().await;
    let matches = editor.search(&query, case_sensitive);
    
    window.emit("editor:found", &matches)
        .map_err(|e| e.to_string())?;
    
    Ok(matches)
}

#[tauri::command]
pub async fn editor_replace(
    query: String,
    replace: String,
    case_sensitive: bool,
    state: tauri::State<'_, crate::AppState>,
) -> Result<usize, String> {
    let mut editor = state.core.editor.lock().await;
    Ok(editor.replace_all(&query, &replace, case_sensitive))
}

#[tauri::command]
pub async fn editor_format(
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    // Would call formatter
    Ok(())
}