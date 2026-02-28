//! Parsec Accessibility Features
//!
//! Comprehensive accessibility tools including:
//! - Screen reader integration
//! - High contrast themes
//! - Voice control
//! - Keyboard navigation
//! - Dyslexia-friendly modes
//! - Color blindness simulation
//! - Motion reduction

#![allow(dead_code, unused_imports)]

#[cfg(feature = "screen-reader")]
pub mod screen_reader;
#[cfg(feature = "high-contrast")]
pub mod high_contrast;
#[cfg(feature = "voice-control")]
pub mod voice_control;
pub mod keyboard_nav;
#[cfg(feature = "dyslexia-friendly")]
pub mod dyslexia;
#[cfg(feature = "color-blind")]
pub mod color_blind;
pub mod motion;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast, mpsc};
use serde::{Serialize, Deserialize};
use strum::{EnumIter, Display};

// Re-exports
pub use screen_reader::{
    ScreenReader, SpeechOutput, SpeechPriority, Voice, SpeechRate, SpeechPitch,
    ScreenReaderMode, ReadingOptions
};
pub use high_contrast::{
    HighContrastTheme, ContrastLevel, ThemePreset, ColorAdjustment,
    HighContrastManager
};
#[cfg(feature = "voice-control")]
pub use voice_control::{
    VoiceControl, VoiceCommand, CommandContext, VoiceProfile, WakeWord,
    SpeechRecognizer, VoiceFeedback
};
#[cfg(feature = "keyboard-nav")]
pub use keyboard_nav::{
    KeyboardNavigation, NavigationMode, FocusIndicator, ShortcutManager,
    KeyBinding, NavigationHistory
};
#[cfg(feature = "dyslexia-friendly")]
pub use dyslexia::{
    DyslexiaMode, OpenDyslexicFont, ReadingGuide, LineFocus, SyllableHighlight,
    TextSpacing
};
#[cfg(feature = "color-blind")]
pub use color_blind::{
    ColorBlindMode, ColorBlindType, ColorSimulation, ColorCorrection,
    SimulationStrength
};
pub use motion::{
    MotionReduction, AnimationLevel, TransitionSpeed, ParallaxEffect,
    AutoPlayMedia
};

/// Result type for accessibility operations
pub type Result<T> = std::result::Result<T, AccessibilityError>;

/// Accessibility error
#[derive(Debug, thiserror::Error)]
pub enum AccessibilityError {
    #[error("Screen reader error: {0}")]
    ScreenReaderError(String),

    #[error("TTS error: {0}")]
    TtsError(String),

    #[error("Voice recognition error: {0}")]
    VoiceError(String),

    #[error("Microphone error: {0}")]
    MicrophoneError(String),

    #[error("Theme error: {0}")]
    ThemeError(String),

    #[error("Keyboard navigation error: {0}")]
    KeyboardError(String),

    #[error("Font error: {0}")]
    FontError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Channel send error: {0}")]
    ChannelSend(String),

    #[error("IO error: {0}")]
    Io(#[from] futures::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Accessibility profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityProfile {
    pub id: String,
    pub name: String,
    pub screen_reader_enabled: bool,
    pub screen_reader_mode: ScreenReaderMode,
    pub high_contrast_enabled: bool,
    pub high_contrast_theme: Option<String>,
    pub voice_control_enabled: bool,
    pub wake_word: Option<String>,
    pub keyboard_nav_enabled: bool,
    #[cfg(feature = "keyboard-nav")]
    pub navigation_mode: NavigationMode,
    pub dyslexia_mode_enabled: bool,
    pub dyslexia_font: bool,
    #[cfg(feature = "color-blind")]
    pub color_blind_mode: Option<ColorBlindMode>,
    pub motion_reduction_enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Default for AccessibilityProfile {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Default".to_string(),
            screen_reader_enabled: false,
            screen_reader_mode: ScreenReaderMode::OnDemand,
            high_contrast_enabled: false,
            high_contrast_theme: None,
            voice_control_enabled: false,
            wake_word: None,
            keyboard_nav_enabled: true,
            #[cfg(feature = "keyboard-nav")]
            navigation_mode: NavigationMode::Standard,
            dyslexia_mode_enabled: false,
            dyslexia_font: false,
            #[cfg(feature = "color-blind")]
            color_blind_mode: None,
            motion_reduction_enabled: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}

/// Accessibility event
#[derive(Debug, Clone)]
pub enum AccessibilityEvent {
    ScreenReaderStateChanged(bool),
    SpeechStarted(String),
    SpeechCompleted(String),
    #[cfg(feature = "voice-control")]
    VoiceCommandRecognized(VoiceCommand),
    ThemeChanged(String),
    FocusMoved(String),
    ProfileLoaded(String),
    ProfileSaved(String),
    Error(String),
}

/// Accessibility configuration
#[derive(Debug, Clone)]
pub struct AccessibilityConfig {
    /// Profiles directory
    pub profiles_dir: PathBuf,
    /// Enable screen reader
    pub enable_screen_reader: bool,
    /// Default screen reader mode
    pub default_screen_reader_mode: ScreenReaderMode,
    /// Enable high contrast
    pub enable_high_contrast: bool,
    /// Default high contrast theme
    pub default_high_contrast_theme: String,
    /// Enable voice control
    pub enable_voice_control: bool,
    /// Default wake word
    pub default_wake_word: Option<String>,
    /// Voice recognition language
    pub voice_language: String,
    /// Enable keyboard navigation
    pub enable_keyboard_nav: bool,
    #[cfg(feature = "keyboard-nav")]
    /// Default navigation mode
    pub default_navigation_mode: NavigationMode,
    /// Enable dyslexia features
    pub enable_dyslexia: bool,
    /// Enable color blind features
    pub enable_color_blind: bool,
    /// Enable motion reduction
    pub enable_motion: bool,
    /// Speech rate (words per minute)
    pub speech_rate: u32,
    /// Speech pitch
    pub speech_pitch: f32,
}

impl Default for AccessibilityConfig {
    fn default() -> Self {
        let data_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("parsec");

        Self {
            profiles_dir: data_dir.join("accessibility").join("profiles"),
            enable_screen_reader: true,
            default_screen_reader_mode: ScreenReaderMode::OnDemand,
            enable_high_contrast: true,
            default_high_contrast_theme: "high-contrast-dark".to_string(),
            enable_voice_control: false,
            default_wake_word: Some("hey parsec".to_string()),
            voice_language: "en-US".to_string(),
            enable_keyboard_nav: true,
            #[cfg(feature = "keyboard-nav")]
            default_navigation_mode: NavigationMode::Standard,
            enable_dyslexia: true,
            enable_color_blind: true,
            enable_motion: true,
            speech_rate: 180,
            speech_pitch: 1.0,
        }
    }
}

/// Main accessibility engine
pub struct AccessibilityEngine {
    /// Configuration
    config: AccessibilityConfig,
    /// Screen reader
    #[cfg(feature = "screen-reader")]
    screen_reader: Arc<screen_reader::ScreenReader>,
    /// High contrast manager
    high_contrast: Arc<high_contrast::HighContrastManager>,
    /// Voice control
    #[cfg(feature = "voice-control")]
    voice_control: Arc<voice_control::VoiceControl>,
    /// Keyboard navigation
    #[cfg(feature = "keyboard-nav")]
    keyboard_nav: Arc<keyboard_nav::KeyboardNavigation>,
    /// Dyslexia mode
    #[cfg(feature = "dyslexia-friendly")]
    dyslexia: Arc<dyslexia::DyslexiaMode>,
    /// Color blind simulator
    #[cfg(feature = "color-blind")]
    color_blind: Arc<color_blind::ColorBlindSimulator>,
    /// Motion reduction
    motion: Arc<motion::MotionReduction>,
    /// Current profile
    current_profile: Arc<RwLock<AccessibilityProfile>>,
    /// Profiles
    profiles: Arc<RwLock<HashMap<String, AccessibilityProfile>>>,
    /// Event broadcaster
    event_tx: broadcast::Sender<AccessibilityEvent>,
    event_rx: broadcast::Receiver<AccessibilityEvent>,
}

impl AccessibilityEngine {
    /// Create new accessibility engine
    pub async fn new(config: AccessibilityConfig) -> Result<Self> {
        let (event_tx, event_rx) = broadcast::channel(100);

        // Create profiles directory
        tokio::fs::create_dir_all(&config.profiles_dir).await?;

        let engine = Self {
            #[cfg(feature = "screen-reader")]
            screen_reader: Arc::new(screen_reader::ScreenReader::new(config.clone()).await?),
            high_contrast: Arc::new(high_contrast::HighContrastManager::new(config.clone()).await?),
            #[cfg(feature = "voice-control")]
            voice_control: Arc::new(voice_control::VoiceControl::new(config.clone()).await?),
            #[cfg(feature = "keyboard-nav")]
            keyboard_nav: Arc::new(keyboard_nav::KeyboardNavigation::new(config.clone()).await?),
            #[cfg(feature = "dyslexia-friendly")]
            dyslexia: Arc::new(dyslexia::DyslexiaMode::new(config.clone()).await?),
            #[cfg(feature = "color-blind")]
            color_blind: Arc::new(color_blind::ColorBlindSimulator::new(config.clone()).await?),
            motion: Arc::new(motion::MotionReduction::new(config.clone()).await?),
            config: config.clone(),
            current_profile: Arc::new(RwLock::new(AccessibilityProfile::default())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx,
        };

        // Load profiles
        engine.load_profiles().await?;

        // Load default profile
        engine.load_profile("default").await.ok();

        Ok(engine)
    }

    /// Get screen reader
    pub fn screen_reader(&self) -> Arc<screen_reader::ScreenReader> {
        self.screen_reader.clone()
    }

    /// Get high contrast manager
    pub fn high_contrast(&self) -> Arc<high_contrast::HighContrastManager> {
        self.high_contrast.clone()
    }

    /// Access the voice control manager
    #[cfg(feature = "voice-control")]
    pub fn get_voice_control(&self) -> Arc<voice_control::VoiceControl> {
        self.voice_control.clone()
    }

    /// Get keyboard navigation
    #[cfg(feature = "keyboard-nav")]
    pub fn keyboard_nav(&self) -> Arc<keyboard_nav::KeyboardNavigation> {
        self.keyboard_nav.clone()
    }

    /// Access dyslexia mode handler
    #[cfg(feature = "dyslexia-friendly")]
    pub fn get_dyslexia(&self) -> Arc<dyslexia::DyslexiaMode> {
        self.dyslexia.clone()
    }

    /// Access color blind simulator
    #[cfg(feature = "color-blind")]
    pub fn get_color_blind(&self) -> Arc<color_blind::ColorBlindSimulator> {
        self.color_blind.clone()
    }

    /// Get motion reduction
    pub fn motion(&self) -> Arc<motion::MotionReduction> {
        self.motion.clone()
    }

    /// Get current profile
    pub async fn current_profile(&self) -> AccessibilityProfile {
        self.current_profile.read().await.clone()
    }

    /// Set current profile
    pub async fn set_current_profile(&self, profile: AccessibilityProfile) {
        *self.current_profile.write().await = profile.clone();
        self.apply_profile(&profile).await;
        let _ = self.event_tx.send(AccessibilityEvent::ProfileLoaded(profile.id));
    }

    /// Apply profile settings
    async fn apply_profile(&self, profile: &AccessibilityProfile) {
        // Apply screen reader
        #[cfg(feature = "screen-reader")]
        {
            if profile.screen_reader_enabled {
                self.screen_reader.enable().await;
                self.screen_reader.set_mode(profile.screen_reader_mode).await;
            } else {
                self.screen_reader.disable().await;
            }
        }

        // Apply high contrast
        #[cfg(feature = "high-contrast")]
        {
            if profile.high_contrast_enabled {
                if let Some(theme) = &profile.high_contrast_theme {
                    let _ = self.high_contrast.set_theme(theme).await;
                }
                self.high_contrast.enable().await;
            } else {
                self.high_contrast.disable().await;
            }
        }

        // Apply voice control
        #[cfg(feature = "voice-control")]
        {
            if profile.voice_control_enabled {
                self.voice_control.enable().await;
                if let Some(wake) = &profile.wake_word {
                    self.voice_control.set_wake_word(wake).await;
                }
            } else {
                self.voice_control.disable().await;
            }
        }

        // Apply keyboard navigation
        #[cfg(feature = "keyboard-nav")]
        {
            self.keyboard_nav.set_mode(profile.navigation_mode).await;
            if profile.keyboard_nav_enabled {
                self.keyboard_nav.enable().await;
            } else {
                self.keyboard_nav.disable().await;
            }
        }

        // Apply dyslexia mode
        #[cfg(feature = "dyslexia-friendly")]
        {
            if profile.dyslexia_mode_enabled {
                self.dyslexia.enable().await;
                self.dyslexia.set_font_enabled(profile.dyslexia_font).await;
            } else {
                self.dyslexia.disable().await;
            }
        }

        // Apply color blind mode
        #[cfg(feature = "color-blind")]
        {
            if let Some(mode) = profile.color_blind_mode {
                self.color_blind.set_mode(mode).await;
                self.color_blind.enable().await;
            } else {
                self.color_blind.disable().await;
            }
        }

        // Apply motion reduction
        if profile.motion_reduction_enabled {
            self.motion.enable().await;
        } else {
            self.motion.disable().await;
        }
    }

    /// Load profiles from disk
    async fn load_profiles(&self) -> Result<()> {
        let mut profiles = self.profiles.write().await;
        profiles.clear();

        let mut read_dir = tokio::fs::read_dir(&self.config.profiles_dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(profile) = serde_json::from_str::<AccessibilityProfile>(&content) {
                        profiles.insert(profile.id.clone(), profile);
                    }
                }
            }
        }

        Ok(())
    }

    /// Load profile by ID
    pub async fn load_profile(&self, id: &str) -> Result<AccessibilityProfile> {
        let profiles = self.profiles.read().await;
        if let Some(profile) = profiles.get(id) {
            self.set_current_profile(profile.clone()).await;
            Ok(profile.clone())
        } else {
            // Try to load from file
            let path = self.config.profiles_dir.join(format!("{}.json", id));
            if path.exists() {
                let content = tokio::fs::read_to_string(path).await?;
                let profile: AccessibilityProfile = serde_json::from_str(&content)?;
                self.profiles.write().await.insert(profile.id.clone(), profile.clone());
                self.set_current_profile(profile.clone()).await;
                Ok(profile)
            } else {
                Err(AccessibilityError::ConfigError(format!("Profile not found: {}", id)))
            }
        }
    }

    /// Save profile
    pub async fn save_profile(&self, profile: &AccessibilityProfile) -> Result<()> {
        let path = self.config.profiles_dir.join(format!("{}.json", profile.id));
        let json = serde_json::to_string_pretty(profile)?;
        tokio::fs::write(path, json).await?;

        self.profiles.write().await.insert(profile.id.clone(), profile.clone());
        let _ = self.event_tx.send(AccessibilityEvent::ProfileSaved(profile.id.clone()));

        Ok(())
    }

    /// Delete profile
    pub async fn delete_profile(&self, id: &str) -> Result<()> {
        let path = self.config.profiles_dir.join(format!("{}.json", id));
        if path.exists() {
            tokio::fs::remove_file(path).await?;
        }
        self.profiles.write().await.remove(id);
        Ok(())
    }

    /// List profiles
    pub async fn list_profiles(&self) -> Vec<AccessibilityProfile> {
        self.profiles.read().await.values().cloned().collect()
    }

    /// Create default profile
    pub async fn create_default_profile(&self) -> AccessibilityProfile {
        let profile = AccessibilityProfile::default();
        let _ = self.save_profile(&profile).await;
        profile
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<AccessibilityEvent> {
        self.event_tx.subscribe()
    }

    /// Check if any accessibility feature is enabled
    pub async fn is_any_enabled(&self) -> bool {
        let profile = self.current_profile.read().await;
        profile.screen_reader_enabled
            || profile.high_contrast_enabled
            || profile.dyslexia_mode_enabled
            || profile.motion_reduction_enabled
            || (cfg!(feature = "voice-control") && profile.voice_control_enabled)
            || {
                #[cfg(feature = "color-blind")]
                {
                    profile.color_blind_mode.is_some()
                }
                #[cfg(not(feature = "color-blind"))]
                {
                    false
                }
            }
            || (cfg!(feature = "keyboard-nav") && profile.keyboard_nav_enabled)
    }

    /// Get accessibility status
    pub async fn status(&self) -> AccessibilityStatus {
        let profile = self.current_profile.read().await;
        AccessibilityStatus {
            screen_reader: profile.screen_reader_enabled,
            high_contrast: profile.high_contrast_enabled,
            #[cfg(feature = "voice-control")]
            voice_control: profile.voice_control_enabled,
            #[cfg(not(feature = "voice-control"))]
            voice_control: false,
            #[cfg(feature = "keyboard-nav")]
            keyboard_nav: profile.keyboard_nav_enabled,
            #[cfg(not(feature = "keyboard-nav"))]
            keyboard_nav: false,
            dyslexia_mode: profile.dyslexia_mode_enabled,
            #[cfg(feature = "color-blind")]
            color_blind: profile.color_blind_mode.is_some(),
            #[cfg(not(feature = "color-blind"))]
            color_blind: false,
            motion_reduction: profile.motion_reduction_enabled,
        }
    }
}

/// Accessibility status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityStatus {
    pub screen_reader: bool,
    pub high_contrast: bool,
    pub voice_control: bool,
    pub keyboard_nav: bool,
    pub dyslexia_mode: bool,
    pub color_blind: bool,
    pub motion_reduction: bool,
}