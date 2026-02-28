//! Font preview and management system

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::fs;
use serde::{Serialize, Deserialize};
use font_kit::{
    font::Font as FontKitFont,
    handle::Handle,
    family_name::FamilyName,
    properties::{Weight, Stretch, Style},
    source::SystemSource,
};
use fontdb::{Database, Family, Query, Source};
use rusttype::{Font as RustTypeFont, Scale, point};
use ttf_parser::Face;
use image::{RgbaImage, Rgba};
use base64::{Engine as _, engine::general_purpose};

use crate::{Result, DesignError, DesignConfig};

/// Font weight
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
    ExtraBlack,
    Custom(u16),
}

impl FontWeight {
    pub fn to_number(&self) -> u16 {
        match self {
            FontWeight::Thin => 100,
            FontWeight::ExtraLight => 200,
            FontWeight::Light => 300,
            FontWeight::Normal => 400,
            FontWeight::Medium => 500,
            FontWeight::SemiBold => 600,
            FontWeight::Bold => 700,
            FontWeight::ExtraBold => 800,
            FontWeight::Black => 900,
            FontWeight::ExtraBlack => 950,
            FontWeight::Custom(w) => *w,
        }
    }

    pub fn from_number(n: u16) -> Self {
        match n {
            100 => FontWeight::Thin,
            200 => FontWeight::ExtraLight,
            300 => FontWeight::Light,
            400 => FontWeight::Normal,
            500 => FontWeight::Medium,
            600 => FontWeight::SemiBold,
            700 => FontWeight::Bold,
            800 => FontWeight::ExtraBold,
            900 => FontWeight::Black,
            950 => FontWeight::ExtraBlack,
            _ => FontWeight::Custom(n),
        }
    }
}

/// Font style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

/// Font stretch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FontStretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
    Custom(f32),
}

/// Font variant
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontVariant {
    pub weight: FontWeight,
    pub style: FontStyle,
    pub stretch: FontStretch,
}

/// Font family
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontFamily {
    pub id: String,
    pub name: String,
    pub variants: Vec<FontVariant>,
    pub default_variant: FontVariant,
    pub category: String,
    pub license: String,
    pub designer: Option<String>,
    pub installed: bool,
    pub file_paths: HashMap<String, PathBuf>,
}

/// Font
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Font {
    pub id: String,
    pub family_id: String,
    pub family_name: String,
    pub full_name: String,
    pub postscript_name: String,
    pub variant: FontVariant,
    pub file_path: PathBuf,
    pub data: Vec<u8>,
    pub metrics: FontMetrics,
}

/// Font metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontMetrics {
    pub units_per_em: u16,
    pub ascent: i16,
    pub descent: i16,
    pub line_gap: i16,
    pub cap_height: i16,
    pub x_height: i16,
    pub underline_position: i16,
    pub underline_thickness: i16,
    pub strikeout_position: i16,
    pub strikeout_thickness: i16,
}

/// Font preview text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontPreviewText {
    pub text: String,
    pub size: f32,
    pub color: crate::color_picker::Color,
    pub background: crate::color_picker::Color,
    pub line_height: f32,
    pub letter_spacing: f32,
    pub word_spacing: f32,
}

impl Default for FontPreviewText {
    fn default() -> Self {
        Self {
            text: "The quick brown fox jumps over the lazy dog".to_string(),
            size: 48.0,
            color: crate::color_picker::Color::new(0, 0, 0, 1.0),
            background: crate::color_picker::Color::new(255, 255, 255, 1.0),
            line_height: 1.2,
            letter_spacing: 0.0,
            word_spacing: 0.0,
        }
    }
}

/// Font preview
pub struct FontPreview {
    /// System font source
    system_source: SystemSource,
    /// Font database
    database: Arc<RwLock<Database>>,
    /// Loaded fonts
    fonts: Arc<RwLock<HashMap<String, Font>>>,
    /// Font families
    families: Arc<RwLock<HashMap<String, FontFamily>>>,
    /// Configuration
    config: DesignConfig,
    /// Preview cache
    cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl FontPreview {
    /// Create new font preview
    pub async fn new(config: DesignConfig) -> Result<Self> {
        let mut database = Database::new();
        database.load_system_fonts();

        let preview = Self {
            system_source: SystemSource::new(),
            database: Arc::new(RwLock::new(database)),
            fonts: Arc::new(RwLock::new(HashMap::new())),
            families: Arc::new(RwLock::new(HashMap::new())),
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load system fonts
        preview.load_system_fonts().await?;

        // Load custom fonts
        preview.load_custom_fonts().await?;

        Ok(preview)
    }

    /// Load system fonts
    async fn load_system_fonts(&self) -> Result<()> {
        let families = self.system_source.all_families()
            .map_err(|e| DesignError::FontNotFound(format!("Failed to list fonts: {}", e)))?;

        for family_name in families {
            let handles = self.system_source.select_family_by_name(&family_name)
                .unwrap_or_default();

            if !handles.is_empty() {
                self.load_font_family(&family_name, handles).await?;
            }
        }

        Ok(())
    }

    /// Load font family
    async fn load_font_family(&self, family_name: &str, handles: Vec<Handle>) -> Result<String> {
        let family_id = uuid::Uuid::new_v4().to_string();
        let mut variants = Vec::new();
        let mut variant_map = HashMap::new();
        let mut fonts = Vec::new();

        for handle in handles {
            if let Ok(font_data) = handle.load() {
                if let Some(font) = self.font_from_handle(&family_name, font_data).await? {
                    variants.push(font.variant.clone());
                    variant_map.insert(self.variant_key(&font.variant), font.file_path.clone());
                    fonts.push(font);
                }
            }
        }

        if variants.is_empty() {
            return Err(DesignError::FontNotFound(format!("No valid fonts in family: {}", family_name)));
        }

        let default_variant = variants.iter()
            .find(|v| matches!(v.weight, FontWeight::Normal) && matches!(v.style, FontStyle::Normal))
            .cloned()
            .unwrap_or_else(|| variants[0].clone());

        let family = FontFamily {
            id: family_id.clone(),
            name: family_name.to_string(),
            variants,
            default_variant,
            category: "system".to_string(),
            license: "System".to_string(),
            designer: None,
            installed: true,
            file_paths: variant_map,
        };

        self.families.write().await.insert(family_id.clone(), family);

        for font in fonts {
            self.fonts.write().await.insert(font.id.clone(), font);
        }

        Ok(family_id)
    }

    /// Create font from handle
    async fn font_from_handle(&self, family_name: &str, font_data: font_kit::font::Font) -> Result<Option<Font>> {
        let font_ref = font_data.as_ref();
        let postscript_name = font_ref.postscript_name()
            .unwrap_or_else(|| family_name.to_string().replace(' ', "-"));

        let properties = font_ref.properties();
        
        let variant = FontVariant {
            weight: FontWeight::from_number(properties.weight.0),
            style: match properties.style {
                Style::Normal => FontStyle::Normal,
                Style::Italic => FontStyle::Italic,
                Style::Oblique => FontStyle::Oblique,
            },
            stretch: FontStretch::Custom(properties.stretch.0),
        };

        // Get raw font data
        let font_bytes = match font_data.copy_font_data() {
            Some(data) => data.to_vec(),
            None => return Ok(None),
        };

        // Parse TTF/OTF for metrics
        let face = Face::parse(&font_bytes, 0)
            .map_err(|e| DesignError::FontNotFound(format!("Failed to parse font: {}", e)))?;

        let metrics = FontMetrics {
            units_per_em: face.units_per_em(),
            ascent: face.ascender(),
            descent: face.descender(),
            line_gap: face.line_gap(),
            cap_height: face.capital_height().unwrap_or(0),
            x_height: face.x_height().unwrap_or(0),
            underline_position: face.underline_position().unwrap_or(0),
            underline_thickness: face.underline_thickness().unwrap_or(0),
            strikeout_position: face.strikeout_position().unwrap_or(0),
            strikeout_thickness: face.strikeout_thickness().unwrap_or(0),
        };

        let font = Font {
            id: uuid::Uuid::new_v4().to_string(),
            family_id: String::new(), // Will be set by caller
            family_name: family_name.to_string(),
            full_name: font_ref.full_name(),
            postscript_name,
            variant,
            file_path: PathBuf::new(),
            data: font_bytes,
            metrics,
        };

        Ok(Some(font))
    }

    /// Load custom fonts from directory
    async fn load_custom_fonts(&self) -> Result<()> {
        let fonts_dir = self.config.assets_dir.join("fonts");
        if !fonts_dir.exists() {
            return Ok(());
        }

        let mut read_dir = fs::read_dir(&fonts_dir).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "ttf" || ext == "otf" || ext == "woff" || ext == "woff2" {
                    self.load_font_file(&path).await?;
                }
            }
        }

        Ok(())
    }

    /// Load font from file
    pub async fn load_font_file(&self, path: &Path) -> Result<String> {
        let data = fs::read(path).await?;
        
        // Parse font
        let face = Face::parse(&data, 0)
            .map_err(|e| DesignError::FontNotFound(format!("Failed to parse font: {}", e)))?;

        let family_name = face.family_name()
            .unwrap_or_else(|| path.file_stem().unwrap_or_default().to_string_lossy().to_string());

        let variant = FontVariant {
            weight: FontWeight::from_number(face.weight().to_number()),
            style: match face.style() {
                ttf_parser::Style::Normal => FontStyle::Normal,
                ttf_parser::Style::Italic => FontStyle::Italic,
                ttf_parser::Style::Oblique => FontStyle::Oblique,
            },
            stretch: FontStretch::Custom(face.width().to_number() as f32 / 100.0),
        };

        let metrics = FontMetrics {
            units_per_em: face.units_per_em(),
            ascent: face.ascender(),
            descent: face.descender(),
            line_gap: face.line_gap(),
            cap_height: face.capital_height().unwrap_or(0),
            x_height: face.x_height().unwrap_or(0),
            underline_position: face.underline_position().unwrap_or(0),
            underline_thickness: face.underline_thickness().unwrap_or(0),
            strikeout_position: face.strikeout_position().unwrap_or(0),
            strikeout_thickness: face.strikeout_thickness().unwrap_or(0),
        };

        let postscript_name = face.postscript_name()
            .unwrap_or_else(|| format!("{}-{}", family_name, variant.weight.to_number()));

        let font_id = uuid::Uuid::new_v4().to_string();
        let font = Font {
            id: font_id.clone(),
            family_id: String::new(),
            family_name: family_name.clone(),
            full_name: format!("{} {}", family_name, variant.weight.to_number()),
            postscript_name,
            variant: variant.clone(),
            file_path: path.to_path_buf(),
            data,
            metrics,
        };

        // Add to family
        let variant_key = self.variant_key(&variant);
        let mut families = self.families.write().await;
        
        let family = if let Some(family) = families.values_mut()
            .find(|f| f.name == family_name) {
            family.variants.push(variant);
            family.file_paths.insert(variant_key, path.to_path_buf());
            family
        } else {
            let family_id = uuid::Uuid::new_v4().to_string();
            let family = FontFamily {
                id: family_id.clone(),
                name: family_name,
                variants: vec![variant],
                default_variant: variant,
                category: "custom".to_string(),
                license: "Custom".to_string(),
                designer: None,
                installed: true,
                file_paths: HashMap::from([(variant_key, path.to_path_buf())]),
            };
            families.insert(family_id, family);
            families.get_mut(&family_id).unwrap()
        };

        // Add font
        self.fonts.write().await.insert(font_id, font);

        Ok(family.id.clone())
    }

    /// Get font family
    pub async fn get_family(&self, id: &str) -> Option<FontFamily> {
        self.families.read().await.get(id).cloned()
    }

    /// Get font by ID
    pub async fn get_font(&self, id: &str) -> Option<Font> {
        self.fonts.read().await.get(id).cloned()
    }

    /// Get font by variant
    pub async fn get_font_by_variant(&self, family_name: &str, variant: &FontVariant) -> Option<Font> {
        let families = self.families.read().await;
        if let Some(family) = families.values().find(|f| f.name == family_name) {
            let variant_key = self.variant_key(variant);
            if let Some(path) = family.file_paths.get(&variant_key) {
                return self.fonts.read().await.values()
                    .find(|f| f.file_path == *path)
                    .cloned();
            }
        }
        None
    }

    /// List all font families
    pub async fn list_families(&self) -> Vec<FontFamily> {
        self.families.read().await.values().cloned().collect()
    }

    /// List all fonts
    pub async fn list_fonts(&self) -> Vec<Font> {
        self.fonts.read().await.values().cloned().collect()
    }

    /// Search fonts
    pub async fn search(&self, query: &str) -> Vec<FontFamily> {
        let query = query.to_lowercase();
        self.families.read().await
            .values()
            .filter(|f| f.name.to_lowercase().contains(&query))
            .cloned()
            .collect()
    }

    /// Generate preview image
    pub async fn generate_preview(
        &self,
        font: &Font,
        preview_text: &FontPreviewText,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>> {
        let cache_key = format!(
            "{}-{}-{}-{}-{}",
            font.id,
            preview_text.size,
            preview_text.text,
            width,
            height
        );

        // Check cache
        if let Some(cached) = self.cache.read().await.get(&cache_key) {
            return Ok(cached.clone());
        }

        // Load font
        let face = Face::parse(&font.data, 0)
            .map_err(|e| DesignError::FontNotFound(format!("Failed to parse font: {}", e)))?;

        let scale = Scale::uniform(preview_text.size);
        let font = RustTypeFont::from_bytes(font.data.clone(), 0)
            .map_err(|e| DesignError::FontNotFound(format!("Failed to load font: {}", e)))?;

        // Create image
        let mut img = RgbaImage::new(width, height);
        
        // Fill background
        let bg = Rgba([
            preview_text.background.r,
            preview_text.background.g,
            preview_text.background.b,
            (preview_text.background.a * 255.0) as u8,
        ]);
        for pixel in img.pixels_mut() {
            *pixel = bg;
        }

        // Layout text
        let mut x = 20.0;
        let mut y = preview_text.size + 20.0;

        for ch in preview_text.text.chars() {
            if ch == '\n' {
                x = 20.0;
                y += preview_text.size * preview_text.line_height;
                continue;
            }

            let glyph = font.glyph(ch)
                .scaled(scale)
                .positioned(point(x, y));

            if let Some(bb) = glyph.pixel_bounding_box() {
                glyph.draw(|gx, gy, v| {
                    let px = bb.min.x + gx as i32;
                    let py = bb.min.y + gy as i32;
                    
                    if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                        let pixel = img.get_pixel_mut(px as u32, py as u32);
                        let fg = Rgba([
                            preview_text.color.r,
                            preview_text.color.g,
                            preview_text.color.b,
                            (preview_text.color.a * 255.0 * v) as u8,
                        ]);
                        *pixel = fg;
                    }
                });
            }

            x += glyph.unpositioned().h_advance().unwrap_or(0.0) + preview_text.letter_spacing;
        }

        // Encode as PNG
        let mut png_data = Vec::new();
        let mut encoder = image::codecs::png::PngEncoder::new(&mut png_data);
        encoder.encode(
            &img,
            width,
            height,
            image::ColorType::Rgba8,
        )?;

        // Cache result
        self.cache.write().await.insert(cache_key, png_data.clone());

        Ok(png_data)
    }

    /// Generate preview as base64
    pub async fn generate_preview_base64(
        &self,
        font: &Font,
        preview_text: &FontPreviewText,
        width: u32,
        height: u32,
    ) -> Result<String> {
        let png = self.generate_preview(font, preview_text, width, height).await?;
        Ok(format!(
            "data:image/png;base64,{}",
            general_purpose::STANDARD.encode(png)
        ))
    }

    /// Get font metrics for text
    pub fn measure_text(&self, font: &Font, text: &str, size: f32) -> (f32, f32) {
        let face = match Face::parse(&font.data, 0) {
            Ok(face) => face,
            Err(_) => return (0.0, 0.0),
        };

        let scale = size / face.units_per_em() as f32;
        
        let mut width = 0.0;
        for c in text.chars() {
            if let Some(glyph) = face.glyph_index(c) {
                if let Some(metrics) = face.glyph_hor_metrics(glyph) {
                    width += metrics.advance_width as f32 * scale;
                }
            }
        }

        let height = (face.ascender() - face.descender()) as f32 * scale;

        (width, height)
    }

    /// Get font variants
    pub fn get_variants(&self, font: &Font) -> Vec<FontVariant> {
        let mut variants = Vec::new();
        
        for weight in [100, 200, 300, 400, 500, 600, 700, 800, 900] {
            variants.push(FontVariant {
                weight: FontWeight::from_number(weight),
                style: FontStyle::Normal,
                stretch: FontStretch::Normal,
            });
            
            variants.push(FontVariant {
                weight: FontWeight::from_number(weight),
                style: FontStyle::Italic,
                stretch: FontStretch::Normal,
            });
        }

        variants
    }

    /// Generate variant key
    fn variant_key(&self, variant: &FontVariant) -> String {
        format!(
            "{}-{:?}-{:?}",
            variant.weight.to_number(),
            variant.style,
            variant.stretch
        )
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }

    /// Install font
    pub async fn install_font(&self, font_data: Vec<u8>, filename: Option<&str>) -> Result<String> {
        let fonts_dir = self.config.assets_dir.join("fonts");
        fs::create_dir_all(&fonts_dir).await?;

        let filename = filename.unwrap_or_else(|| {
            let face = Face::parse(&font_data, 0).ok();
            let name = face.and_then(|f| f.family_name()).unwrap_or("font");
            format!("{}.ttf", name)
        });

        let path = fonts_dir.join(filename);
        fs::write(&path, font_data).await?;

        self.load_font_file(&path).await
    }

    /// Uninstall font
    pub async fn uninstall_font(&self, font_id: &str) -> Result<()> {
        if let Some(font) = self.fonts.write().await.remove(font_id) {
            // Remove from family
            let mut families = self.families.write().await;
            if let Some(family) = families.values_mut()
                .find(|f| f.name == font.family_name) {
                let variant_key = self.variant_key(&font.variant);
                family.file_paths.remove(&variant_key);
                
                // Remove family if empty
                if family.file_paths.is_empty() {
                    families.retain(|_, f| f.name != font.family_name);
                }
            }

            // Delete file if it's a custom font
            if font.file_path.exists() {
                let _ = fs::remove_file(&font.file_path).await;
            }
        }

        Ok(())
    }
}