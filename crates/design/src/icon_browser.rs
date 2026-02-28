//! Icon browser and management system

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::fs;
use serde::{Serialize, Deserialize};
use image::{DynamicImage, ImageFormat};
use base64::{Engine as _, engine::general_purpose};
use resvg::{tiny_skia, usvg};
use tracing::{info, warn, debug};

use crate::{Result, DesignError, DesignConfig};

/// Icon size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IconSize {
    Tiny(usize),      // 16x16
    Small(usize),     // 24x24
    Medium(usize),    // 32x32
    Large(usize),     // 48x48
    XLarge(usize),    // 64x64
    XXLarge(usize),   // 96x96
    Custom(usize),    // Custom size
}

impl IconSize {
    pub fn pixels(&self) -> usize {
        match self {
            IconSize::Tiny(s) => *s,
            IconSize::Small(s) => *s,
            IconSize::Medium(s) => *s,
            IconSize::Large(s) => *s,
            IconSize::XLarge(s) => *s,
            IconSize::XXLarge(s) => *s,
            IconSize::Custom(s) => *s,
        }
    }

    pub fn default_sizes() -> Vec<IconSize> {
        vec![
            IconSize::Tiny(16),
            IconSize::Small(24),
            IconSize::Medium(32),
            IconSize::Large(48),
            IconSize::XLarge(64),
            IconSize::XXLarge(96),
        ]
    }
}

/// Icon variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IconVariant {
    Outline,
    Filled,
    TwoTone,
    Sharp,
    Rounded,
    Custom(String),
}

/// Icon category
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IconCategory {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon_count: usize,
    pub parent: Option<String>,
}

/// Icon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Icon {
    pub id: String,
    pub name: String,
    pub set: String,
    pub category: String,
    pub tags: Vec<String>,
    pub variant: IconVariant,
    pub svg: String,
    pub width: u32,
    pub height: u32,
    pub view_box: (u32, u32, u32, u32),
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub popularity: u32,
}

impl Icon {
    /// Render icon to PNG at specified size
    pub async fn to_png(&self, size: IconSize) -> Result<Vec<u8>> {
        let pixel_size = size.pixels();
        
        // Parse SVG
        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_data(self.svg.as_bytes(), &opt)
            .map_err(|e| DesignError::SvgError(format!("Failed to parse SVG: {}", e)))?;

        // Create pixmap
        let mut pixmap = tiny_skia::Pixmap::new(pixel_size as u32, pixel_size as u32)
            .ok_or_else(|| DesignError::SvgError("Failed to create pixmap".to_string()))?;

        // Render
        resvg::render(
            &tree,
            tiny_skia::Transform::from_scale(
                pixel_size as f32 / self.width as f32,
                pixel_size as f32 / self.height as f32,
            ),
            &mut pixmap.as_mut(),
        );

        // Encode as PNG
        let png_data = pixmap.encode_png()
            .map_err(|e| DesignError::SvgError(format!("Failed to encode PNG: {}", e)))?;

        Ok(png_data)
    }

    /// Render icon to base64 PNG
    pub async fn to_base64_png(&self, size: IconSize) -> Result<String> {
        let png = self.to_png(size).await?;
        Ok(format!(
            "data:image/png;base64,{}",
            general_purpose::STANDARD.encode(png)
        ))
    }

    /// Render icon to data URL
    pub async fn to_data_url(&self) -> String {
        format!(
            "data:image/svg+xml;base64,{}",
            general_purpose::STANDARD.encode(self.svg.as_bytes())
        )
    }

    /// Apply color to icon
    pub fn recolor(&self, color: &crate::color_picker::Color) -> Result<Icon> {
        let mut svg = self.svg.clone();
        
        // Replace currentColor or fill attributes
        let hex = color.to_hex();
        
        // Simple regex replacement - in production use proper SVG parser
        let re = regex::Regex::new(r#"fill="[^"]*""#)?;
        svg = re.replace_all(&svg, format!("fill=\"{}\"", hex)).to_string();
        
        let re = regex::Regex::new(r#"stroke="[^"]*""#)?;
        svg = re.replace_all(&svg, format!("stroke=\"{}\"", hex)).to_string();
        
        let re = regex::Regex::new(r#"currentColor"#)?;
        svg = re.replace_all(&svg, hex.as_str()).to_string();

        let mut icon = self.clone();
        icon.svg = svg;
        icon.updated_at = chrono::Utc::now();

        Ok(icon)
    }

    /// Resize viewbox
    pub fn resize(&self, width: u32, height: u32) -> Result<Icon> {
        let mut svg = self.svg.clone();
        
        // Update viewBox and dimensions
        let re = regex::Regex::new(r#"viewBox="[^"]*""#)?;
        svg = re.replace_all(&svg, format!("viewBox=\"0 0 {} {}\"", width, height)).to_string();
        
        let re = regex::Regex::new(r#"width="[^"]*""#)?;
        svg = re.replace_all(&svg, format!("width=\"{}\"", width)).to_string();
        
        let re = regex::Regex::new(r#"height="[^"]*""#)?;
        svg = re.replace_all(&svg, format!("height=\"{}\"", height)).to_string();

        let mut icon = self.clone();
        icon.svg = svg;
        icon.width = width;
        icon.height = height;
        icon.view_box = (0, 0, width as i32, height as i32);
        icon.updated_at = chrono::Utc::now();

        Ok(icon)
    }
}

/// Icon pack
#[derive(Debug, Clone)]
pub struct IconPack {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub license: String,
    pub icons: Vec<Icon>,
    pub categories: Vec<IconCategory>,
    pub variants: Vec<IconVariant>,
}

/// Icon browser
pub struct IconBrowser {
    /// Icon sets by ID
    icon_sets: Arc<RwLock<HashMap<String, IconPack>>>,
    /// Icon cache
    cache: Arc<RwLock<HashMap<String, HashMap<IconSize, Vec<u8>>>>>,
    /// Configuration
    config: DesignConfig,
    /// Search index
    search_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl IconBrowser {
    /// Create new icon browser
    pub async fn new(config: DesignConfig) -> Result<Self> {
        let browser = Self {
            icon_sets: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            search_index: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load built-in icon sets
        browser.load_builtin_sets().await?;

        // Load custom icon sets
        browser.load_custom_sets().await?;

        Ok(browser)
    }

    /// Load built-in icon sets
    async fn load_builtin_sets(&self) -> Result<()> {
        // Material Icons
        self.load_material_icons().await?;
        
        // Feather Icons
        self.load_feather_icons().await?;
        
        // Font Awesome
        self.load_font_awesome().await?;
        
        // Heroicons
        self.load_heroicons().await?;

        Ok(())
    }

    /// Load Material Icons
    async fn load_material_icons(&self) -> Result<()> {
        let mut icons = Vec::new();
        
        // Common Material Icons
        let material_icons = vec![
            ("home", "M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z"),
            ("search", "M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"),
            ("settings", "M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94 0 .31.02.64.07.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.57 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"),
        ];

        for (name, path) in material_icons {
            let svg = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24">
                    <path d="{}" fill="currentColor"/>
                </svg>"#,
                path
            );

            icons.push(Icon {
                id: format!("material-{}", name),
                name: name.to_string(),
                set: "material".to_string(),
                category: "ui".to_string(),
                tags: vec![name.to_string(), "material".to_string()],
                variant: IconVariant::Filled,
                svg,
                width: 24,
                height: 24,
                view_box: (0, 0, 24, 24),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                popularity: 1000,
            });
        }

        let pack = IconPack {
            id: "material".to_string(),
            name: "Material Icons".to_string(),
            version: "1.0.0".to_string(),
            author: "Google".to_string(),
            license: "Apache 2.0".to_string(),
            icons,
            categories: vec![
                IconCategory {
                    id: "ui".to_string(),
                    name: "UI".to_string(),
                    description: Some("User interface icons".to_string()),
                    icon_count: 3,
                    parent: None,
                },
            ],
            variants: vec![IconVariant::Filled, IconVariant::Outline, IconVariant::TwoTone],
        };

        self.icon_sets.write().await.insert(pack.id.clone(), pack);
        Ok(())
    }

    /// Load Feather Icons
    async fn load_feather_icons(&self) -> Result<()> {
        let mut icons = Vec::new();
        
        // Common Feather Icons
        let feather_icons = vec![
            ("activity", "M22 12h-4l-3 9-4-18-3 9H2"),
            ("airplay", "M5 17H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2h-1 M12 15l5 6H7l5-6z"),
            ("alert-circle", "M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z M12 8v4 M12 16h.01"),
        ];

        for (name, path) in feather_icons {
            let svg = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="{}"/>
                </svg>"#,
                path
            );

            icons.push(Icon {
                id: format!("feather-{}", name),
                name: name.to_string(),
                set: "feather".to_string(),
                category: "general".to_string(),
                tags: vec![name.to_string(), "feather".to_string()],
                variant: IconVariant::Outline,
                svg,
                width: 24,
                height: 24,
                view_box: (0, 0, 24, 24),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                popularity: 900,
            });
        }

        let pack = IconPack {
            id: "feather".to_string(),
            name: "Feather Icons".to_string(),
            version: "4.29.0".to_string(),
            author: "Cole Bemis".to_string(),
            license: "MIT".to_string(),
            icons,
            categories: vec![
                IconCategory {
                    id: "general".to_string(),
                    name: "General".to_string(),
                    description: Some("General purpose icons".to_string()),
                    icon_count: 3,
                    parent: None,
                },
            ],
            variants: vec![IconVariant::Outline],
        };

        self.icon_sets.write().await.insert(pack.id.clone(), pack);
        Ok(())
    }

    /// Load Font Awesome icons
    async fn load_font_awesome(&self) -> Result<()> {
        let mut icons = Vec::new();
        
        // Font Awesome icons
        let fa_icons = vec![
            ("fa-solid", "circle", "M512 256c0 141.4-114.6 256-256 256S0 397.4 0 256 114.6 0 256 0s256 114.6 256 256z"),
            ("fa-regular", "circle", "M256 512c141.4 0 256-114.6 256-256S397.4 0 256 0 0 114.6 0 256s114.6 256 256 256zm0-96c-88.4 0-160-71.6-160-160s71.6-160 160-160 160 71.6 160 160-71.6 160-160 160z"),
            ("fa-brands", "github", "M256 0C114.6 0 0 114.6 0 256c0 113.1 73.4 209.1 175.2 242.9 12.8 2.4 17.5-5.6 17.5-12.4 0-6.1-0.2-22.2-0.3-43.6-71.3 15.5-86.3-34.3-86.3-34.3-11.7-29.7-28.5-37.6-28.5-37.6-23.3-15.9 1.8-15.6 1.8-15.6 25.8 1.8 39.4 26.5 39.4 26.5 22.9 39.2 60.1 27.9 74.8 21.3 2.3-16.6 9-27.9 16.3-34.3-57.1-6.5-117.1-28.5-117.1-127 0-28 10-50.9 26.5-68.9-2.7-6.5-11.5-32.7 2.5-68 0 0 21.6-6.9 71 26.1 20.6-5.7 42.7-8.6 64.6-8.6 22 0 44 2.9 64.6 8.6 49.3-33 70.9-26.1 70.9-26.1 14 35.3 5.2 61.5 2.5 68 16.5 18 26.5 40.9 26.5 68.9 0 98.7-60.1 120.4-117.3 126.8 9.2 7.9 17.5 23.6 17.5 47.6 0 34.4-0.3 62.1-0.3 70.5 0 6.9 4.6 15 17.5 12.4C438.6 465.1 512 369.1 512 256 512 114.6 397.4 0 256 0z"),
        ];

        for (style, name, path) in fa_icons {
            let svg = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
                    <path d="{}" fill="currentColor"/>
                </svg>"#,
                path
            );

            icons.push(Icon {
                id: format!("fa-{}", name),
                name: name.to_string(),
                set: "fontawesome".to_string(),
                category: style.to_string(),
                tags: vec![name.to_string(), "fontawesome".to_string()],
                variant: if style == "fa-solid" { IconVariant::Filled } else { IconVariant::Outline },
                svg,
                width: 512,
                height: 512,
                view_box: (0, 0, 512, 512),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                popularity: 800,
            });
        }

        let pack = IconPack {
            id: "fontawesome".to_string(),
            name: "Font Awesome".to_string(),
            version: "6.5.1".to_string(),
            author: "Fonticons".to_string(),
            license: "CC BY 4.0".to_string(),
            icons,
            categories: vec![
                IconCategory {
                    id: "fa-solid".to_string(),
                    name: "Solid".to_string(),
                    description: Some("Solid style icons".to_string()),
                    icon_count: 1,
                    parent: None,
                },
                IconCategory {
                    id: "fa-regular".to_string(),
                    name: "Regular".to_string(),
                    description: Some("Regular style icons".to_string()),
                    icon_count: 1,
                    parent: None,
                },
                IconCategory {
                    id: "fa-brands".to_string(),
                    name: "Brands".to_string(),
                    description: Some("Brand icons".to_string()),
                    icon_count: 1,
                    parent: None,
                },
            ],
            variants: vec![IconVariant::Filled, IconVariant::Outline],
        };

        self.icon_sets.write().await.insert(pack.id.clone(), pack);
        Ok(())
    }

    /// Load Heroicons
    async fn load_heroicons(&self) -> Result<()> {
        let mut icons = Vec::new();
        
        // Heroicons
        let heroicons = vec![
            ("academic-cap", "M12 14l9-5-9-5-9 5 9 5z M12 14l6.16-3.422a12.083 12.083 0 01.665 6.479A11.952 11.952 0 0012 20.055a11.952 11.952 0 00-6.824-2.998 12.078 12.078 0 01.665-6.479L12 14z M12 14l9-5-9-5-9 5 9 5zm0 0l6.16-3.422a12.083 12.083 0 01.665 6.479A11.952 11.952 0 0012 20.055a11.952 11.952 0 00-6.824-2.998 12.078 12.078 0 01.665-6.479L12 14z M12 20.055V14"),
            ("adjustments", "M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4"),
            ("archive", "M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4"),
        ];

        for (name, path) in heroicons {
            // Outline version
            let outline_svg = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" width="24" height="24">
                    <path stroke-linecap="round" stroke-linejoin="round" d="{}" />
                </svg>"#,
                path
            );

            icons.push(Icon {
                id: format!("heroicons-outline-{}", name),
                name: name.to_string(),
                set: "heroicons".to_string(),
                category: "outline".to_string(),
                tags: vec![name.to_string(), "heroicons".to_string(), "outline".to_string()],
                variant: IconVariant::Outline,
                svg: outline_svg,
                width: 24,
                height: 24,
                view_box: (0, 0, 24, 24),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                popularity: 700,
            });

            // Solid version
            let solid_svg = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" width="24" height="24">
                    <path d="{}" />
                </svg>"#,
                path
            );

            icons.push(Icon {
                id: format!("heroicons-solid-{}", name),
                name: name.to_string(),
                set: "heroicons".to_string(),
                category: "solid".to_string(),
                tags: vec![name.to_string(), "heroicons".to_string(), "solid".to_string()],
                variant: IconVariant::Filled,
                svg: solid_svg,
                width: 24,
                height: 24,
                view_box: (0, 0, 24, 24),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                popularity: 700,
            });
        }

        let pack = IconPack {
            id: "heroicons".to_string(),
            name: "Heroicons".to_string(),
            version: "2.1.1".to_string(),
            author: "Tailwind Labs".to_string(),
            license: "MIT".to_string(),
            icons,
            categories: vec![
                IconCategory {
                    id: "outline".to_string(),
                    name: "Outline".to_string(),
                    description: Some("Outline style icons".to_string()),
                    icon_count: 3,
                    parent: None,
                },
                IconCategory {
                    id: "solid".to_string(),
                    name: "Solid".to_string(),
                    description: Some("Solid style icons".to_string()),
                    icon_count: 3,
                    parent: None,
                },
            ],
            variants: vec![IconVariant::Outline, IconVariant::Filled],
        };

        self.icon_sets.write().await.insert(pack.id.clone(), pack);
        Ok(())
    }

    /// Load custom icon sets from directory
    async fn load_custom_sets(&self) -> Result<()> {
        let custom_dir = self.config.assets_dir.join("icons");
        if !custom_dir.exists() {
            return Ok(());
        }

        let mut read_dir = fs::read_dir(&custom_dir).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                self.load_icon_set_from_dir(&path).await?;
            }
        }

        Ok(())
    }

    /// Load icon set from directory
    async fn load_icon_set_from_dir(&self, dir: &Path) -> Result<()> {
        let manifest_path = dir.join("manifest.json");
        if !manifest_path.exists() {
            return Ok(());
        }

        let manifest_content = fs::read_to_string(manifest_path).await?;
        let manifest: serde_json::Value = serde_json::from_str(&manifest_content)?;

        let set_id = dir.file_name().unwrap_or_default().to_string_lossy().to_string();
        let set_name = manifest["name"].as_str().unwrap_or(&set_id).to_string();
        let set_version = manifest["version"].as_str().unwrap_or("1.0.0").to_string();
        let set_author = manifest["author"].as_str().unwrap_or("Unknown").to_string();
        let set_license = manifest["license"].as_str().unwrap_or("Unknown").to_string();

        let mut icons = Vec::new();
        let icons_dir = dir.join("icons");
        if icons_dir.exists() {
            let mut icon_files = fs::read_dir(&icons_dir).await?;
            while let Some(icon_file) = icon_files.next_entry().await? {
                let icon_path = icon_file.path();
                if icon_path.extension().and_then(|e| e.to_str()) == Some("svg") {
                    if let Ok(svg) = fs::read_to_string(&icon_path).await {
                        let name = icon_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                        
                        // Parse SVG for dimensions
                        let (width, height, view_box) = self.parse_svg_dimensions(&svg);

                        icons.push(Icon {
                            id: format!("{}-{}", set_id, name),
                            name,
                            set: set_id.clone(),
                            category: manifest["category"].as_str().unwrap_or("custom").to_string(),
                            tags: vec![],
                            variant: IconVariant::Custom("custom".to_string()),
                            svg,
                            width,
                            height,
                            view_box,
                            created_at: chrono::Utc::now(),
                            updated_at: chrono::Utc::now(),
                            popularity: 0,
                        });
                    }
                }
            }
        }

        let pack = IconPack {
            id: set_id,
            name: set_name,
            version: set_version,
            author: set_author,
            license: set_license,
            icons,
            categories: vec![],
            variants: vec![IconVariant::Custom("custom".to_string())],
        };

        self.icon_sets.write().await.insert(pack.id.clone(), pack);
        Ok(())
    }

    /// Parse SVG dimensions
    fn parse_svg_dimensions(&self, svg: &str) -> (u32, u32, (u32, u32, u32, u32)) {
        let mut width = 24;
        let mut height = 24;
        let mut view_box = (0, 0, 24, 24);

        // Extract width
        let re = regex::Regex::new(r#"width="(\d+)""#).unwrap();
        if let Some(caps) = re.captures(svg) {
            if let Ok(w) = caps[1].parse() {
                width = w;
            }
        }

        // Extract height
        let re = regex::Regex::new(r#"height="(\d+)""#).unwrap();
        if let Some(caps) = re.captures(svg) {
            if let Ok(h) = caps[1].parse() {
                height = h;
            }
        }

        // Extract viewBox
        let re = regex::Regex::new(r#"viewBox="(\d+)\s+(\d+)\s+(\d+)\s+(\d+)""#).unwrap();
        if let Some(caps) = re.captures(svg) {
            if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
                caps[1].parse(), caps[2].parse(), caps[3].parse(), caps[4].parse()
            ) {
                view_box = (x, y, w, h);
                width = w;
                height = h;
            }
        }

        (width, height, view_box)
    }

    /// Search icons
    pub async fn search(&self, query: &str, set: Option<&str>, category: Option<&str>) -> Result<Vec<Icon>> {
        let query = query.to_lowercase();
        let mut results = Vec::new();

        for pack in self.icon_sets.read().await.values() {
            if let Some(set_filter) = set {
                if pack.id != set_filter {
                    continue;
                }
            }

            for icon in &pack.icons {
                if let Some(cat_filter) = category {
                    if icon.category != cat_filter {
                        continue;
                    }
                }

                if icon.name.to_lowercase().contains(&query) ||
                   icon.tags.iter().any(|t| t.to_lowercase().contains(&query)) {
                    results.push(icon.clone());
                }
            }
        }

        Ok(results)
    }

    /// Get icon by ID
    pub async fn get_icon(&self, id: &str) -> Option<Icon> {
        for pack in self.icon_sets.read().await.values() {
            for icon in &pack.icons {
                if icon.id == id {
                    return Some(icon.clone());
                }
            }
        }
        None
    }

    /// Get icons by set
    pub async fn get_set_icons(&self, set_id: &str) -> Option<Vec<Icon>> {
        self.icon_sets.read().await.get(set_id).map(|pack| pack.icons.clone())
    }

    /// Get all icon sets
    pub async fn get_sets(&self) -> Vec<IconPack> {
        self.icon_sets.read().await.values().cloned().collect()
    }

    /// Get categories for set
    pub async fn get_categories(&self, set_id: &str) -> Vec<IconCategory> {
        if let Some(pack) = self.icon_sets.read().await.get(set_id) {
            pack.categories.clone()
        } else {
            Vec::new()
        }
    }

    /// Get icon from cache
    pub async fn get_cached(&self, icon_id: &str, size: IconSize) -> Option<Vec<u8>> {
        self.cache.read().await.get(icon_id).and_then(|sizes| sizes.get(&size).cloned())
    }

    /// Cache icon
    pub async fn cache_icon(&self, icon_id: &str, size: IconSize, data: Vec<u8>) {
        let mut cache = self.cache.write().await;
        let icon_cache = cache.entry(icon_id.to_string()).or_insert_with(HashMap::new);
        
        // Limit cache size
        if icon_cache.len() >= self.config.max_icon_cache {
            if let Some(oldest) = icon_cache.keys().next().cloned() {
                icon_cache.remove(&oldest);
            }
        }
        
        icon_cache.insert(size, data);
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }

    /// Add custom icon
    pub async fn add_custom_icon(&self, set_id: &str, icon: Icon) -> Result<()> {
        let mut sets = self.icon_sets.write().await;
        if let Some(pack) = sets.get_mut(set_id) {
            pack.icons.push(icon);
            
            // Update search index
            let mut index = self.search_index.write().await;
            for tag in &icon.tags {
                index.entry(tag.clone()).or_insert_with(Vec::new).push(icon.id.clone());
            }
        }
        Ok(())
    }

    /// Create new icon set
    pub async fn create_set(&self, name: String, author: String, license: String) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let pack = IconPack {
            id: id.clone(),
            name,
            version: "1.0.0".to_string(),
            author,
            license,
            icons: Vec::new(),
            categories: Vec::new(),
            variants: vec![IconVariant::Custom("custom".to_string())],
        };

        self.icon_sets.write().await.insert(id.clone(), pack);
        Ok(id)
    }

    /// Export icon set
    pub async fn export_set(&self, set_id: &str, format: &str) -> Result<Vec<u8>> {
        let pack = self.icon_sets.read().await.get(set_id)
            .ok_or_else(|| DesignError::IconNotFound(format!("Set not found: {}", set_id)))?
            .clone();

        match format {
            "json" => Ok(serde_json::to_vec_pretty(&pack)?),
            "yaml" => Ok(serde_yaml::to_string(&pack)?.into_bytes()),
            "zip" => self.create_zip_archive(&pack).await,
            _ => Err(DesignError::InvalidFormat(format!("Unsupported format: {}", format))),
        }
    }

    /// Create ZIP archive of icon set
    async fn create_zip_archive(&self, pack: &IconPack) -> Result<Vec<u8>> {
        use std::io::Cursor;
        use zip::{ZipWriter, write::FileOptions};

        let mut buffer = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(&mut buffer);

        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);

        // Add manifest
        let manifest = serde_json::json!({
            "name": pack.name,
            "version": pack.version,
            "author": pack.author,
            "license": pack.license,
            "icon_count": pack.icons.len(),
        });
        zip.start_file("manifest.json", options)?;
        zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;

        // Add icons
        for icon in &pack.icons {
            zip.start_file(format!("icons/{}.svg", icon.name), options)?;
            zip.write_all(icon.svg.as_bytes())?;
        }

        zip.finish()?;
        Ok(buffer.into_inner())
    }
}