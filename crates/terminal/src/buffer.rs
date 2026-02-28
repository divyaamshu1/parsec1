//! Terminal buffer management

use crate::Result;

/// Basic color used for theming
#[derive(Debug, Clone, Copy)]
pub enum Color {
    /// default color (use terminal default)
    Default,
    /// indexed 256-color value
    Indexed(u8),
    /// truecolor rgb
    Rgb(u8, u8, u8),
}


/// Terminal buffer
#[derive(Debug, Clone)]
pub struct TerminalBuffer {
    content: Vec<String>,
}

impl TerminalBuffer {
    /// Create new buffer
    pub fn new() -> Self {
        Self {
            content: vec![String::new()],
        }
    }

    /// Write to buffer
    pub fn write(&mut self, _data: &[u8]) {
        // Stub
    }

    /// Get content
    pub async fn content(&self) -> Vec<Vec<crate::renderer::Cell>> {
        Vec::new()
    }

    /// Get visible content
    pub fn visible_content(&self) -> Vec<Vec<crate::renderer::Cell>> {
        Vec::new()
    }

    /// Get cursor position
    pub fn cursor(&self) -> (u16, u16) {
        (0, 0)
    }

    /// Get size
    pub fn size(&self) -> (u16, u16) {
        (80, 24)
    }

    /// Get title
    pub fn title(&self) -> &str {
        "Terminal"
    }

    /// Check bell received
    pub fn bell_received(&self) -> bool {
        false
    }

    /// Clear bell
    pub fn clear_bell(&mut self) {}

    /// Resize
    pub fn resize(&mut self, _rows: u16, _cols: u16) {}

    /// Clear
    pub fn clear(&mut self) {}

    /// Clear scrollback
    pub fn clear_scrollback(&mut self) {}

    /// Set scroll offset
    pub fn set_scroll_offset(&mut self, _offset: isize) {}

    /// Lines scrolled
    pub fn lines_scrolled(&self) -> u32 {
        0
    }

    /// Bell count
    pub fn bell_count(&self) -> u32 {
        0
    }

    /// Resize count
    pub fn resize_count(&self) -> u32 {
        0
    }
}
