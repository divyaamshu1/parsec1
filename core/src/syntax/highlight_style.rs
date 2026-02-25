//! Highlight style types
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightStyle {
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
}

impl Default for HighlightStyle {
    fn default() -> Self {
        Self {
            foreground: None,
            background: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        }
    }
}