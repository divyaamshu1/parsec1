//! Parsec Terminal Emulator
//!
//! A feature-rich terminal emulator with support for:
//! - Multiple terminal multiplexing
//! - Split panes (horizontal/vertical)
//! - Search with regex and fuzzy matching
//! - Custom themes
//! - Full PTY support (Unix/Windows)
//! - WebAssembly support for web terminals
//! - Serialization for state persistence

#![allow(dead_code, unused_imports)]

pub mod multiplexer;
pub mod split;
pub mod search;
pub mod themes;
mod error;
mod pty;
mod renderer;
mod buffer;
mod selection;

// Re-exports
pub use error::TerminalError;
pub use multiplexer::{Multiplexer, TerminalSession, SessionEvent};
pub use split::{SplitManager, SplitPane, PaneId, Direction};
pub use search::{TerminalSearch, SearchMatch, SearchOptions, SearchDirection};
pub use themes::{Theme, ThemeManager, ColorScheme, FontStyle};
pub use pty::{PtyProcess, PtySize, PtyEvent};
pub use renderer::{TerminalRenderer, Cell};
pub use buffer::TerminalBuffer;
pub use selection::Selection;

use std::sync::Arc;
use tokio::sync::RwLock;

/// Result type for terminal operations
pub type Result<T> = std::result::Result<T, TerminalError>;

/// Terminal instance configuration
#[derive(Debug, Clone)]
pub struct TerminalConfig {
    /// Initial rows
    pub rows: u16,
    /// Initial columns
    pub cols: u16,
    /// Scrollback buffer size
    pub scrollback_lines: usize,
    /// Theme name
    pub theme: String,
    /// Enable true color (24-bit)
    pub true_color: bool,
    /// Enable bracketed paste mode
    pub bracketed_paste: bool,
    /// Enable focus events
    pub focus_events: bool,
    /// Mouse reporting mode
    pub mouse_mode: MouseMode,
    /// Cursor style
    pub cursor_style: CursorStyle,
    /// Font family
    pub font_family: String,
    /// Font size in pixels
    pub font_size: u16,
    /// Line height
    pub line_height: f32,
    /// Enable blinking cursor
    pub blink_cursor: bool,
    /// Shell path
    pub shell: Option<String>,
    /// Working directory
    pub working_dir: Option<String>,
    /// Environment variables
    pub env: Vec<(String, String)>,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            rows: 24,
            cols: 80,
            scrollback_lines: 10000,
            theme: "dark".to_string(),
            true_color: true,
            bracketed_paste: true,
            focus_events: false,
            mouse_mode: MouseMode::default(),
            cursor_style: CursorStyle::Block,
            font_family: "monospace".to_string(),
            font_size: 14,
            line_height: 1.2,
            blink_cursor: true,
            shell: None,
            working_dir: None,
            env: Vec::new(),
        }
    }
}

/// Mouse reporting mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseMode {
    /// Mouse reporting disabled
    Disabled,
    /// Basic mouse tracking (click events)
    Basic,
    /// Mouse tracking with drag events
    Drag,
    /// Mouse tracking with motion events
    Motion,
    /// SGR extended mouse mode (1006)
    Sgr,
    /// URXVT extended mouse mode (1015)
    Urxvt,
}

/// Cursor appearance style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    Block,
    Beam,
    Underline,
}

impl Default for MouseMode {
    fn default() -> Self {
        MouseMode::Disabled
    }
}

/// Terminal event
#[derive(Debug, Clone)]
pub enum TerminalEvent {
    /// Data received from PTY
    Data(Vec<u8>),
    /// Terminal resized
    Resized { rows: u16, cols: u16 },
    /// Bell character received
    Bell,
    /// Title changed
    TitleChanged(String),
    /// Icon name changed
    IconNameChanged(String),
    /// Clipboard request
    ClipboardRequest(String),
    /// Color palette changed
    PaletteChanged(Vec<(u8, [u8; 3])>),
    /// Mouse event
    Mouse(MouseEvent),
    /// Focus gained
    FocusGained,
    /// Focus lost
    FocusLost,
    /// OSC command
    OscCommand(u16, Vec<String>),
}

/// Mouse event
#[derive(Debug, Clone, Copy)]
pub struct MouseEvent {
    pub button: MouseButton,
    pub action: MouseAction,
    pub modifiers: u8,
    pub row: u16,
    pub col: u16,
}

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,
    None,
}

/// Mouse action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseAction {
    Press,
    Release,
    Click,
    Drag,
    Motion,
}

/// Terminal statistics
#[derive(Debug, Clone, Default)]
pub struct TerminalStats {
    /// Bytes written to PTY
    pub bytes_written: u64,
    /// Bytes read from PTY
    pub bytes_read: u64,
    /// Number of lines scrolled
    pub lines_scrolled: u64,
    /// Number of bell events
    pub bell_count: u64,
    /// Peak memory usage
    pub peak_memory_kb: usize,
    /// Current memory usage
    pub current_memory_kb: usize,
    /// Uptime in seconds
    pub uptime_secs: u64,
    /// Number of resize events
    pub resize_count: u32,
}

/// Terminal handle for interacting with a running terminal
#[derive(Clone)]
pub struct TerminalHandle {
    id: String,
    multiplexer: Arc<RwLock<Multiplexer>>,
}

impl TerminalHandle {
    /// Create new terminal handle
    pub fn new(id: String, multiplexer: Arc<RwLock<Multiplexer>>) -> Self {
        Self { id, multiplexer }
    }

    /// Get terminal ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Write data to terminal
    pub async fn write(&self, data: &[u8]) -> Result<()> {
        let multiplexer = self.multiplexer.read().await;
        if let Some(session) = multiplexer.get_session(&self.id) {
            session.write(data).await?;
            Ok(())
        } else {
            Err(TerminalError::SessionNotFound(self.id.clone()))
        }
    }

    /// Resize terminal
    pub async fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        let multiplexer = self.multiplexer.read().await;
        if let Some(session) = multiplexer.get_session(&self.id) {
            session.resize(rows, cols).await?;
            Ok(())
        } else {
            Err(TerminalError::SessionNotFound(self.id.clone()))
        }
    }

    /// Clear terminal
    pub async fn clear(&self) -> Result<()> {
        let multiplexer = self.multiplexer.read().await;
        if let Some(session) = multiplexer.get_session(&self.id) {
            session.clear().await?;
            Ok(())
        } else {
            Err(TerminalError::SessionNotFound(self.id.clone()))
        }
    }

    /// Clear scrollback buffer
    pub async fn clear_scrollback(&self) -> Result<()> {
        let multiplexer = self.multiplexer.read().await;
        if let Some(session) = multiplexer.get_session(&self.id) {
            session.clear_scrollback().await?;
            Ok(())
        } else {
            Err(TerminalError::SessionNotFound(self.id.clone()))
        }
    }

    /// Get terminal content
    pub async fn content(&self) -> Result<Vec<Vec<Cell>>> {
        let multiplexer = self.multiplexer.read().await;
        if let Some(session) = multiplexer.get_session(&self.id) {
            Ok(session.content().await)
        } else {
            Err(TerminalError::SessionNotFound(self.id.clone()))
        }
    }

    /// Get visible content with scroll offset
    pub async fn visible_content(&self) -> Result<Vec<Vec<Cell>>> {
        let multiplexer = self.multiplexer.read().await;
        if let Some(session) = multiplexer.get_session(&self.id) {
            Ok(session.visible_content().await)
        } else {
            Err(TerminalError::SessionNotFound(self.id.clone()))
        }
    }

    /// Get cursor position
    pub async fn cursor_position(&self) -> Result<(u16, u16)> {
        let multiplexer = self.multiplexer.read().await;
        if let Some(session) = multiplexer.get_session(&self.id) {
            Ok(session.cursor_position().await)
        } else {
            Err(TerminalError::SessionNotFound(self.id.clone()))
        }
    }

    /// Get terminal size
    pub async fn size(&self) -> Result<(u16, u16)> {
        let multiplexer = self.multiplexer.read().await;
        if let Some(session) = multiplexer.get_session(&self.id) {
            Ok(session.size().await)
        } else {
            Err(TerminalError::SessionNotFound(self.id.clone()))
        }
    }

    /// Get terminal title
    pub async fn title(&self) -> Result<String> {
        let multiplexer = self.multiplexer.read().await;
        if let Some(session) = multiplexer.get_session(&self.id) {
            Ok(session.title().await)
        } else {
            Err(TerminalError::SessionNotFound(self.id.clone()))
        }
    }

    /// Get terminal stats
    pub async fn stats(&self) -> Result<TerminalStats> {
        let multiplexer = self.multiplexer.read().await;
        if let Some(session) = multiplexer.get_session(&self.id) {
            Ok(session.stats().await)
        } else {
            Err(TerminalError::SessionNotFound(self.id.clone()))
        }
    }

    /// Close terminal
    pub async fn close(&self) -> Result<()> {
        let mut multiplexer = self.multiplexer.write().await;
        multiplexer.close_session(&self.id).await?;
        Ok(())
    }

    /// Check if terminal is alive
    pub async fn is_alive(&self) -> bool {
        let multiplexer = self.multiplexer.read().await;
        multiplexer.get_session(&self.id).map_or(false, |s| s.is_alive())
    }
}