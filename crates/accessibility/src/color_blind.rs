//! Color blindness simulation and correction

use std::sync::Arc;

use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use palette::{Srgb, FromColor, IntoColor, LinSrgb, Lms};
use palette::encoding::linear::Linear;

use crate::{Result, AccessibilityError, AccessibilityConfig};

/// Color blindness type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorBlindType {
    /// Protanopia (red-blind)
    Protanopia,
    /// Protanomaly (red-weak)
    Protanomaly,
    /// Deuteranopia (green-blind)
    Deuteranopia,
    /// Deuteranomaly (green-weak)
    Deuteranomaly,
    /// Tritanopia (blue-blind)
    Tritanopia,
    /// Tritanomaly (blue-weak)
    Tritanomaly,
    /// Achromatopsia (total color blindness)
    Achromatopsia,
    /// Achromatomaly (partial color blindness)
    Achromatomaly,
}

impl ColorBlindType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            ColorBlindType::Protanopia => "Protanopia (Red-blind)",
            ColorBlindType::Protanomaly => "Protanomaly (Red-weak)",
            ColorBlindType::Deuteranopia => "Deuteranopia (Green-blind)",
            ColorBlindType::Deuteranomaly => "Deuteranomaly (Green-weak)",
            ColorBlindType::Tritanopia => "Tritanopia (Blue-blind)",
            ColorBlindType::Tritanomaly => "Tritanomaly (Blue-weak)",
            ColorBlindType::Achromatopsia => "Achromatopsia (Total color blindness)",
            ColorBlindType::Achromatomaly => "Achromatomaly (Partial color blindness)",
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            ColorBlindType::Protanopia => "Difficulty distinguishing red and green",
            ColorBlindType::Protanomaly => "Reduced sensitivity to red light",
            ColorBlindType::Deuteranopia => "Difficulty distinguishing green and red",
            ColorBlindType::Deuteranomaly => "Reduced sensitivity to green light",
            ColorBlindType::Tritanopia => "Difficulty distinguishing blue and yellow",
            ColorBlindType::Tritanomaly => "Reduced sensitivity to blue light",
            ColorBlindType::Achromatopsia => "See only shades of gray",
            ColorBlindType::Achromatomaly => "Reduced color perception",
        }
    }
}

/// Color blindness mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ColorBlindMode {
    pub enabled: bool,
    pub blindness_type: ColorBlindType,
    pub severity: f32,  // 0.0 to 1.0
    pub correct_colors: bool,
}

impl Default for ColorBlindMode {
    fn default() -> Self {
        Self {
            enabled: false,
            blindness_type: ColorBlindType::Deuteranopia,
            severity: 1.0,
            correct_colors: false,
        }
    }
}

/// Simulation strength
#[derive(Debug, Clone, Copy)]
pub struct SimulationStrength(pub f32);

impl Default for SimulationStrength {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Color correction
#[derive(Debug, Clone, Copy)]
pub struct ColorCorrection {
    pub enabled: bool,
    pub strength: f32,
    pub target_type: ColorBlindType,
}

/// Color blind simulator
pub struct ColorBlindSimulator {
    /// Is enabled
    enabled: Arc<RwLock<bool>>,
    /// Current mode
    mode: Arc<RwLock<ColorBlindMode>>,
    /// Simulation strength
    strength: Arc<RwLock<SimulationStrength>>,
    /// Correction enabled
    correction: Arc<RwLock<ColorCorrection>>,
    /// Configuration
    config: AccessibilityConfig,
}

impl ColorBlindSimulator {
    /// Create new color blind simulator
    pub async fn new(config: AccessibilityConfig) -> Result<Self> {
        Ok(Self {
            enabled: Arc::new(RwLock::new(false)),
            mode: Arc::new(RwLock::new(ColorBlindMode::default())),
            strength: Arc::new(RwLock::new(SimulationStrength::default())),
            correction: Arc::new(RwLock::new(ColorCorrection {
                enabled: false,
                strength: 1.0,
                target_type: ColorBlindType::Deuteranopia,
            })),
            config,
        })
    }

    /// Enable simulation
    pub async fn enable(&self) {
        *self.enabled.write().await = true;
    }

    /// Disable simulation
    pub async fn disable(&self) {
        *self.enabled.write().await = false;
    }

    /// Check if enabled
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// Set mode
    pub async fn set_mode(&self, mode: ColorBlindMode) {
        *self.mode.write().await = mode;
    }

    /// Get mode
    pub async fn mode(&self) -> ColorBlindMode {
        *self.mode.read().await
    }

    /// Set simulation strength
    pub async fn set_strength(&self, strength: SimulationStrength) {
        *self.strength.write().await = strength;
    }

    /// Get simulation strength
    pub async fn strength(&self) -> SimulationStrength {
        *self.strength.read().await
    }

    /// Simulate color blindness on RGB color
    pub fn simulate_color(&self, r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        let rgb = Srgb::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
        );

        let linear: LinSrgb = rgb.into_linear();

        let simulated = match self.mode.blocking_read().blindness_type {
            ColorBlindType::Protanopia => self.simulate_protanopia(linear),
            ColorBlindType::Protanomaly => self.simulate_protanomaly(linear),
            ColorBlindType::Deuteranopia => self.simulate_deuteranopia(linear),
            ColorBlindType::Deuteranomaly => self.simulate_deuteranomaly(linear),
            ColorBlindType::Tritanopia => self.simulate_tritanopia(linear),
            ColorBlindType::Tritanomaly => self.simulate_tritanomaly(linear),
            ColorBlindType::Achromatopsia => self.simulate_achromatopsia(linear),
            ColorBlindType::Achromatomaly => self.simulate_achromatomaly(linear),
        };

        let rgb: Srgb<f32> = Srgb::from_linear(simulated);
        (
            (rgb.red * 255.0) as u8,
            (rgb.green * 255.0) as u8,
            (rgb.blue * 255.0) as u8,
        )
    }

    /// Simulate protanopia
    fn simulate_protanopia(&self, rgb: LinSrgb) -> LinSrgb {
        // LMS transformation matrix for protanopia
        let l = 0.0;
        let m = 0.0;
        let s = rgb.red * 0.0 + rgb.green * 0.0 + rgb.blue * 1.0;
        
        // Convert back to RGB
        let r = 0.0;
        let g = 0.0;
        let b = s;

        LinSrgb::new(r, g, b)
    }

    /// Simulate protanomaly
    fn simulate_protanomaly(&self, rgb: LinSrgb) -> LinSrgb {
        let severity = self.mode.blocking_read().severity;
        let protan = self.simulate_protanopia(rgb);
        
        // Blend with original based on severity
        LinSrgb::new(
            rgb.red * (1.0 - severity) + protan.red * severity,
            rgb.green * (1.0 - severity) + protan.green * severity,
            rgb.blue * (1.0 - severity) + protan.blue * severity,
        )
    }

    /// Simulate deuteranopia
    fn simulate_deuteranopia(&self, rgb: LinSrgb) -> LinSrgb {
        // Simplified deuteranopia simulation
        let r = rgb.red * 0.625 + rgb.green * 0.375;
        let g = rgb.red * 0.7 + rgb.green * 0.3;
        let b = rgb.blue;

        LinSrgb::new(r, g, b)
    }

    /// Simulate deuteranomaly
    fn simulate_deuteranomaly(&self, rgb: LinSrgb) -> LinSrgb {
        let severity = self.mode.blocking_read().severity;
        let deuteran = self.simulate_deuteranopia(rgb);
        
        LinSrgb::new(
            rgb.red * (1.0 - severity) + deuteran.red * severity,
            rgb.green * (1.0 - severity) + deuteran.green * severity,
            rgb.blue * (1.0 - severity) + deuteran.blue * severity,
        )
    }

    /// Simulate tritanopia
    fn simulate_tritanopia(&self, rgb: LinSrgb) -> LinSrgb {
        // Simplified tritanopia simulation
        let r = rgb.red * 0.95 + rgb.green * 0.05;
        let g = rgb.green;
        let b = rgb.blue * 0.5 + rgb.red * 0.5;

        LinSrgb::new(r, g, b)
    }

    /// Simulate tritanomaly
    fn simulate_tritanomaly(&self, rgb: LinSrgb) -> LinSrgb {
        let severity = self.mode.blocking_read().severity;
        let tritan = self.simulate_tritanopia(rgb);
        
        LinSrgb::new(
            rgb.red * (1.0 - severity) + tritan.red * severity,
            rgb.green * (1.0 - severity) + tritan.green * severity,
            rgb.blue * (1.0 - severity) + tritan.blue * severity,
        )
    }

    /// Simulate achromatopsia (grayscale)
    fn simulate_achromatopsia(&self, rgb: LinSrgb) -> LinSrgb {
        let gray = rgb.red * 0.299 + rgb.green * 0.587 + rgb.blue * 0.114;
        LinSrgb::new(gray, gray, gray)
    }

    /// Simulate achromatomaly
    fn simulate_achromatomaly(&self, rgb: LinSrgb) -> LinSrgb {
        let severity = self.mode.blocking_read().severity;
        let gray = self.simulate_achromatopsia(rgb);
        
        LinSrgb::new(
            rgb.red * (1.0 - severity) + gray.red * severity,
            rgb.green * (1.0 - severity) + gray.green * severity,
            rgb.blue * (1.0 - severity) + gray.blue * severity,
        )
    }

    /// Apply color correction
    pub fn correct_color(&self, r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        let correction = self.correction.blocking_read();
        if !correction.enabled {
            return (r, g, b);
        }

        // Simple correction: shift colors away from problematic ranges
        // In production, use proper color correction algorithms
        let (r, g, b) = match correction.target_type {
            ColorBlindType::Protanopia | ColorBlindType::Protanomaly => {
                // Reduce red, boost blue
                (r, g, (b as f32 * 1.2).min(255.0) as u8)
            }
            ColorBlindType::Deuteranopia | ColorBlindType::Deuteranomaly => {
                // Boost red and blue
                ((r as f32 * 1.2).min(255.0) as u8, g, (b as f32 * 1.2).min(255.0) as u8)
            }
            ColorBlindType::Tritanopia | ColorBlindType::Tritanomaly => {
                // Boost red and green
                ((r as f32 * 1.2).min(255.0) as u8, (g as f32 * 1.2).min(255.0) as u8, b)
            }
            _ => (r, g, b),
        };

        (r, g, b)
    }

    /// Simulate an entire image
    pub async fn simulate_image(&self, data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
        if !self.is_enabled().await {
            return Ok(data.to_vec());
        }

        // This would process each pixel
        // Simplified version - in production, use image crate
        Ok(data.to_vec())
    }

    /// Generate CSS filter for color blindness
    pub fn to_css_filter(&self) -> String {
        match self.mode.blocking_read().blindness_type {
            ColorBlindType::Protanopia => "filter: url('#protanopia');",
            ColorBlindType::Deuteranopia => "filter: url('#deuteranopia');",
            ColorBlindType::Tritanopia => "filter: url('#tritanopia');",
            ColorBlindType::Achromatopsia => "filter: grayscale(100%);",
            _ => "",
        }.to_string()
    }

    /// Generate SVG filter definitions
    pub fn svg_filters(&self) -> String {
        r#"
        <svg style="display:none;">
            <defs>
                <filter id="protanopia">
                    <feColorMatrix type="matrix" values="
                        0.567, 0.433, 0, 0, 0
                        0.558, 0.442, 0, 0, 0
                        0,     0.242, 0.758, 0, 0
                        0,     0,     0,     1, 0
                    "/>
                </filter>
                <filter id="deuteranopia">
                    <feColorMatrix type="matrix" values="
                        0.625, 0.375, 0, 0, 0
                        0.7,   0.3,   0, 0, 0
                        0,     0.3,   0.7, 0, 0
                        0,     0,     0,   1, 0
                    "/>
                </filter>
                <filter id="tritanopia">
                    <feColorMatrix type="matrix" values="
                        0.95, 0.05,  0,    0, 0
                        0,    0.433, 0.567, 0, 0
                        0,    0.475, 0.525, 0, 0
                        0,    0,     0,     1, 0
                    "/>
                </filter>
            </defs>
        </svg>
        "#.to_string()
    }
}