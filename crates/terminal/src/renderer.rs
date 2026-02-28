//! Terminal rendering

use crate::Result;

/// Cell in terminal
#[derive(Debug, Clone, Copy)]
pub struct Cell {
    pub character: char,
    pub foreground: Option<crate::buffer::Color>,
    pub background: Option<crate::buffer::Color>,
    pub reverse: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            character: '\0',
            foreground: None,
            background: None,
            reverse: false,
        }
    }
}


/// Terminal renderer
#[derive(Debug, Clone)]
pub struct TerminalRenderer;

impl TerminalRenderer {
    /// Create new renderer
    pub fn new() -> Self {
        Self
    }

    /// Render content
    pub fn render(&self, _content: &[Vec<Cell>]) -> Result<String> {
        Ok(String::new())
    }
}
