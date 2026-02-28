//! Import/export functionality for configurations

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::fs;
use serde::{Serialize, Deserialize};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use tar::{Builder, Archive};
use base64::{Engine as _, engine::general_purpose};
use std::io::Read;

use crate::{
    Result, CustomizationError, CustomizationConfig,
    keybindings::Keymap,
    themes::Theme,
    layouts::WorkspaceLayout,
    profiles::ConfigurationProfile,
};

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Yaml,
    Toml,
    TarGz,
    Zip,
    Base64,
}

/// Import source
#[derive(Debug)]
pub enum ImportSource {
    File(PathBuf),
    String(String),
    Bytes(Vec<u8>),
    Url(String),
}

/// Export target
#[derive(Debug)]
pub enum ExportTarget {
    File(PathBuf),
    String,
    Bytes,
}

/// Package manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub dependencies: Vec<String>,
}

/// Configuration package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPackage {
    pub manifest: PackageManifest,
    pub keymap: Option<Keymap>,
    pub theme: Option<Theme>,
    pub layout: Option<WorkspaceLayout>,
    pub settings: HashMap<String, serde_json::Value>,
}

/// Import/Export manager
pub struct ImportExport {
    /// Configuration
    config: CustomizationConfig,
    /// Import history
    history: Arc<RwLock<Vec<ImportRecord>>>,
}

/// Import record
#[derive(Debug, Clone)]
pub struct ImportRecord {
    pub name: String,
    pub source: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub success: bool,
    pub error: Option<String>,
}

impl ImportExport {
    /// Create new import/export manager
    pub async fn new(config: CustomizationConfig) -> Result<Self> {
        Ok(Self {
            config,
            history: Arc::new(RwLock::new(Vec::with_capacity(100))),
        })
    }

    /// Export configuration package
    pub async fn export(&self, package: &ConfigPackage, format: ExportFormat) -> Result<Vec<u8>> {
        match format {
            ExportFormat::Json => self.export_json(package).await,
            ExportFormat::Yaml => self.export_yaml(package).await,
            ExportFormat::Toml => self.export_toml(package).await,
            ExportFormat::TarGz => self.export_targz(package).await,
            ExportFormat::Zip => self.export_zip(package).await,
            ExportFormat::Base64 => self.export_base64(package).await,
        }
    }

    /// Import configuration package
    pub async fn import(&self, data: &[u8], format: ExportFormat) -> Result<ConfigPackage> {
        let result = match format {
            ExportFormat::Json => self.import_json(data).await,
            ExportFormat::Yaml => self.import_yaml(data).await,
            ExportFormat::Toml => self.import_toml(data).await,
            ExportFormat::TarGz => self.import_targz(data).await,
            ExportFormat::Zip => self.import_zip(data).await,
            ExportFormat::Base64 => self.import_base64(data).await,
        };

        // Record import
        if let Ok(package) = &result {
            self.history.write().await.push(ImportRecord {
                name: package.manifest.name.clone(),
                source: "import".to_string(),
                timestamp: chrono::Utc::now(),
                success: true,
                error: None,
            });
        }

        result
    }

    /// Export as JSON
    async fn export_json(&self, package: &ConfigPackage) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec_pretty(package)?)
    }

    /// Import from JSON
    async fn import_json(&self, data: &[u8]) -> Result<ConfigPackage> {
        Ok(serde_json::from_slice(data)?)
    }

    /// Export as YAML
    async fn export_yaml(&self, package: &ConfigPackage) -> Result<Vec<u8>> {
        Ok(serde_yaml::to_string(package)?.into_bytes())
    }

    /// Import from YAML
    async fn import_yaml(&self, data: &[u8]) -> Result<ConfigPackage> {
        Ok(serde_yaml::from_slice(data)?)
    }

    /// Export as TOML
    async fn export_toml(&self, package: &ConfigPackage) -> Result<Vec<u8>> {
        Ok(toml::to_string(package).map_err(CustomizationError::TomlSer)?.into_bytes())
    }

    /// Import from TOML
    async fn import_toml(&self, data: &[u8]) -> Result<ConfigPackage> {
        Ok(toml::from_slice(data)?)
    }

    /// Export as tar.gz
    async fn export_targz(&self, package: &ConfigPackage) -> Result<Vec<u8>> {
        let mut tar_bytes = Vec::new();
        
        {
            let tar_encoder = GzEncoder::new(&mut tar_bytes, Compression::default());
            let mut tar_builder = Builder::new(tar_encoder);

            // Add manifest
            let manifest_json = serde_json::to_string_pretty(&package.manifest)?;
            tar_builder.append_data(
                &mut tar::Header::new_gnu(),
                "manifest.json",
                manifest_json.as_bytes(),
            )?;

            // Add keymap
            if let Some(keymap) = &package.keymap {
                let keymap_json = serde_json::to_string_pretty(keymap)?;
                tar_builder.append_data(
                    &mut tar::Header::new_gnu(),
                    "keymap.json",
                    keymap_json.as_bytes(),
                )?;
            }

            // Add theme
            if let Some(theme) = &package.theme {
                let theme_json = serde_json::to_string_pretty(theme)?;
                tar_builder.append_data(
                    &mut tar::Header::new_gnu(),
                    "theme.json",
                    theme_json.as_bytes(),
                )?;
            }

            // Add layout
            if let Some(layout) = &package.layout {
                let layout_json = serde_json::to_string_pretty(layout)?;
                tar_builder.append_data(
                    &mut tar::Header::new_gnu(),
                    "layout.json",
                    layout_json.as_bytes(),
                )?;
            }

            // Add settings
            let settings_json = serde_json::to_string_pretty(&package.settings)?;
            tar_builder.append_data(
                &mut tar::Header::new_gnu(),
                "settings.json",
                settings_json.as_bytes(),
            )?;

            tar_builder.finish()?;
        }

        Ok(tar_bytes)
    }

    /// Import from tar.gz
    async fn import_targz(&self, data: &[u8]) -> Result<ConfigPackage> {
        let tar_decoder = GzDecoder::new(data);
        let mut archive = Archive::new(tar_decoder);

        let mut manifest = None;
        let mut keymap = None;
        let mut theme = None;
        let mut layout = None;
        let mut settings = None;

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?.to_string_lossy().to_string();

            match path.as_str() {
                "manifest.json" => {
                    let mut content = Vec::new();
                    entry.read_to_end(&mut content)?;
                    manifest = Some(serde_json::from_slice(&content)?);
                }
                "keymap.json" => {
                    let mut content = Vec::new();
                    entry.read_to_end(&mut content)?;
                    keymap = Some(serde_json::from_slice(&content)?);
                }
                "theme.json" => {
                    let mut content = Vec::new();
                    entry.read_to_end(&mut content)?;
                    theme = Some(serde_json::from_slice(&content)?);
                }
                "layout.json" => {
                    let mut content = Vec::new();
                    entry.read_to_end(&mut content)?;
                    layout = Some(serde_json::from_slice(&content)?);
                }
                "settings.json" => {
                    let mut content = Vec::new();
                    entry.read_to_end(&mut content)?;
                    settings = Some(serde_json::from_slice(&content)?);
                }
                _ => {}
            }
        }

        Ok(ConfigPackage {
            manifest: manifest.ok_or_else(|| CustomizationError::ImportExportError("Missing manifest".to_string()))?,
            keymap,
            theme,
            layout,
            settings: settings.unwrap_or_default(),
        })
    }

    /// Export as zip (simplified - would need zip crate)
    async fn export_zip(&self, _package: &ConfigPackage) -> Result<Vec<u8>> {
        Err(CustomizationError::ImportExportError("ZIP export not implemented".to_string()))
    }

    /// Import from zip
    async fn import_zip(&self, _data: &[u8]) -> Result<ConfigPackage> {
        Err(CustomizationError::ImportExportError("ZIP import not implemented".to_string()))
    }

    /// Export as base64
    async fn export_base64(&self, package: &ConfigPackage) -> Result<Vec<u8>> {
        let json = self.export_json(package).await?;
        Ok(general_purpose::STANDARD.encode(json).into_bytes())
    }

    /// Import from base64
    async fn import_base64(&self, data: &[u8]) -> Result<ConfigPackage> {
        let decoded = general_purpose::STANDARD.decode(data).map_err(CustomizationError::Base64)?;
        self.import_json(&decoded).await
    }

    /// Import from file
    pub async fn import_from_file(&self, path: &Path) -> Result<ConfigPackage> {
        let data = fs::read(path).await?;
        let format = self.detect_format(path)?;
        self.import(&data, format).await
    }

    /// Export to file
    pub async fn export_to_file(&self, package: &ConfigPackage, path: &Path) -> Result<()> {
        let format = self.detect_format(path)?;
        let data = self.export(package, format).await?;
        fs::write(path, data).await?;
        Ok(())
    }

    /// Detect format from file extension
    fn detect_format(&self, path: &Path) -> Result<ExportFormat> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("json") => Ok(ExportFormat::Json),
            Some("yaml") | Some("yml") => Ok(ExportFormat::Yaml),
            Some("toml") => Ok(ExportFormat::Toml),
            Some("tar.gz") | Some("tgz") => Ok(ExportFormat::TarGz),
            Some("zip") => Ok(ExportFormat::Zip),
            Some("b64") => Ok(ExportFormat::Base64),
            _ => Err(CustomizationError::ImportExportError("Unknown file format".to_string())),
        }
    }

    /// Get import history
    pub async fn history(&self, limit: Option<usize>) -> Vec<ImportRecord> {
        let history = self.history.read().await;
        let limit = limit.unwrap_or(history.len());
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Validate package
    pub fn validate(&self, package: &ConfigPackage) -> Result<()> {
        if package.manifest.name.is_empty() {
            return Err(CustomizationError::ImportExportError("Package name cannot be empty".to_string()));
        }

        if package.manifest.version.is_empty() {
            return Err(CustomizationError::ImportExportError("Package version cannot be empty".to_string()));
        }

        Ok(())
    }

    /// Create package from current configuration
    pub async fn create_package(
        &self,
        name: String,
        description: Option<String>,
        author: Option<String>,
    ) -> Result<ConfigPackage> {
        // This would collect current configuration from managers
        Ok(ConfigPackage {
            manifest: PackageManifest {
                name,
                version: env!("CARGO_PKG_VERSION").to_string(),
                created_at: chrono::Utc::now(),
                description,
                author,
                dependencies: Vec::new(),
            },
            keymap: None,
            theme: None,
            layout: None,
            settings: HashMap::new(),
        })
    }
}