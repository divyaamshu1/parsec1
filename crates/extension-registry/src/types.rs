use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub extension_id: String,
    #[serde(default)]
    pub extension_name: String,
    #[serde(default)]
    pub display_name: String,
    pub version: String,
    pub publisher: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub rating: f32,
    #[serde(default)]
    pub rating_count: u32,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub readme_url: Option<String>,
    #[serde(default)]
    pub changelog_url: Option<String>,
    #[serde(default)]
    pub release_date: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub last_updated: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub dependencies: Vec<ExtensionDependency>,
    #[serde(default)]
    pub extension_pack: Vec<String>,
    #[serde(default)]
    pub engines: HashMap<String, String>,
    #[serde(default)]
    pub categories_labels: Vec<CategoryInfo>,
}

/// Publisher information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherInfo {
    pub publisher_id: String,
    pub publisher_name: String,
    pub display_name: String,
    pub domain: Option<String>,
    pub verified: bool,
}

/// Extension dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionDependency {
    pub extension_id: String,
    pub version: String,
    pub optional: bool,
}

/// Category information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryInfo {
    pub category_id: String,
    pub category_name: String,
    pub category_label: String,
}

/// Search query parameters
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub text: String,
    pub categories: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub publisher: Option<String>,
    pub page: usize,
    pub page_size: usize,
    pub sort_by: SortBy,
    pub sort_order: SortOrder,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            text: String::new(),
            categories: None,
            tags: None,
            publisher: None,
            page: 1,
            page_size: 50,
            sort_by: SortBy::Relevance,
            sort_order: SortOrder::Descending,
        }
    }
}

/// Sort by options
#[derive(Debug, Clone, Copy)]
pub enum SortBy {
    Relevance,
    Downloads,
    Rating,
    Updated,
    Published,
    Name,
}

/// Sort order
#[derive(Debug, Clone, Copy)]
pub enum SortOrder {
    Ascending,
    Descending,
}

/// Search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub total: usize,
    pub extensions: Vec<ExtensionInfo>,
    pub page: usize,
    pub page_size: usize,
    pub has_more: bool,
}

/// Extension version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionVersion {
    pub version: String,
    pub target_platform: Option<String>,
    pub engine_version: String,
    pub asset_uri: String,
    pub file_size: u64,
    pub release_date: chrono::DateTime<chrono::Utc>,
    pub is_pre_release: bool,
}

/// Download result
#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub extension_id: String,
    pub version: String,
    pub local_path: PathBuf,
    pub file_size: u64,
    pub integrity_hash: Option<String>,
}

/// Marketplace error types
#[derive(Debug, thiserror::Error)]
pub enum MarketplaceError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Extension not found: {0}")]
    NotFound(String),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

#[derive(Debug, Clone)]
pub struct InstalledExtension {
    pub id: String,
    pub path: PathBuf,
    pub manifest: ExtensionManifest,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub entry: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Extension not found: {0}")]
    NotFound(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}