//! Terminal themes and color schemes

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

use crate::buffer::Color;

use crate::Cell;

/// RGB color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }

    pub fn to_hex(&self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    pub fn to_ansi(&self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.r, self.g, self.b)
    }

    pub fn to_ansi_bg(&self) -> String {
        format!("\x1b[48;2;{};{};{}m", self.r, self.g, self.b)
    }
}

/// Font style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FontStyle {
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub reverse: bool,
    pub strikethrough: bool,
    pub hidden: bool,
}

impl Default for FontStyle {
    fn default() -> Self {
        Self {
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            blink: false,
            reverse: false,
            strikethrough: false,
            hidden: false,
        }
    }
}

/// Color scheme for a terminal theme
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ColorScheme {
    /// Name
    pub name: String,
    /// Author
    pub author: Option<String>,
    /// Background color
    pub background: RgbColor,
    /// Foreground color
    pub foreground: RgbColor,
    /// Cursor color
    pub cursor: RgbColor,
    /// Selection background
    pub selection: RgbColor,
    /// ANSI black (color 0)
    pub black: RgbColor,
    /// ANSI red (color 1)
    pub red: RgbColor,
    /// ANSI green (color 2)
    pub green: RgbColor,
    /// ANSI yellow (color 3)
    pub yellow: RgbColor,
    /// ANSI blue (color 4)
    pub blue: RgbColor,
    /// ANSI magenta (color 5)
    pub magenta: RgbColor,
    /// ANSI cyan (color 6)
    pub cyan: RgbColor,
    /// ANSI white (color 7)
    pub white: RgbColor,
    /// ANSI bright black (color 8)
    pub bright_black: RgbColor,
    /// ANSI bright red (color 9)
    pub bright_red: RgbColor,
    /// ANSI bright green (color 10)
    pub bright_green: RgbColor,
    /// ANSI bright yellow (color 11)
    pub bright_yellow: RgbColor,
    /// ANSI bright blue (color 12)
    pub bright_blue: RgbColor,
    /// ANSI bright magenta (color 13)
    pub bright_magenta: RgbColor,
    /// ANSI bright cyan (color 14)
    pub bright_cyan: RgbColor,
    /// ANSI bright white (color 15)
    pub bright_white: RgbColor,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::dark()
    }
}

impl ColorScheme {
    /// Create dark theme
    pub fn dark() -> Self {
        Self {
            name: "Dark".to_string(),
            author: None,
            background: RgbColor::new(30, 30, 30),
            foreground: RgbColor::new(220, 220, 220),
            cursor: RgbColor::new(255, 255, 255),
            selection: RgbColor::new(60, 60, 60),
            black: RgbColor::new(0, 0, 0),
            red: RgbColor::new(205, 49, 49),
            green: RgbColor::new(13, 188, 121),
            yellow: RgbColor::new(229, 229, 16),
            blue: RgbColor::new(36, 114, 200),
            magenta: RgbColor::new(188, 63, 188),
            cyan: RgbColor::new(17, 168, 205),
            white: RgbColor::new(229, 229, 229),
            bright_black: RgbColor::new(102, 102, 102),
            bright_red: RgbColor::new(241, 76, 76),
            bright_green: RgbColor::new(35, 209, 139),
            bright_yellow: RgbColor::new(245, 245, 67),
            bright_blue: RgbColor::new(59, 142, 234),
            bright_magenta: RgbColor::new(214, 112, 214),
            bright_cyan: RgbColor::new(41, 184, 219),
            bright_white: RgbColor::new(255, 255, 255),
        }
    }

    /// Create light theme
    pub fn light() -> Self {
        Self {
            name: "Light".to_string(),
            author: None,
            background: RgbColor::new(255, 255, 255),
            foreground: RgbColor::new(30, 30, 30),
            cursor: RgbColor::new(0, 0, 0),
            selection: RgbColor::new(200, 200, 200),
            black: RgbColor::new(0, 0, 0),
            red: RgbColor::new(205, 49, 49),
            green: RgbColor::new(13, 188, 121),
            yellow: RgbColor::new(229, 229, 16),
            blue: RgbColor::new(36, 114, 200),
            magenta: RgbColor::new(188, 63, 188),
            cyan: RgbColor::new(17, 168, 205),
            white: RgbColor::new(229, 229, 229),
            bright_black: RgbColor::new(102, 102, 102),
            bright_red: RgbColor::new(241, 76, 76),
            bright_green: RgbColor::new(35, 209, 139),
            bright_yellow: RgbColor::new(245, 245, 67),
            bright_blue: RgbColor::new(59, 142, 234),
            bright_magenta: RgbColor::new(214, 112, 214),
            bright_cyan: RgbColor::new(41, 184, 219),
            bright_white: RgbColor::new(255, 255, 255),
        }
    }

    /// Create Solarized dark theme
    pub fn solarized_dark() -> Self {
        Self {
            name: "Solarized Dark".to_string(),
            author: Some("Ethan Schoonover".to_string()),
            background: RgbColor::new(0, 43, 54),
            foreground: RgbColor::new(131, 148, 150),
            cursor: RgbColor::new(220, 220, 220),
            selection: RgbColor::new(7, 54, 66),
            black: RgbColor::new(7, 54, 66),
            red: RgbColor::new(220, 50, 47),
            green: RgbColor::new(133, 153, 0),
            yellow: RgbColor::new(181, 137, 0),
            blue: RgbColor::new(38, 139, 210),
            magenta: RgbColor::new(211, 54, 130),
            cyan: RgbColor::new(42, 161, 152),
            white: RgbColor::new(238, 232, 213),
            bright_black: RgbColor::new(0, 43, 54),
            bright_red: RgbColor::new(220, 50, 47),
            bright_green: RgbColor::new(133, 153, 0),
            bright_yellow: RgbColor::new(181, 137, 0),
            bright_blue: RgbColor::new(38, 139, 210),
            bright_magenta: RgbColor::new(211, 54, 130),
            bright_cyan: RgbColor::new(42, 161, 152),
            bright_white: RgbColor::new(253, 246, 227),
        }
    }

    /// Create Nord theme
    pub fn nord() -> Self {
        Self {
            name: "Nord".to_string(),
            author: Some("Arctic Ice Studio".to_string()),
            background: RgbColor::new(46, 52, 64),
            foreground: RgbColor::new(216, 222, 233),
            cursor: RgbColor::new(216, 222, 233),
            selection: RgbColor::new(67, 76, 94),
            black: RgbColor::new(46, 52, 64),
            red: RgbColor::new(191, 97, 106),
            green: RgbColor::new(163, 190, 140),
            yellow: RgbColor::new(235, 203, 139),
            blue: RgbColor::new(129, 161, 193),
            magenta: RgbColor::new(180, 142, 173),
            cyan: RgbColor::new(143, 188, 187),
            white: RgbColor::new(216, 222, 233),
            bright_black: RgbColor::new(76, 86, 106),
            bright_red: RgbColor::new(191, 97, 106),
            bright_green: RgbColor::new(163, 190, 140),
            bright_yellow: RgbColor::new(235, 203, 139),
            bright_blue: RgbColor::new(129, 161, 193),
            bright_magenta: RgbColor::new(180, 142, 173),
            bright_cyan: RgbColor::new(143, 188, 187),
            bright_white: RgbColor::new(236, 239, 244),
        }
    }
}

/// Terminal theme
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub color_scheme: ColorScheme,
    pub font_family: Option<String>,
    pub font_size: Option<u16>,
    pub line_height: Option<f32>,
    pub cursor_style: crate::CursorStyle,
    pub cursor_blink: bool,
    pub scrollbar_style: ScrollbarStyle,
}

/// Scrollbar style
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarStyle {
    Hidden,
    Thin,
    Normal,
    Thick,
}

impl Default for ScrollbarStyle {
    fn default() -> Self {
        ScrollbarStyle::Normal
    }
}

/// Theme manager
pub struct ThemeManager {
    themes: HashMap<String, Theme>,
    active_theme: Arc<RwLock<String>>,
}

impl ThemeManager {
    /// Create new theme manager
    pub fn new() -> Self {
        let mut themes = HashMap::new();
        
        // Add default themes
        themes.insert("dark".to_string(), Theme {
            name: "dark".to_string(),
            color_scheme: ColorScheme::dark(),
            font_family: None,
            font_size: None,
            line_height: None,
            cursor_style: crate::CursorStyle::Block,
            cursor_blink: true,
            scrollbar_style: ScrollbarStyle::Normal,
        });

        themes.insert("light".to_string(), Theme {
            name: "light".to_string(),
            color_scheme: ColorScheme::light(),
            font_family: None,
            font_size: None,
            line_height: None,
            cursor_style: crate::CursorStyle::Beam,
            cursor_blink: true,
            scrollbar_style: ScrollbarStyle::Normal,
        });

        themes.insert("solarized-dark".to_string(), Theme {
            name: "solarized-dark".to_string(),
            color_scheme: ColorScheme::solarized_dark(),
            font_family: None,
            font_size: None,
            line_height: None,
            cursor_style: crate::CursorStyle::Block,
            cursor_blink: true,
            scrollbar_style: ScrollbarStyle::Normal,
        });

        themes.insert("nord".to_string(), Theme {
            name: "nord".to_string(),
            color_scheme: ColorScheme::nord(),
            font_family: None,
            font_size: None,
            line_height: None,
            cursor_style: crate::CursorStyle::Underline,
            cursor_blink: true,
            scrollbar_style: ScrollbarStyle::Normal,
        });

        Self {
            themes,
            active_theme: Arc::new(RwLock::new("dark".to_string())),
        }
    }

    /// Add a theme
    pub fn add_theme(&mut self, theme: Theme) {
        self.themes.insert(theme.name.clone(), theme);
    }

    /// Remove a theme
    pub fn remove_theme(&mut self, name: &str) -> Option<Theme> {
        self.themes.remove(name)
    }

    /// Get a theme
    pub fn get_theme(&self, name: &str) -> Option<Theme> {
        self.themes.get(name).cloned()
    }

    /// List all theme names
    pub fn list_themes(&self) -> Vec<String> {
        self.themes.keys().cloned().collect()
    }

    /// Set active theme
    pub fn set_active_theme(&self, name: &str) -> bool {
        if self.themes.contains_key(name) {
            *self.active_theme.write() = name.to_string();
            true
        } else {
            false
        }
    }

    /// Get active theme
    pub fn active_theme(&self) -> Theme {
        let name = self.active_theme.read().clone();
        self.themes.get(&name).cloned().unwrap_or_else(|| self.themes["dark"].clone())
    }

    /// Apply theme to a cell
    pub fn apply_theme_to_cell(&self, cell: &mut crate::Cell, theme: &Theme) {
        if cell.reverse {
            std::mem::swap(&mut cell.foreground, &mut cell.background);
        }

        match cell.foreground {
            Some(Color::Indexed(i)) => {
                let color = match i {
                    0 => theme.color_scheme.black,
                    1 => theme.color_scheme.red,
                    2 => theme.color_scheme.green,
                    3 => theme.color_scheme.yellow,
                    4 => theme.color_scheme.blue,
                    5 => theme.color_scheme.magenta,
                    6 => theme.color_scheme.cyan,
                    7 => theme.color_scheme.white,
                    8 => theme.color_scheme.bright_black,
                    9 => theme.color_scheme.bright_red,
                    10 => theme.color_scheme.bright_green,
                    11 => theme.color_scheme.bright_yellow,
                    12 => theme.color_scheme.bright_blue,
                    13 => theme.color_scheme.bright_magenta,
                    14 => theme.color_scheme.bright_cyan,
                    15 => theme.color_scheme.bright_white,
                    _ => theme.color_scheme.foreground,
                };
                cell.foreground = Some(Color::Rgb(color.r, color.g, color.b));
            }
            Some(Color::Default) => {
                cell.foreground = Some(Color::Rgb(
                    theme.color_scheme.foreground.r,
                    theme.color_scheme.foreground.g,
                    theme.color_scheme.foreground.b,
                ));
            }
            _ => {}
        }

        match cell.background {
            Some(Color::Indexed(i)) => {
                let color = match i {
                    0 => theme.color_scheme.black,
                    1 => theme.color_scheme.red,
                    2 => theme.color_scheme.green,
                    3 => theme.color_scheme.yellow,
                    4 => theme.color_scheme.blue,
                    5 => theme.color_scheme.magenta,
                    6 => theme.color_scheme.cyan,
                    7 => theme.color_scheme.white,
                    8 => theme.color_scheme.bright_black,
                    9 => theme.color_scheme.bright_red,
                    10 => theme.color_scheme.bright_green,
                    11 => theme.color_scheme.bright_yellow,
                    12 => theme.color_scheme.bright_blue,
                    13 => theme.color_scheme.bright_magenta,
                    14 => theme.color_scheme.bright_cyan,
                    15 => theme.color_scheme.bright_white,
                    _ => theme.color_scheme.background,
                };
                cell.background = Some(Color::Rgb(color.r, color.g, color.b));
            }
            Some(Color::Default) => {
                cell.background = Some(Color::Rgb(
                    theme.color_scheme.background.r,
                    theme.color_scheme.background.g,
                    theme.color_scheme.background.b,
                ));
            }
            _ => {}
        }
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}