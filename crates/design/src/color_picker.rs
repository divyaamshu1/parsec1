//! Advanced color picker with multiple color spaces

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use colorsys::{Rgb, Hsl, Hsv, Cmyk, Lab, Lch};
use palette::{Srgb, LinSrgb, Hsl as PaletteHsl, Hsv as PaletteHsv, Lch as PaletteLch};
use palette::casting::from_component_slice;
use colorgrad::{Gradient as ColorGradient, GradientBuilder, Color as GradColor};

use crate::{Result, DesignError};

/// Color space
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorSpace {
    Rgb,
    Hsl,
    Hsv,
    Cmyk,
    Lab,
    Lch,
    Hex,
}

/// Color format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorFormat {
    Hex,
    Rgb,
    Rgba,
    Hsl,
    Hsla,
    Hsv,
    Hsva,
    Cmyk,
    Lab,
    Lch,
}

/// Color
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Color {
    /// Red (0-255)
    pub r: u8,
    /// Green (0-255)
    pub g: u8,
    /// Blue (0-255)
    pub b: u8,
    /// Alpha (0-1)
    pub a: f32,
    /// Color name (optional)
    pub name: Option<String>,
}

impl Color {
    /// Create new color
    pub fn new(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self { r, g, b, a, name: None }
    }

    /// Create from hex string
    pub fn from_hex(hex: &str) -> Result<Self> {
        let hex = hex.trim_start_matches('#');
        
        let (r, g, b, a) = match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).map_err(|_| DesignError::ColorError("Invalid hex".to_string()))?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).map_err(|_| DesignError::ColorError("Invalid hex".to_string()))?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).map_err(|_| DesignError::ColorError("Invalid hex".to_string()))?;
                (r, g, b, 1.0)
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| DesignError::ColorError("Invalid hex".to_string()))?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| DesignError::ColorError("Invalid hex".to_string()))?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| DesignError::ColorError("Invalid hex".to_string()))?;
                (r, g, b, 1.0)
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| DesignError::ColorError("Invalid hex".to_string()))?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| DesignError::ColorError("Invalid hex".to_string()))?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| DesignError::ColorError("Invalid hex".to_string()))?;
                let a = u8::from_str_radix(&hex[6..8], 16).map_err(|_| DesignError::ColorError("Invalid hex".to_string()))?;
                (r, g, b, a as f32 / 255.0)
            }
            _ => return Err(DesignError::ColorError("Invalid hex length".to_string())),
        };

        Ok(Self::new(r, g, b, a))
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        if self.a < 1.0 {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, (self.a * 255.0) as u8)
        } else {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        }
    }

    /// Convert to RGB string
    pub fn to_rgb_string(&self) -> String {
        if self.a < 1.0 {
            format!("rgba({}, {}, {}, {})", self.r, self.g, self.b, self.a)
        } else {
            format!("rgb({}, {}, {})", self.r, self.g, self.b)
        }
    }

    /// Convert to HSL
    pub fn to_hsl(&self) -> (f32, f32, f32) {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let l = (max + min) / 2.0;

        let (h, s) = if delta == 0.0 {
            (0.0, 0.0)
        } else {
            let s = delta / (1.0 - (2.0 * l - 1.0).abs());
            
            let h = if max == r {
                60.0 * (((g - b) / delta) % 6.0)
            } else if max == g {
                60.0 * (((b - r) / delta) + 2.0)
            } else {
                60.0 * (((r - g) / delta) + 4.0)
            };

            (h, s)
        };

        (h, s * 100.0, l * 100.0)
    }

    /// Convert to HSL string
    pub fn to_hsl_string(&self) -> String {
        let (h, s, l) = self.to_hsl();
        if self.a < 1.0 {
            format!("hsla({}deg, {}%, {}%, {})", h.round(), s.round(), l.round(), self.a)
        } else {
            format!("hsl({}deg, {}%, {}%)", h.round(), s.round(), l.round())
        }
    }

    /// Convert to HSV
    pub fn to_hsv(&self) -> (f32, f32, f32) {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let v = max;

        let (h, s) = if delta == 0.0 {
            (0.0, 0.0)
        } else {
            let s = delta / max;

            let h = if max == r {
                60.0 * (((g - b) / delta) % 6.0)
            } else if max == g {
                60.0 * (((b - r) / delta) + 2.0)
            } else {
                60.0 * (((r - g) / delta) + 4.0)
            };

            (h, s)
        };

        (h, s * 100.0, v * 100.0)
    }

    /// Convert to CMYK
    pub fn to_cmyk(&self) -> (f32, f32, f32, f32) {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;

        let k = 1.0 - r.max(g).max(b);
        
        if k == 1.0 {
            return (0.0, 0.0, 0.0, 1.0);
        }

        let c = (1.0 - r - k) / (1.0 - k);
        let m = (1.0 - g - k) / (1.0 - k);
        let y = (1.0 - b - k) / (1.0 - k);

        (c * 100.0, m * 100.0, y * 100.0, k * 100.0)
    }

    /// Lighten color
    pub fn lighten(&self, amount: f32) -> Self {
        let (h, s, l) = self.to_hsl();
        let new_l = (l + amount).min(100.0).max(0.0);
        Self::from_hsl(h, s, new_l, self.a)
    }

    /// Darken color
    pub fn darken(&self, amount: f32) -> Self {
        self.lighten(-amount)
    }

    /// Saturate color
    pub fn saturate(&self, amount: f32) -> Self {
        let (h, s, l) = self.to_hsl();
        let new_s = (s + amount).min(100.0).max(0.0);
        Self::from_hsl(h, new_s, l, self.a)
    }

    /// Desaturate color
    pub fn desaturate(&self, amount: f32) -> Self {
        self.saturate(-amount)
    }

    /// Rotate hue
    pub fn rotate_hue(&self, degrees: f32) -> Self {
        let (h, s, l) = self.to_hsl();
        let new_h = (h + degrees) % 360.0;
        Self::from_hsl(new_h, s, l, self.a)
    }

    /// Get complementary color
    pub fn complementary(&self) -> Self {
        self.rotate_hue(180.0)
    }

    /// Create from HSL
    pub fn from_hsl(h: f32, s: f32, l: f32, a: f32) -> Self {
        let h = h / 360.0;
        let s = s / 100.0;
        let l = l / 100.0;

        if s == 0.0 {
            let v = (l * 255.0) as u8;
            return Self::new(v, v, v, a);
        }

        let hue_to_rgb = |p: f32, q: f32, t: f32| -> f32 {
            let t = if t < 0.0 { t + 1.0 } else if t > 1.0 { t - 1.0 } else { t };
            
            if t < 1.0 / 6.0 {
                p + (q - p) * 6.0 * t
            } else if t < 1.0 / 2.0 {
                q
            } else if t < 2.0 / 3.0 {
                p + (q - p) * (2.0 / 3.0 - t) * 6.0
            } else {
                p
            }
        };

        let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
        let p = 2.0 * l - q;

        let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
        let g = hue_to_rgb(p, q, h);
        let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

        Self::new(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            a,
        )
    }

    /// Get contrast ratio with another color
    pub fn contrast_ratio(&self, other: &Color) -> f32 {
        let l1 = self.luminance();
        let l2 = other.luminance();
        
        let lighter = l1.max(l2);
        let darker = l1.min(l2);
        
        (lighter + 0.05) / (darker + 0.05)
    }

    /// Calculate relative luminance
    fn luminance(&self) -> f32 {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;

        let r = if r <= 0.03928 { r / 12.92 } else { ((r + 0.055) / 1.055).powf(2.4) };
        let g = if g <= 0.03928 { g / 12.92 } else { ((g + 0.055) / 1.055).powf(2.4) };
        let b = if b <= 0.03928 { b / 12.92 } else { ((b + 0.055) / 1.055).powf(2.4) };

        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// Check WCAG compliance
    pub fn wcag_compliance(&self, background: &Color) -> WcagLevel {
        let ratio = self.contrast_ratio(background);
        
        if ratio >= 7.0 {
            WcagLevel::AAA
        } else if ratio >= 4.5 {
            WcagLevel::AA
        } else if ratio >= 3.0 {
            WcagLevel::AALarge
        } else {
            WcagLevel::Fail
        }
    }
}

/// WCAG compliance level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WcagLevel {
    AAA,
    AA,
    AALarge,
    Fail,
}

/// Gradient stop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientStop {
    pub color: Color,
    pub position: f32,
}

/// Gradient
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gradient {
    pub name: String,
    pub stops: Vec<GradientStop>,
    pub angle: f32,
    pub repeat: bool,
}

impl Gradient {
    /// Create linear gradient
    pub fn linear(name: String, stops: Vec<GradientStop>, angle: f32) -> Self {
        Self {
            name,
            stops,
            angle,
            repeat: false,
        }
    }

    /// Create radial gradient
    pub fn radial(name: String, stops: Vec<GradientStop>) -> Self {
        Self {
            name,
            stops,
            angle: 0.0,
            repeat: false,
        }
    }

    /// Generate CSS gradient
    pub fn to_css(&self) -> String {
        let stops_str: Vec<String> = self.stops
            .iter()
            .map(|s| format!("{} {}%", s.color.to_hex(), s.position * 100.0))
            .collect();

        format!(
            "linear-gradient({}deg, {})",
            self.angle,
            stops_str.join(", ")
        )
    }

    /// Sample color at position
    pub fn sample(&self, position: f32) -> Option<Color> {
        if self.stops.is_empty() {
            return None;
        }

        if self.stops.len() == 1 {
            return Some(self.stops[0].color.clone());
        }

        let position = if self.repeat {
            position % 1.0
        } else {
            position.clamp(0.0, 1.0)
        };

        // Find surrounding stops
        for i in 0..self.stops.len() - 1 {
            let stop1 = &self.stops[i];
            let stop2 = &self.stops[i + 1];

            if position >= stop1.position && position <= stop2.position {
                let t = (position - stop1.position) / (stop2.position - stop1.position);
                return Some(self.interpolate(stop1, stop2, t));
            }
        }

        if position < self.stops[0].position {
            Some(self.stops[0].color.clone())
        } else {
            Some(self.stops.last().unwrap().color.clone())
        }
    }

    /// Interpolate between two colors
    fn interpolate(&self, stop1: &GradientStop, stop2: &GradientStop, t: f32) -> Color {
        let r = (stop1.color.r as f32 * (1.0 - t) + stop2.color.r as f32 * t) as u8;
        let g = (stop1.color.g as f32 * (1.0 - t) + stop2.color.g as f32 * t) as u8;
        let b = (stop1.color.b as f32 * (1.0 - t) + stop2.color.b as f32 * t) as u8;
        let a = stop1.color.a * (1.0 - t) + stop2.color.a * t;

        Color::new(r, g, b, a)
    }
}

/// Color palette
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPalette {
    pub id: String,
    pub name: String,
    pub colors: Vec<Color>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Color harmony type
#[derive(Debug, Clone, Copy)]
pub enum ColorHarmony {
    Monochromatic,
    Complementary,
    SplitComplementary,
    Triadic,
    Tetradic,
    Square,
    Analogous,
}

/// Color picker
pub struct ColorPicker {
    history: Arc<RwLock<Vec<Color>>>,
    palettes: Arc<RwLock<HashMap<String, ColorPalette>>>,
    favorites: Arc<RwLock<Vec<String>>>,
}

impl ColorPicker {
    /// Create new color picker
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(Vec::with_capacity(50))),
            palettes: Arc::new(RwLock::new(HashMap::new())),
            favorites: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add color to history
    pub async fn add_to_history(&self, color: Color) {
        let mut history = self.history.write().await;
        
        // Remove if already exists
        if let Some(pos) = history.iter().position(|c| 
            c.r == color.r && c.g == color.g && c.b == color.b
        ) {
            history.remove(pos);
        }

        history.insert(0, color);
        
        // Keep only last 50
        if history.len() > 50 {
            history.pop();
        }
    }

    /// Get color history
    pub async fn get_history(&self) -> Vec<Color> {
        self.history.read().await.clone()
    }

    /// Clear history
    pub async fn clear_history(&self) {
        self.history.write().await.clear();
    }

    /// Create palette
    pub async fn create_palette(&self, name: String, colors: Vec<Color>, tags: Vec<String>) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let palette = ColorPalette {
            id: id.clone(),
            name,
            colors,
            tags,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.palettes.write().await.insert(id.clone(), palette);
        id
    }

    /// Get palette
    pub async fn get_palette(&self, id: &str) -> Option<ColorPalette> {
        self.palettes.read().await.get(id).cloned()
    }

    /// List palettes
    pub async fn list_palettes(&self) -> Vec<ColorPalette> {
        self.palettes.read().await.values().cloned().collect()
    }

    /// Delete palette
    pub async fn delete_palette(&self, id: &str) {
        self.palettes.write().await.remove(id);
    }

    /// Add to favorites
    pub async fn add_favorite(&self, color_id: String) {
        self.favorites.write().await.push(color_id);
    }

    /// Remove from favorites
    pub async fn remove_favorite(&self, color_id: &str) {
        let mut favorites = self.favorites.write().await;
        favorites.retain(|id| id != color_id);
    }

    /// Get favorites
    pub async fn get_favorites(&self) -> Vec<String> {
        self.favorites.read().await.clone()
    }

    /// Generate harmony colors
    pub fn generate_harmony(&self, base: &Color, harmony: ColorHarmony) -> Vec<Color> {
        let (h, s, l) = base.to_hsl();

        match harmony {
            ColorHarmony::Monochromatic => {
                vec![
                    base.clone(),
                    Color::from_hsl(h, s, (l * 0.8).min(100.0), base.a),
                    Color::from_hsl(h, s, (l * 1.2).min(100.0), base.a),
                    Color::from_hsl(h, s * 0.8, l, base.a),
                    Color::from_hsl(h, s * 1.2, l, base.a),
                ]
            }
            ColorHarmony::Complementary => {
                vec![
                    base.clone(),
                    Color::from_hsl((h + 180.0) % 360.0, s, l, base.a),
                ]
            }
            ColorHarmony::SplitComplementary => {
                vec![
                    base.clone(),
                    Color::from_hsl((h + 150.0) % 360.0, s, l, base.a),
                    Color::from_hsl((h + 210.0) % 360.0, s, l, base.a),
                ]
            }
            ColorHarmony::Triadic => {
                vec![
                    base.clone(),
                    Color::from_hsl((h + 120.0) % 360.0, s, l, base.a),
                    Color::from_hsl((h + 240.0) % 360.0, s, l, base.a),
                ]
            }
            ColorHarmony::Tetradic => {
                vec![
                    base.clone(),
                    Color::from_hsl((h + 90.0) % 360.0, s, l, base.a),
                    Color::from_hsl((h + 180.0) % 360.0, s, l, base.a),
                    Color::from_hsl((h + 270.0) % 360.0, s, l, base.a),
                ]
            }
            ColorHarmony::Square => {
                vec![
                    base.clone(),
                    Color::from_hsl((h + 90.0) % 360.0, s, l, base.a),
                    Color::from_hsl((h + 180.0) % 360.0, s, l, base.a),
                    Color::from_hsl((h + 270.0) % 360.0, s, l, base.a),
                ]
            }
            ColorHarmony::Analogous => {
                vec![
                    Color::from_hsl((h - 30.0 + 360.0) % 360.0, s, l, base.a),
                    base.clone(),
                    Color::from_hsl((h + 30.0) % 360.0, s, l, base.a),
                ]
            }
        }
    }

    /// Parse color from string
    pub fn parse_color(&self, input: &str) -> Result<Color> {
        let input = input.trim();

        // Try hex
        if input.starts_with('#') || input.len() == 3 || input.len() == 6 || input.len() == 8 {
            return Color::from_hex(input);
        }

        // Try rgb/rgba
        if input.starts_with("rgb") {
            let re = regex::Regex::new(r"rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([0-9.]+))?\)")?;
            if let Some(caps) = re.captures(input) {
                let r = caps[1].parse()?;
                let g = caps[2].parse()?;
                let b = caps[3].parse()?;
                let a = caps.get(4).map_or(1.0, |m| m.as_str().parse().unwrap_or(1.0));
                return Ok(Color::new(r, g, b, a));
            }
        }

        // Try hsl/hsla
        if input.starts_with("hsl") {
            let re = regex::Regex::new(r"hsla?\((\d+),\s*(\d+)%,\s*(\d+)%(?:,\s*([0-9.]+))?\)")?;
            if let Some(caps) = re.captures(input) {
                let h = caps[1].parse()?;
                let s = caps[2].parse()?;
                let l = caps[3].parse()?;
                let a = caps.get(4).map_or(1.0, |m| m.as_str().parse().unwrap_or(1.0));
                return Ok(Color::from_hsl(h as f32, s as f32, l as f32, a));
            }
        }

        Err(DesignError::ColorError("Unable to parse color".to_string()))
    }

    /// Generate gradient
    pub fn generate_gradient(&self, colors: Vec<Color>, steps: usize) -> Gradient {
        let stops: Vec<GradientStop> = colors
            .into_iter()
            .enumerate()
            .map(|(i, color)| GradientStop {
                color,
                position: i as f32 / (colors.len() - 1) as f32,
            })
            .collect();

        Gradient::linear("Generated".to_string(), stops, 90.0)
    }

    /// Suggest accessible colors
    pub fn suggest_accessible(&self, base: &Color, level: WcagLevel, count: usize) -> Vec<Color> {
        let mut suggestions = Vec::new();
        let (h, s, l) = base.to_hsl();

        for i in 0..count {
            let test_l = (l + 20.0 * (i as f32 + 1.0)).min(100.0);
            let test = Color::from_hsl(h, s, test_l, base.a);
            
            if test.contrast_ratio(base) >= 4.5 {
                suggestions.push(test);
            }
        }

        for i in 0..count {
            let test_l = (l - 20.0 * (i as f32 + 1.0)).max(0.0);
            let test = Color::from_hsl(h, s, test_l, base.a);
            
            if test.contrast_ratio(base) >= 4.5 {
                suggestions.push(test);
            }
        }

        suggestions.truncate(count);
        suggestions
    }
}

impl Default for ColorPicker {
    fn default() -> Self {
        Self::new()
    }
}