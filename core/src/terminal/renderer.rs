//! Terminal renderer for converting buffer cells to styled text

use super::{TerminalBuffer, Color};

/// Render cell with style information
#[derive(Debug, Clone)]
pub struct RenderCell {
    pub character: char,
    pub foreground: Color,
    pub background: Color,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub reverse: bool,
    pub strikethrough: bool,
    pub is_cursor: bool,
    pub is_selected: bool,
}

/// Terminal renderer
#[derive(Debug)]
pub struct TerminalRenderer {
    /// Cursor blink state
    cursor_visible: bool,
    /// Last blink toggle time
    last_blink: std::time::Instant,
    /// Blink interval
    blink_interval: std::time::Duration,
}

impl TerminalRenderer {
    /// Create new terminal renderer
    pub fn new() -> Self {
        Self {
            cursor_visible: true,
            last_blink: std::time::Instant::now(),
            blink_interval: std::time::Duration::from_millis(500),
        }
    }

    /// Render terminal buffer to render cells
    pub fn render(&mut self, buffer: &TerminalBuffer, cursor_pos: (u16, u16)) -> Vec<Vec<RenderCell>> {
        // Update cursor blink
        self.update_cursor_blink();
        
        let visible = buffer.visible_content();
        let scroll_offset = buffer.scroll_offset();
        let mut result = Vec::with_capacity(visible.len());
        
        for (row_idx, row) in visible.iter().enumerate() {
            let mut render_row = Vec::with_capacity(row.len());
            
            for (col_idx, cell) in row.iter().enumerate() {
                let is_cursor = if scroll_offset == 0 {
                    // Only show cursor when not scrolled back
                    row_idx == cursor_pos.0 as usize && col_idx == cursor_pos.1 as usize && self.cursor_visible
                } else {
                    false
                };
                
                render_row.push(RenderCell {
                    character: cell.character,
                    foreground: cell.foreground,
                    background: cell.background,
                    bold: cell.bold,
                    dim: cell.dim,
                    italic: cell.italic,
                    underline: cell.underline,
                    blink: cell.blink,
                    reverse: cell.reverse,
                    strikethrough: cell.strikethrough,
                    is_cursor,
                    is_selected: false, // Selection handled separately
                });
            }
            
            result.push(render_row);
        }
        
        result
    }

    /// Update cursor blink state
    fn update_cursor_blink(&mut self) {
        let now = std::time::Instant::now();
        if now - self.last_blink >= self.blink_interval {
            self.cursor_visible = !self.cursor_visible;
            self.last_blink = now;
        }
    }

    /// Convert render cells to styled text (for debugging or fallback)
    pub fn to_string(&self, cells: &[Vec<RenderCell>]) -> String {
        let mut result = String::new();
        
        for row in cells {
            for cell in row {
                result.push(cell.character);
            }
            result.push('\n');
        }
        
        result
    }

    /// Apply ANSI styling to a character
    pub fn apply_ansi_style(&self, cell: &RenderCell) -> String {
        let mut codes: Vec<String> = Vec::new();
        
        // Foreground color
        match cell.foreground {
            Color::Black => codes.push("30".to_string()),
            Color::Red => codes.push("31".to_string()),
            Color::Green => codes.push("32".to_string()),
            Color::Yellow => codes.push("33".to_string()),
            Color::Blue => codes.push("34".to_string()),
            Color::Magenta => codes.push("35".to_string()),
            Color::Cyan => codes.push("36".to_string()),
            Color::White => codes.push("37".to_string()),
            Color::BrightBlack => codes.push("90".to_string()),
            Color::BrightRed => codes.push("91".to_string()),
            Color::BrightGreen => codes.push("92".to_string()),
            Color::BrightYellow => codes.push("93".to_string()),
            Color::BrightBlue => codes.push("94".to_string()),
            Color::BrightMagenta => codes.push("95".to_string()),
            Color::BrightCyan => codes.push("96".to_string()),
            Color::BrightWhite => codes.push("97".to_string()),
            Color::Indexed(n) => codes.push(format!("38;5;{}", n)),
            Color::Rgb(r, g, b) => codes.push(format!("38;2;{};{};{}", r, g, b)),
            _ => {}
        }
        
        // Background color
        match cell.background {
            Color::Black => codes.push("40".to_string()),
            Color::Red => codes.push("41".to_string()),
            Color::Green => codes.push("42".to_string()),
            Color::Yellow => codes.push("43".to_string()),
            Color::Blue => codes.push("44".to_string()),
            Color::Magenta => codes.push("45".to_string()),
            Color::Cyan => codes.push("46".to_string()),
            Color::White => codes.push("47".to_string()),
            Color::BrightBlack => codes.push("100".to_string()),
            Color::BrightRed => codes.push("101".to_string()),
            Color::BrightGreen => codes.push("102".to_string()),
            Color::BrightYellow => codes.push("103".to_string()),
            Color::BrightBlue => codes.push("104".to_string()),
            Color::BrightMagenta => codes.push("105".to_string()),
            Color::BrightCyan => codes.push("106".to_string()),
            Color::BrightWhite => codes.push("107".to_string()),
            Color::Indexed(n) => codes.push(format!("48;5;{}", n)),
            Color::Rgb(r, g, b) => codes.push(format!("48;2;{};{};{}", r, g, b)),
            _ => {}
        }
        
        // Text styles
        if cell.bold {
            codes.push("1".to_string());
        }
        if cell.dim {
            codes.push("2".to_string());
        }
        if cell.italic {
            codes.push("3".to_string());
        }
        if cell.underline {
            codes.push("4".to_string());
        }
        if cell.blink {
            codes.push("5".to_string());
        }
        if cell.reverse {
            codes.push("7".to_string());
        }
        if cell.strikethrough {
            codes.push("9".to_string());
        }
        
        if codes.is_empty() {
            cell.character.to_string()
        } else {
            format!("\x1b[{}m{}\x1b[0m", codes.join(";"), cell.character)
        }
    }

    /// Set cursor blink interval
    pub fn set_blink_interval(&mut self, interval: std::time::Duration) {
        self.blink_interval = interval;
    }

    /// Force cursor visible (for debugging)
    pub fn force_cursor_visible(&mut self) {
        self.cursor_visible = true;
    }

    /// Force cursor hidden (for debugging)
    pub fn force_cursor_hidden(&mut self) {
        self.cursor_visible = false;
    }
}

impl Default for TerminalRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let renderer = TerminalRenderer::new();
        assert!(renderer.cursor_visible);
    }

    #[test]
    fn test_cursor_blink() {
        let mut renderer = TerminalRenderer::new();
        renderer.force_cursor_visible();
        assert!(renderer.cursor_visible);
        
        std::thread::sleep(std::time::Duration::from_millis(600));
        renderer.update_cursor_blink();
        assert!(!renderer.cursor_visible);
    }

    #[test]
    fn test_ansi_style() {
        let renderer = TerminalRenderer::new();
        let cell = RenderCell {
            character: 'A',
            foreground: Color::Red,
            background: Color::Blue,
            bold: true,
            dim: false,
            italic: false,
            underline: false,
            blink: false,
            reverse: false,
            strikethrough: false,
            is_cursor: false,
            is_selected: false,
        };
        
        let styled = renderer.apply_ansi_style(&cell);
        assert!(styled.contains("\x1b[31;44;1m"));
        assert!(styled.contains("A"));
    }
}