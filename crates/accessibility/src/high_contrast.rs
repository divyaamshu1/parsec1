//! High contrast themes for visual impairment

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use css_color::Rgba;
use std::str::FromStr;
use palette::{Srgb, FromColor, Hsla, Lcha};

use crate::{Result, AccessibilityError, AccessibilityConfig};

/// Contrast level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContrastLevel {
    Standard,
    Enhanced,
    Maximum,
}

/// Theme preset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemePreset {
    HighContrastLight,
    HighContrastDark,
    HighContrastCustom,
    Inverted,
    Monochrome,
}

/// Color adjustment
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ColorAdjustment {
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub hue_shift: f32,
    pub invert: bool,
}

impl Default for ColorAdjustment {
    fn default() -> Self {
        Self {
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            hue_shift: 0.0,
            invert: false,
        }
    }
}

/// High contrast theme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighContrastTheme {
    pub id: String,
    pub name: String,
    pub preset: ThemePreset,
    pub background: String,
    pub foreground: String,
    pub primary: String,
    pub secondary: String,
    pub accent: String,
    pub success: String,
    pub warning: String,
    pub error: String,
    pub info: String,
    pub selection: String,
    pub line_highlight: String,
    pub border: String,
    pub shadow: String,
    pub contrast_level: ContrastLevel,
    pub adjustment: ColorAdjustment,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Default for HighContrastTheme {
    fn default() -> Self {
        Self::high_contrast_dark()
    }
}

impl HighContrastTheme {
    /// Create high contrast dark theme
    pub fn high_contrast_dark() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "High Contrast Dark".to_string(),
            preset: ThemePreset::HighContrastDark,
            background: "#000000".to_string(),
            foreground: "#ffffff".to_string(),
            primary: "#ffff00".to_string(),
            secondary: "#00ff00".to_string(),
            accent: "#00ffff".to_string(),
            success: "#00ff00".to_string(),
            warning: "#ffff00".to_string(),
            error: "#ff0000".to_string(),
            info: "#00ffff".to_string(),
            selection: "#0000ff".to_string(),
            line_highlight: "#333333".to_string(),
            border: "#ffffff".to_string(),
            shadow: "#ffffff".to_string(),
            contrast_level: ContrastLevel::Maximum,
            adjustment: ColorAdjustment::default(),
            created_at: chrono::Utc::now(),
        }
    }

    /// Create high contrast light theme
    pub fn high_contrast_light() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "High Contrast Light".to_string(),
            preset: ThemePreset::HighContrastLight,
            background: "#ffffff".to_string(),
            foreground: "#000000".to_string(),
            primary: "#0000ff".to_string(),
            secondary: "#008000".to_string(),
            accent: "#ff00ff".to_string(),
            success: "#008000".to_string(),
            warning: "#ffff00".to_string(),
            error: "#ff0000".to_string(),
            info: "#0000ff".to_string(),
            selection: "#add8e6".to_string(),
            line_highlight: "#f0f0f0".to_string(),
            border: "#000000".to_string(),
            shadow: "#000000".to_string(),
            contrast_level: ContrastLevel::Maximum,
            adjustment: ColorAdjustment::default(),
            created_at: chrono::Utc::now(),
        }
    }

    /// Create inverted theme
    pub fn inverted(base: &HighContrastTheme) -> Self {
        let mut theme = base.clone();
        theme.preset = ThemePreset::Inverted;
        theme.name = format!("{} (Inverted)", theme.name);
        theme.adjustment.invert = true;
        theme
    }

    /// Apply color adjustment to a color
    pub fn adjust_color(&self, color_str: &str) -> Result<String> {
        let rgba = css_color::Rgba::from_str(color_str)
            .map_err(|_| AccessibilityError::ThemeError("Invalid color".to_string()))?;

        let mut r = rgba.red as f32;
        let mut g = rgba.green as f32;
        let mut b = rgba.blue as f32;
        let a = rgba.alpha as f32;

        // Apply adjustments
        if self.adjustment.invert {
            r = 1.0 - r;
            g = 1.0 - g;
            b = 1.0 - b;
        }

        // Apply brightness
        r *= self.adjustment.brightness;
        g *= self.adjustment.brightness;
        b *= self.adjustment.brightness;

        // Apply contrast
        r = (r - 0.5) * self.adjustment.contrast + 0.5;
        g = (g - 0.5) * self.adjustment.contrast + 0.5;
        b = (b - 0.5) * self.adjustment.contrast + 0.5;

        // Apply saturation
        let gray = r * 0.299 + g * 0.587 + b * 0.114;
        r = gray + (r - gray) * self.adjustment.saturation;
        g = gray + (g - gray) * self.adjustment.saturation;
        b = gray + (b - gray) * self.adjustment.saturation;

        // Apply hue shift
        if self.adjustment.hue_shift != 0.0 {
            let rgb = Srgb::new(r, g, b);
            let mut lcha = Lcha::from_color(rgb);
            lcha.hue += self.adjustment.hue_shift;
            let rgb: Srgb = Srgb::from_color(lcha);
            r = rgb.red;
            g = rgb.green;
            b = rgb.blue;
        }

        // Clamp values
        r = r.clamp(0.0, 1.0);
        g = g.clamp(0.0, 1.0);
        b = b.clamp(0.0, 1.0);

        let rgba = Rgba::new(r, g, b, a);
        // convert RGBA components back to hex string (#RRGGBBAA)
        let r_u8 = (rgba.red.clamp(0.0,1.0) * 255.0) as u8;
        let g_u8 = (rgba.green.clamp(0.0,1.0) * 255.0) as u8;
        let b_u8 = (rgba.blue.clamp(0.0,1.0) * 255.0) as u8;
        let a_u8 = (rgba.alpha.clamp(0.0,1.0) * 255.0) as u8;
        Ok(format!("#{:02X}{:02X}{:02X}{:02X}", r_u8, g_u8, b_u8, a_u8))
    }

    /// Check contrast ratio between two colors
    pub fn contrast_ratio(&self, color1: &str, color2: &str) -> Result<f32> {
        let rgba1 = css_color::Rgba::from_str(color1)
            .map_err(|_| AccessibilityError::ThemeError("Invalid color".to_string()))?;
        let rgba2 = css_color::Rgba::from_str(color2)
            .map_err(|_| AccessibilityError::ThemeError("Invalid color".to_string()))?;

        let l1 = self.luminance(rgba1);
        let l2 = self.luminance(rgba2);

        let lighter = l1.max(l2);
        let darker = l1.min(l2);

        Ok((lighter + 0.05) / (darker + 0.05))
    }

    /// Calculate relative luminance
    fn luminance(&self, rgba: Rgba) -> f32 {
        let r = self.gamma_decode(rgba.red);
        let g = self.gamma_decode(rgba.green);
        let b = self.gamma_decode(rgba.blue);

        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// Gamma decode color component
    fn gamma_decode(&self, c: f32) -> f32 {
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    /// Check WCAG compliance
    pub fn meets_wcag_aa(&self, foreground: &str, background: &str) -> Result<bool> {
        let ratio = self.contrast_ratio(foreground, background)?;
        Ok(ratio >= 4.5)
    }

    /// Check WCAG AAA compliance
    pub fn meets_wcag_aaa(&self, foreground: &str, background: &str) -> Result<bool> {
        let ratio = self.contrast_ratio(foreground, background)?;
        Ok(ratio >= 7.0)
    }
}

/// High contrast manager
pub struct HighContrastManager {
    /// Is enabled
    enabled: Arc<RwLock<bool>>,
    /// Available themes
    themes: Arc<RwLock<HashMap<String, HighContrastTheme>>>,
    /// Current theme ID
    current_theme: Arc<RwLock<Option<String>>>,
    /// Configuration
    config: AccessibilityConfig,
}

impl HighContrastManager {
    /// Create new high contrast manager
    pub async fn new(config: AccessibilityConfig) -> Result<Self> {
        let mut themes = HashMap::new();

        // Add default themes
        let dark = HighContrastTheme::high_contrast_dark();
        themes.insert(dark.id.clone(), dark);

        let light = HighContrastTheme::high_contrast_light();
        themes.insert(light.id.clone(), light);

        Ok(Self {
            enabled: Arc::new(RwLock::new(false)),
            themes: Arc::new(RwLock::new(themes)),
            current_theme: Arc::new(RwLock::new(None)),
            config,
        })
    }

    /// Enable high contrast
    pub async fn enable(&self) {
        *self.enabled.write().await = true;
    }

    /// Disable high contrast
    pub async fn disable(&self) {
        *self.enabled.write().await = false;
    }

    /// Check if enabled
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// Add theme
    pub async fn add_theme(&self, theme: HighContrastTheme) {
        self.themes.write().await.insert(theme.id.clone(), theme);
    }

    /// Remove theme
    pub async fn remove_theme(&self, id: &str) {
        self.themes.write().await.remove(id);
    }

    /// Set current theme
    pub async fn set_theme(&self, id: &str) -> Result<()> {
        let themes = self.themes.read().await;
        if themes.contains_key(id) {
            *self.current_theme.write().await = Some(id.to_string());
            Ok(())
        } else {
            Err(AccessibilityError::ThemeError(format!("Theme not found: {}", id)))
        }
    }

    /// Get current theme
    pub async fn current_theme(&self) -> Option<HighContrastTheme> {
        let current = self.current_theme.read().await.clone();
        if let Some(id) = current {
            self.themes.read().await.get(&id).cloned()
        } else {
            None
        }
    }

    /// List themes
    pub async fn list_themes(&self) -> Vec<HighContrastTheme> {
        self.themes.read().await.values().cloned().collect()
    }

    /// Get theme by ID
    pub async fn get_theme(&self, id: &str) -> Option<HighContrastTheme> {
        self.themes.read().await.get(id).cloned()
    }

    /// Apply theme to CSS
    pub async fn to_css(&self, theme: &HighContrastTheme) -> String {
        let mut css = String::new();

        css.push_str(":root {\n");
        css.push_str(&format!("  --background: {};\n", theme.background));
        css.push_str(&format!("  --foreground: {};\n", theme.foreground));
        css.push_str(&format!("  --primary: {};\n", theme.primary));
        css.push_str(&format!("  --secondary: {};\n", theme.secondary));
        css.push_str(&format!("  --accent: {};\n", theme.accent));
        css.push_str(&format!("  --success: {};\n", theme.success));
        css.push_str(&format!("  --warning: {};\n", theme.warning));
        css.push_str(&format!("  --error: {};\n", theme.error));
        css.push_str(&format!("  --info: {};\n", theme.info));
        css.push_str(&format!("  --selection: {};\n", theme.selection));
        css.push_str(&format!("  --line-highlight: {};\n", theme.line_highlight));
        css.push_str(&format!("  --border: {};\n", theme.border));
        css.push_str(&format!("  --shadow: {};\n", theme.shadow));
        css.push_str("}\n");

        css
    }

    /// Create custom theme
    pub fn create_custom_theme(&self, name: String, base: &HighContrastTheme) -> HighContrastTheme {
        let mut theme = base.clone();
        theme.id = uuid::Uuid::new_v4().to_string();
        theme.name = name;
        theme.preset = ThemePreset::HighContrastCustom;
        theme.created_at = chrono::Utc::now();
        theme
    }

    /// Adjust theme
    pub fn adjust_theme(&self, theme: &HighContrastTheme, adjustment: ColorAdjustment) -> HighContrastTheme {
        let mut theme = theme.clone();
        theme.adjustment = adjustment;
        theme
    }
}