//! Motion reduction for vestibular disorders

use std::sync::Arc;

use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

use crate::{Result, AccessibilityError, AccessibilityConfig};

/// Animation level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimationLevel {
    /// All animations enabled
    Full,
    /// Reduced animations
    Reduced,
    /// No animations
    None,
    /// Essential animations only
    Essential,
}

/// Transition speed
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TransitionSpeed {
    Normal,
    Slow,
    Fast,
    Instant,
}

/// Parallax effect
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParallaxEffect {
    /// Full parallax
    Full,
    /// Reduced parallax
    Reduced,
    /// No parallax
    None,
}

/// Auto-play media
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoPlayMedia {
    /// Auto-play all
    All,
    /// Auto-play only videos without audio
    VideosWithoutAudio,
    /// No auto-play
    None,
}

/// Motion reduction
pub struct MotionReduction {
    /// Is enabled
    enabled: Arc<RwLock<bool>>,
    /// Animation level
    animation_level: Arc<RwLock<AnimationLevel>>,
    /// Transition speed
    transition_speed: Arc<RwLock<TransitionSpeed>>,
    /// Parallax effect
    parallax: Arc<RwLock<ParallaxEffect>>,
    /// Auto-play media
    auto_play: Arc<RwLock<AutoPlayMedia>>,
    /// Reduce blur effects
    reduce_blur: Arc<RwLock<bool>>,
    /// Reduce transparency
    reduce_transparency: Arc<RwLock<bool>>,
    /// Configuration
    config: AccessibilityConfig,
}

impl MotionReduction {
    /// Create new motion reduction
    pub async fn new(config: AccessibilityConfig) -> Result<Self> {
        Ok(Self {
            enabled: Arc::new(RwLock::new(false)),
            animation_level: Arc::new(RwLock::new(AnimationLevel::Full)),
            transition_speed: Arc::new(RwLock::new(TransitionSpeed::Normal)),
            parallax: Arc::new(RwLock::new(ParallaxEffect::Full)),
            auto_play: Arc::new(RwLock::new(AutoPlayMedia::All)),
            reduce_blur: Arc::new(RwLock::new(false)),
            reduce_transparency: Arc::new(RwLock::new(false)),
            config,
        })
    }

    /// Enable motion reduction
    pub async fn enable(&self) {
        *self.enabled.write().await = true;
        self.apply_reduced_settings().await;
    }

    /// Disable motion reduction
    pub async fn disable(&self) {
        *self.enabled.write().await = false;
        self.apply_normal_settings().await;
    }

    /// Apply reduced settings
    async fn apply_reduced_settings(&self) {
        *self.animation_level.write().await = AnimationLevel::Reduced;
        *self.transition_speed.write().await = TransitionSpeed::Slow;
        *self.parallax.write().await = ParallaxEffect::Reduced;
        *self.auto_play.write().await = AutoPlayMedia::None;
        *self.reduce_blur.write().await = true;
        *self.reduce_transparency.write().await = true;
    }

    /// Apply normal settings
    async fn apply_normal_settings(&self) {
        *self.animation_level.write().await = AnimationLevel::Full;
        *self.transition_speed.write().await = TransitionSpeed::Normal;
        *self.parallax.write().await = ParallaxEffect::Full;
        *self.auto_play.write().await = AutoPlayMedia::All;
        *self.reduce_blur.write().await = false;
        *self.reduce_transparency.write().await = false;
    }

    /// Check if enabled
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// Set animation level
    pub async fn set_animation_level(&self, level: AnimationLevel) {
        *self.animation_level.write().await = level;
    }

    /// Get animation level
    pub async fn animation_level(&self) -> AnimationLevel {
        *self.animation_level.read().await
    }

    /// Set transition speed
    pub async fn set_transition_speed(&self, speed: TransitionSpeed) {
        *self.transition_speed.write().await = speed;
    }

    /// Get transition speed
    pub async fn transition_speed(&self) -> TransitionSpeed {
        *self.transition_speed.read().await
    }

    /// Set parallax effect
    pub async fn set_parallax(&self, effect: ParallaxEffect) {
        *self.parallax.write().await = effect;
    }

    /// Get parallax effect
    pub async fn parallax(&self) -> ParallaxEffect {
        *self.parallax.read().await
    }

    /// Set auto-play media
    pub async fn set_auto_play(&self, auto_play: AutoPlayMedia) {
        *self.auto_play.write().await = auto_play;
    }

    /// Get auto-play media
    pub async fn auto_play(&self) -> AutoPlayMedia {
        *self.auto_play.read().await
    }

    /// Set reduce blur
    pub async fn set_reduce_blur(&self, reduce: bool) {
        *self.reduce_blur.write().await = reduce;
    }

    /// Get reduce blur
    pub async fn reduce_blur(&self) -> bool {
        *self.reduce_blur.read().await
    }

    /// Set reduce transparency
    pub async fn set_reduce_transparency(&self, reduce: bool) {
        *self.reduce_transparency.write().await = reduce;
    }

    /// Get reduce transparency
    pub async fn reduce_transparency(&self) -> bool {
        *self.reduce_transparency.read().await
    }

    /// Generate CSS for motion reduction
    pub fn to_css(&self) -> String {
        let mut css = String::new();

        if *self.reduce_blur.blocking_read() {
            css.push_str("* { backdrop-filter: none !important; filter: none !important; }\n");
        }

        if *self.reduce_transparency.blocking_read() {
            css.push_str("* { opacity: 1 !important; background: rgba(0,0,0,0.9) !important; }\n");
        }

        match *self.animation_level.blocking_read() {
            AnimationLevel::Full => {}
            AnimationLevel::Reduced => {
                css.push_str("* { animation-duration: 0.001ms !important; animation-iteration-count: 1 !important; transition-duration: 0.001ms !important; }\n");
            }
            AnimationLevel::None | AnimationLevel::Essential => {
                css.push_str("* { animation: none !important; transition: none !important; }\n");
            }
        }

        match *self.transition_speed.blocking_read() {
            TransitionSpeed::Normal => {}
            TransitionSpeed::Slow => {
                css.push_str("* { transition-duration: 0.5s !important; }\n");
            }
            TransitionSpeed::Fast => {
                css.push_str("* { transition-duration: 0.1s !important; }\n");
            }
            TransitionSpeed::Instant => {
                css.push_str("* { transition-duration: 0s !important; }\n");
            }
        }

        match *self.parallax.blocking_read() {
            ParallaxEffect::Full => {}
            ParallaxEffect::Reduced => {
                css.push_str(".parallax { transform: none !important; }\n");
            }
            ParallaxEffect::None => {
                css.push_str("* { transform: none !important; perspective: none !important; }\n");
            }
        }

        match *self.auto_play.blocking_read() {
            AutoPlayMedia::All => {}
            AutoPlayMedia::VideosWithoutAudio => {
                css.push_str("video[autoplay][muted] { autoplay: true; } video[autoplay]:not([muted]) { autoplay: false; }\n");
            }
            AutoPlayMedia::None => {
                css.push_str("video, audio { autoplay: false; }\n");
            }
        }

        css
    }

    /// Check if should reduce motion
    pub fn should_reduce(&self) -> bool {
        *self.enabled.blocking_read()
    }

    /// Get preferred animation duration
    pub fn animation_duration(&self, normal_ms: u64) -> u64 {
        match *self.transition_speed.blocking_read() {
            TransitionSpeed::Normal => normal_ms,
            TransitionSpeed::Slow => normal_ms * 2,
            TransitionSpeed::Fast => normal_ms / 2,
            TransitionSpeed::Instant => 0,
        }
    }
}