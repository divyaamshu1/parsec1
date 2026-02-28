//! Dyslexia-friendly reading modes

use std::sync::Arc;

use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use regex::Regex;

#[cfg(feature = "dyslexia-friendly")]
use opendyslexic::OpenDyslexic;

use crate::{Result, AccessibilityError, AccessibilityConfig};

/// Dyslexia-friendly mode
pub struct DyslexiaMode {
    /// Is enabled
    enabled: Arc<RwLock<bool>>,
    /// Use OpenDyslexic font
    font_enabled: Arc<RwLock<bool>>,
    /// Reading guide enabled
    reading_guide: Arc<RwLock<bool>>,
    /// Line focus enabled
    line_focus: Arc<RwLock<bool>>,
    /// Syllable highlighting enabled
    syllable_highlight: Arc<RwLock<bool>>,
    /// Text spacing
    spacing: Arc<RwLock<TextSpacing>>,
    /// Reading guide settings
    guide_settings: Arc<RwLock<ReadingGuideSettings>>,
    /// Line focus settings
    focus_settings: Arc<RwLock<LineFocusSettings>>,
    /// Configuration
    config: AccessibilityConfig,
}

/// Text spacing
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TextSpacing {
    pub letter_spacing: f32,
    pub word_spacing: f32,
    pub line_height: f32,
    pub paragraph_spacing: f32,
}

impl Default for TextSpacing {
    fn default() -> Self {
        Self {
            letter_spacing: 0.05,
            word_spacing: 0.1,
            line_height: 1.5,
            paragraph_spacing: 1.0,
        }
    }
}

/// Reading guide settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingGuideSettings {
    pub enabled: bool,
    pub color: String,
    pub opacity: f32,
    pub height: f32,
    pub follow_cursor: bool,
}

impl Default for ReadingGuideSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            color: "#ffeb3b".to_string(),
            opacity: 0.3,
            height: 1.5,
            follow_cursor: true,
        }
    }
}

/// Line focus settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineFocusSettings {
    pub enabled: bool,
    pub lines_before: usize,
    pub lines_after: usize,
    pub dim_opacity: f32,
    pub highlight_current: bool,
}

impl Default for LineFocusSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            lines_before: 1,
            lines_after: 1,
            dim_opacity: 0.3,
            highlight_current: true,
        }
    }
}

/// Reading guide
pub struct ReadingGuide {
    /// Current position
    position: Arc<RwLock<f32>>,
    /// Settings
    settings: Arc<RwLock<ReadingGuideSettings>>,
}

impl ReadingGuide {
    /// Create new reading guide
    pub fn new() -> Self {
        Self {
            position: Arc::new(RwLock::new(0.0)),
            settings: Arc::new(RwLock::new(ReadingGuideSettings::default())),
        }
    }

    /// Set position
    pub async fn set_position(&self, y: f32) {
        *self.position.write().await = y;
    }

    /// Get position
    pub async fn position(&self) -> f32 {
        *self.position.read().await
    }

    /// Set settings
    pub async fn set_settings(&self, settings: ReadingGuideSettings) {
        *self.settings.write().await = settings;
    }

    /// Get settings
    pub async fn settings(&self) -> ReadingGuideSettings {
        self.settings.read().await.clone()
    }

    /// Check if position is in guide
    pub async fn in_guide(&self, y: f32, line_height: f32) -> bool {
        let pos = *self.position.read().await;
        let height = self.settings.read().await.height * line_height;
        y >= pos - height / 2.0 && y <= pos + height / 2.0
    }
}

/// Line focus
pub struct LineFocus {
    /// Current line
    current_line: Arc<RwLock<usize>>,
    /// Settings
    settings: Arc<RwLock<LineFocusSettings>>,
}

impl LineFocus {
    /// Create new line focus
    pub fn new() -> Self {
        Self {
            current_line: Arc::new(RwLock::new(0)),
            settings: Arc::new(RwLock::new(LineFocusSettings::default())),
        }
    }

    /// Set current line
    pub async fn set_current_line(&self, line: usize) {
        *self.current_line.write().await = line;
    }

    /// Get current line
    pub async fn current_line(&self) -> usize {
        *self.current_line.read().await
    }

    /// Set settings
    pub async fn set_settings(&self, settings: LineFocusSettings) {
        *self.settings.write().await = settings;
    }

    /// Get settings
    pub async fn settings(&self) -> LineFocusSettings {
        self.settings.read().await.clone()
    }

    /// Check if line is focused
    pub async fn is_focused(&self, line: usize) -> bool {
        let current = *self.current_line.read().await;
        let settings = self.settings.read().await;
        
        if !settings.enabled {
            return true;
        }

        let diff = if line > current { line - current } else { current - line };
        diff <= settings.lines_before || diff <= settings.lines_after
    }
}

/// Syllable highlighter
pub struct SyllableHighlighter {
    /// Regex for syllable detection
    syllable_regex: Regex,
    /// Highlight color
    highlight_color: String,
}

impl SyllableHighlighter {
    /// Create new syllable highlighter
    pub fn new() -> Result<Self> {
        // Simple syllable detection regex (English)
        // In production, use proper syllable detection library
        let regex = Regex::new(r"[aeiouy]+[^aeiouy]*")
            .map_err(|e| AccessibilityError::FontError(format!("Regex error: {}", e)))?;

        Ok(Self {
            syllable_regex: regex,
            highlight_color: "#ff9800".to_string(),
        })
    }

    /// Highlight syllables in text
    pub fn highlight_syllables(&self, text: &str) -> Vec<Syllable> {
        let mut syllables = Vec::new();
        let mut last_end = 0;

        for mat in self.syllable_regex.find_iter(text) {
            syllables.push(Syllable {
                text: mat.as_str().to_string(),
                start: mat.start(),
                end: mat.end(),
                highlight: true,
            });
            last_end = mat.end();
        }

        // Add remaining text as non-syllable
        if last_end < text.len() {
            syllables.push(Syllable {
                text: text[last_end..].to_string(),
                start: last_end,
                end: text.len(),
                highlight: false,
            });
        }

        syllables
    }
}

/// Syllable
#[derive(Debug, Clone)]
pub struct Syllable {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub highlight: bool,
}

/// OpenDyslexic font wrapper
#[cfg(feature = "dyslexia-friendly")]
pub struct OpenDyslexicFont {
    /// Font data
    font_data: Vec<u8>,
    /// Is loaded
    loaded: bool,
}

#[cfg(feature = "dyslexia-friendly")]
impl OpenDyslexicFont {
    /// Create new OpenDyslexic font
    pub fn new() -> Result<Self> {
        let font_data = include_bytes!("../fonts/OpenDyslexic-Regular.otf").to_vec();
        
        Ok(Self {
            font_data,
            loaded: true,
        })
    }

    /// Get font data
    pub fn data(&self) -> &[u8] {
        &self.font_data
    }

    /// Apply font to CSS
    pub fn to_css(&self) -> String {
        String::from(r#"
            @font-face {
                font-family: 'OpenDyslexic';
                src: url('fonts/OpenDyslexic-Regular.otf') format('opentype');
                font-weight: normal;
                font-style: normal;
            }
            * {
                font-family: 'OpenDyslexic', sans-serif !important;
            }
        "#)
    }
}

impl DyslexiaMode {
    /// Create new dyslexia mode
    pub async fn new(config: AccessibilityConfig) -> Result<Self> {
        Ok(Self {
            enabled: Arc::new(RwLock::new(false)),
            font_enabled: Arc::new(RwLock::new(false)),
            reading_guide: Arc::new(RwLock::new(true)),
            line_focus: Arc::new(RwLock::new(true)),
            syllable_highlight: Arc::new(RwLock::new(false)),
            spacing: Arc::new(RwLock::new(TextSpacing::default())),
            guide_settings: Arc::new(RwLock::new(ReadingGuideSettings::default())),
            focus_settings: Arc::new(RwLock::new(LineFocusSettings::default())),
            config,
        })
    }

    /// Enable dyslexia mode
    pub async fn enable(&self) {
        *self.enabled.write().await = true;
    }

    /// Disable dyslexia mode
    pub async fn disable(&self) {
        *self.enabled.write().await = false;
    }

    /// Check if enabled
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// Set font enabled
    pub async fn set_font_enabled(&self, enabled: bool) {
        *self.font_enabled.write().await = enabled;
    }

    /// Check if font enabled
    pub async fn font_enabled(&self) -> bool {
        *self.font_enabled.read().await
    }

    /// Set reading guide enabled
    pub async fn set_reading_guide(&self, enabled: bool) {
        *self.reading_guide.write().await = enabled;
    }

    /// Check if reading guide enabled
    pub async fn reading_guide_enabled(&self) -> bool {
        *self.reading_guide.read().await
    }

    /// Set line focus enabled
    pub async fn set_line_focus(&self, enabled: bool) {
        *self.line_focus.write().await = enabled;
    }

    /// Check if line focus enabled
    pub async fn line_focus_enabled(&self) -> bool {
        *self.line_focus.read().await
    }

    /// Set syllable highlight enabled
    pub async fn set_syllable_highlight(&self, enabled: bool) {
        *self.syllable_highlight.write().await = enabled;
    }

    /// Check if syllable highlight enabled
    pub async fn syllable_highlight_enabled(&self) -> bool {
        *self.syllable_highlight.read().await
    }

    /// Set text spacing
    pub async fn set_spacing(&self, spacing: TextSpacing) {
        *self.spacing.write().await = spacing;
    }

    /// Get text spacing
    pub async fn spacing(&self) -> TextSpacing {
        *self.spacing.read().await
    }

    /// Apply spacing to CSS
    pub fn spacing_to_css(&self) -> String {
        let spacing = self.spacing.blocking_read();
        format!(
            r#"
            * {{
                letter-spacing: {}em;
                word-spacing: {}em;
                line-height: {};
                margin-bottom: {}em;
            }}
            "#,
            spacing.letter_spacing,
            spacing.word_spacing,
            spacing.line_height,
            spacing.paragraph_spacing
        )
    }

    /// Get reading guide
    pub fn reading_guide(&self) -> ReadingGuide {
        ReadingGuide::new()
    }

    /// Get line focus
    pub fn line_focus(&self) -> LineFocus {
        LineFocus::new()
    }

    /// Get syllable highlighter
    pub fn syllable_highlighter(&self) -> Result<SyllableHighlighter> {
        SyllableHighlighter::new()
    }

    /// Get OpenDyslexic font
    #[cfg(feature = "dyslexia-friendly")]
    pub fn opendyslexic_font(&self) -> Option<OpenDyslexicFont> {
        if *self.font_enabled.blocking_read() {
            OpenDyslexicFont::new().ok()
        } else {
            None
        }
    }
}