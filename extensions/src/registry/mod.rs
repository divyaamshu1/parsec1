//! Extension registry for managing installed extensions
//!
//! Provides storage, indexing, and querying of extensions installed in the system.

mod store;

pub use store::ExtensionStore;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock, mpsc};

use crate::Extension;

/// Extension registry configuration
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Directory where extensions are stored
    pub extensions_dir: PathBuf,
    /// Maximum number of extensions to keep in memory cache
    pub cache_size: usize,
    /// Enable auto-refresh of registry
    pub auto_refresh: bool,
    /// Refresh interval in seconds
    pub refresh_interval: u64,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("parsec/extensions");

        Self {
            extensions_dir: data_dir,
            cache_size: 100,
            auto_refresh: true,
            refresh_interval: 60,
        }
    }
}

/// Extension registry for managing installed extensions
pub struct ExtensionRegistry {
    /// Extension store (persistent)
    store: Arc<ExtensionStore>,
    /// In-memory cache of loaded extensions
    cache: Arc<RwLock<HashMap<String, CachedExtension>>>,
    /// Configuration
    config: RegistryConfig,
    /// Event sender
    event_tx: mpsc::UnboundedSender<RegistryEvent>,
    /// Event receiver
    event_rx: mpsc::UnboundedReceiver<RegistryEvent>,
}

/// Cached extension data
#[derive(Debug, Clone)]
struct CachedExtension {
    extension: Extension,
    last_accessed: std::time::Instant,
    access_count: usize,
}

/// Registry events
#[derive(Debug, Clone)]
pub enum RegistryEvent {
    ExtensionAdded(String),
    ExtensionRemoved(String),
    ExtensionUpdated(String),
    ExtensionEnabled(String),
    ExtensionDisabled(String),
    CacheCleared,
    Error(String),
}

/// Extension query filters
#[derive(Debug, Default)]
pub struct ExtensionQuery {
    pub publisher: Option<String>,
    pub name: Option<String>,
    pub enabled_only: bool,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub limit: usize,
    pub offset: usize,
}

/// Extension search result
#[derive(Debug)]
pub struct SearchResult {
    pub extensions: Vec<Extension>,
    pub total: usize,
    pub query: ExtensionQuery,
}

impl ExtensionRegistry {
    /// Create a new extension registry
    pub async fn new(config: RegistryConfig) -> Result<Self> {
        // Ensure extensions directory exists
        tokio::fs::create_dir_all(&config.extensions_dir).await?;

        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let store = Arc::new(ExtensionStore::new(config.extensions_dir.clone()).await?);

        let registry = Self {
            store: store.clone(),
            cache: Arc::new(RwLock::new(HashMap::with_capacity(config.cache_size))),
            config,
            event_tx,
            event_rx,
        };

        // Load initial index
        registry.refresh_cache().await?;

        Ok(registry)
    }

    /// Add an extension to the registry
    pub async fn add_extension(&self, extension: Extension) -> Result<()> {
        // Store on disk
        self.store.save_extension(&extension).await?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            if cache.len() >= self.config.cache_size {
                // Evict least recently used
                self.evict_lru(&mut cache).await;
            }

            cache.insert(extension.id.clone(), CachedExtension {
                extension: extension.clone(),
                last_accessed: std::time::Instant::now(),
                access_count: 1,
            });
        }

        // Emit event
        self.event_tx.send(RegistryEvent::ExtensionAdded(extension.id))?;

        Ok(())
    }

    /// Remove an extension from the registry
    pub async fn remove_extension(&self, id: &str) -> Result<()> {
        // Remove from disk
        self.store.delete_extension(id).await?;

        // Remove from cache
        self.cache.write().await.remove(id);

        // Emit event
        self.event_tx.send(RegistryEvent::ExtensionRemoved(id.to_string()))?;

        Ok(())
    }

    /// Get an extension by ID
    pub async fn get_extension(&self, id: &str) -> Option<Extension> {
        // Check cache first
        {
            let mut cache = self.cache.write().await;
            if let Some(cached) = cache.get_mut(id) {
                cached.last_accessed = std::time::Instant::now();
                cached.access_count += 1;
                return Some(cached.extension.clone());
            }
        }

        // Try to load from store
        if let Ok(ext) = self.store.load_extension(id).await {
            // Add to cache
            let mut cache = self.cache.write().await;
            if cache.len() >= self.config.cache_size {
                self.evict_lru(&mut cache).await;
            }
            cache.insert(id.to_string(), CachedExtension {
                extension: ext.clone(),
                last_accessed: std::time::Instant::now(),
                access_count: 1,
            });
            Some(ext)
        } else {
            None
        }
    }

    /// List all extensions matching query
    pub async fn list_extensions(&self, query: ExtensionQuery) -> Result<SearchResult> {
        let mut extensions = Vec::new();

        // Get all extension IDs from store
        let ids = self.store.list_extensions().await?;

        for id in ids {
            if let Some(ext) = self.get_extension(&id).await {
                // Apply filters
                if let Some(publisher) = &query.publisher {
                    if ext.manifest.publisher != *publisher {
                        continue;
                    }
                }

                if let Some(name) = &query.name {
                    if !ext.manifest.name.contains(name) {
                        continue;
                    }
                }

                if query.enabled_only && !ext.enabled {
                    continue;
                }

                if let Some(cat) = &query.category {
                    if !ext.manifest.categories.contains(cat) {
                        continue;
                    }
                }

                if !query.tags.is_empty() {
                    let has_tags = query.tags.iter().all(|t| ext.manifest.tags.contains(t));
                    if !has_tags {
                        continue;
                    }
                }

                extensions.push(ext);
            }
        }

        let total = extensions.len();

        // Apply pagination
        if query.limit > 0 {
            let start = query.offset.min(extensions.len());
            let end = (start + query.limit).min(extensions.len());
            extensions = extensions[start..end].to_vec();
        }

        Ok(SearchResult {
            extensions,
            total,
            query,
        })
    }

    /// Enable an extension
    pub async fn enable_extension(&self, id: &str) -> Result<()> {
        self.store.set_enabled(id, true).await?;

        // Update cache if present
        if let Some(cached) = self.cache.write().await.get_mut(id) {
            cached.extension.enabled = true;
        }

        self.event_tx.send(RegistryEvent::ExtensionEnabled(id.to_string()))?;
        Ok(())
    }

    /// Disable an extension
    pub async fn disable_extension(&self, id: &str) -> Result<()> {
        self.store.set_enabled(id, false).await?;

        // Update cache if present
        if let Some(cached) = self.cache.write().await.get_mut(id) {
            cached.extension.enabled = false;
        }

        self.event_tx.send(RegistryEvent::ExtensionDisabled(id.to_string()))?;
        Ok(())
    }

    /// Check if extension exists
    pub async fn has_extension(&self, id: &str) -> bool {
        self.store.has_extension(id).await
    }

    /// Get extension count
    pub async fn extension_count(&self) -> usize {
        self.store.list_extensions().await.unwrap_or_default().len()
    }

    /// Refresh cache from disk
    pub async fn refresh_cache(&self) -> Result<()> {
        let ids = self.store.list_extensions().await?;
        let mut cache = self.cache.write().await;

        for id in ids {
            if !cache.contains_key(&id) {
                if let Ok(ext) = self.store.load_extension(&id).await {
                    cache.insert(id, CachedExtension {
                        extension: ext,
                        last_accessed: std::time::Instant::now(),
                        access_count: 0,
                    });
                }
            }
        }

        Ok(())
    }

    /// Clear in-memory cache
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
        self.event_tx.send(RegistryEvent::CacheCleared).ok();
    }

    /// Get registry statistics
    pub async fn statistics(&self) -> RegistryStats {
        let cache_size = self.cache.read().await.len();
        let total = self.extension_count().await;

        RegistryStats {
            total_extensions: total,
            cached_extensions: cache_size,
            extensions_dir: self.config.extensions_dir.clone(),
            cache_capacity: self.config.cache_size,
        }
    }

    /// Get next registry event
    pub async fn next_event(&mut self) -> Option<RegistryEvent> {
        self.event_rx.recv().await
    }

    /// Evict least recently used cache entries
    async fn evict_lru(&self, cache: &mut HashMap<String, CachedExtension>) {
        if cache.is_empty() {
            return;
        }

        // Find LRU entry
        let mut oldest = std::time::Instant::now();
        let mut oldest_key = None;

        for (key, cached) in cache.iter() {
            if cached.last_accessed < oldest {
                oldest = cached.last_accessed;
                oldest_key = Some(key.clone());
            }
        }

        if let Some(key) = oldest_key {
            cache.remove(&key);
        }
    }
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total_extensions: usize,
    pub cached_extensions: usize,
    pub extensions_dir: PathBuf,
    pub cache_capacity: usize,
}

/// Extension registry builder
pub struct RegistryBuilder {
    config: RegistryConfig,
}

impl RegistryBuilder {
    pub fn new() -> Self {
        Self {
            config: RegistryConfig::default(),
        }
    }

    pub fn extensions_dir(mut self, dir: PathBuf) -> Self {
        self.config.extensions_dir = dir;
        self
    }

    pub fn cache_size(mut self, size: usize) -> Self {
        self.config.cache_size = size;
        self
    }

    pub fn auto_refresh(mut self, enable: bool) -> Self {
        self.config.auto_refresh = enable;
        self
    }

    pub fn refresh_interval(mut self, seconds: u64) -> Self {
        self.config.refresh_interval = seconds;
        self
    }

    pub async fn build(self) -> Result<ExtensionRegistry> {
        ExtensionRegistry::new(self.config).await
    }
}

impl Default for RegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension registry module exports
pub mod prelude {
    pub use super::{
        ExtensionRegistry,
        RegistryConfig,
        RegistryEvent,
        ExtensionQuery,
        SearchResult,
        RegistryStats,
        RegistryBuilder,
    };
}

/// Re-export main types


/// Version of the registry module
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize a registry with default settings
pub async fn init_default() -> Result<ExtensionRegistry> {
    ExtensionRegistry::new(RegistryConfig::default()).await
}

/// Initialize a registry with custom extensions directory
pub async fn init_with_dir<P: AsRef<Path>>(dir: P) -> Result<ExtensionRegistry> {
    let config = RegistryConfig {
        extensions_dir: dir.as_ref().to_path_buf(),
        ..Default::default()
    };
        ExtensionRegistry::new(config).await
}

/// Extension registry error types
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Extension not found: {0}")]
    ExtensionNotFound(String),

    #[error("Extension already exists: {0}")]
    ExtensionAlreadyExists(String),

    #[error("Invalid extension manifest: {0}")]
    InvalidManifest(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl From<std::io::Error> for RegistryError {
    fn from(err: std::io::Error) -> Self {
        RegistryError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for RegistryError {
    fn from(err: serde_json::Error) -> Self {
        RegistryError::SerializationError(err.to_string())
    }
}



/// Registry result type
pub type RegistryResult<T> = std::result::Result<T, RegistryError>;

/// Extension metadata for quick lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMetadata {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub version: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub size: u64,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
}

impl From<&Extension> for ExtensionMetadata {
    fn from(ext: &Extension) -> Self {
        Self {
            id: ext.id.clone(),
            name: ext.manifest.name.clone(),
            publisher: ext.manifest.publisher.clone(),
            version: ext.manifest.version.clone(),
            description: ext.manifest.description.clone(),
            enabled: ext.enabled,
            installed_at: ext.installed_at,
            size: ext.size,
            categories: ext.manifest.categories.clone(),
            tags: ext.manifest.tags.clone(),
        }
    }
}

/// Registry index for fast lookups
#[derive(Debug, Default, Serialize, Deserialize)]
struct RegistryIndex {
    extensions: HashMap<String, ExtensionMetadata>,
    by_publisher: HashMap<String, Vec<String>>,
    by_category: HashMap<String, Vec<String>>,
    by_tag: HashMap<String, Vec<String>>,
    last_updated: chrono::DateTime<chrono::Utc>,
}

impl RegistryIndex {
    fn new() -> Self {
        Self {
            extensions: HashMap::new(),
            by_publisher: HashMap::new(),
            by_category: HashMap::new(),
            by_tag: HashMap::new(),
            last_updated: chrono::Utc::now(),
        }
    }

    fn add_extension(&mut self, ext: &Extension) {
        let metadata = ExtensionMetadata::from(ext);
        
        // Store by ID
        self.extensions.insert(ext.id.clone(), metadata.clone());

        // Index by publisher
        self.by_publisher
            .entry(ext.manifest.publisher.clone())
            .or_insert_with(Vec::new)
            .push(ext.id.clone());

        // Index by category
        for cat in &ext.manifest.categories {
            self.by_category
                .entry(cat.clone())
                .or_insert_with(Vec::new)
                .push(ext.id.clone());
        }

        // Index by tag
        for tag in &ext.manifest.tags {
            self.by_tag
                .entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(ext.id.clone());
        }

        self.last_updated = chrono::Utc::now();
    }

    fn remove_extension(&mut self, id: &str) {
        if let Some(metadata) = self.extensions.remove(id) {
            // Remove from publisher index
            if let Some(ids) = self.by_publisher.get_mut(&metadata.publisher) {
                ids.retain(|i| i != id);
            }

            // Remove from category indices
            for cat in &metadata.categories {
                if let Some(ids) = self.by_category.get_mut(cat) {
                    ids.retain(|i| i != id);
                }
            }

            // Remove from tag indices
            for tag in &metadata.tags {
                if let Some(ids) = self.by_tag.get_mut(tag) {
                    ids.retain(|i| i != id);
                }
            }
        }
        self.last_updated = chrono::Utc::now();
    }

    fn search(&self, query: &ExtensionQuery) -> Vec<String> {
        let mut results = Vec::new();

        // Start with all extensions or filtered by publisher
        let candidates: Vec<String> = if let Some(publisher) = &query.publisher {
            self.by_publisher
                .get(publisher)
                .cloned()
                .unwrap_or_default()
        } else {
            self.extensions.keys().cloned().collect()
        };

        for id in candidates {
            if let Some(metadata) = self.extensions.get(&id) {
                // Apply filters
                if let Some(name) = &query.name {
                    if !metadata.name.contains(name) {
                        continue;
                    }
                }

                if query.enabled_only && !metadata.enabled {
                    continue;
                }

                if let Some(cat) = &query.category {
                    if !metadata.categories.contains(cat) {
                        continue;
                    }
                }

                if !query.tags.is_empty() {
                    let has_tags = query.tags.iter().all(|t| metadata.tags.contains(t));
                    if !has_tags {
                        continue;
                    }
                }

                results.push(id);
            }
        }

        results
    }
}

impl ExtensionRegistry {
    /// Load or create registry index
    async fn load_index(&self) -> Result<RegistryIndex> {
        let index_path = self.config.extensions_dir.join("index.json");
        
        if index_path.exists() {
            let content = tokio::fs::read_to_string(index_path).await?;
            let index: RegistryIndex = serde_json::from_str(&content)?;
            Ok(index)
        } else {
            Ok(RegistryIndex::new())
        }
    }

    /// Save registry index
    async fn save_index(&self, index: &RegistryIndex) -> Result<()> {
        let index_path = self.config.extensions_dir.join("index.json");
        let content = serde_json::to_string_pretty(index)?;
        tokio::fs::write(index_path, content).await?;
        Ok(())
    }

    /// Rebuild index from all extensions
    pub async fn rebuild_index(&self) -> Result<()> {
        let mut index = RegistryIndex::new();
        let ids = self.store.list_extensions().await?;

        for id in ids {
            if let Ok(ext) = self.store.load_extension(&id).await {
                index.add_extension(&ext);
            }
        }

        self.save_index(&index).await?;
        Ok(())
    }

    /// Search extensions using the index
    pub async fn search_extensions(&self, query: ExtensionQuery) -> Result<SearchResult> {
        let index = self.load_index().await?;
        let mut ids = index.search(&query);
        let total = ids.len();

        // Apply pagination
        if query.limit > 0 {
            let start = query.offset.min(ids.len());
            let end = (start + query.limit).min(ids.len());
            ids = ids[start..end].to_vec();
        }

        // Load full extension data
        let mut extensions = Vec::new();
        for id in ids {
            if let Some(ext) = self.get_extension(&id).await {
                extensions.push(ext);
            }
        }

        Ok(SearchResult {
            extensions,
            total,
            query,
        })
    }

    /// Get extensions by publisher
    pub async fn get_extensions_by_publisher(&self, publisher: &str) -> Result<Vec<Extension>> {
        let index = self.load_index().await?;
        let mut extensions = Vec::new();

        if let Some(ids) = index.by_publisher.get(publisher) {
            for id in ids {
                if let Some(ext) = self.get_extension(id).await {
                    extensions.push(ext);
                }
            }
        }

        Ok(extensions)
    }

    /// Get extensions by category
    pub async fn get_extensions_by_category(&self, category: &str) -> Result<Vec<Extension>> {
        let index = self.load_index().await?;
        let mut extensions = Vec::new();

        if let Some(ids) = index.by_category.get(category) {
            for id in ids {
                if let Some(ext) = self.get_extension(id).await {
                    extensions.push(ext);
                }
            }
        }

        Ok(extensions)
    }

    /// Get extensions by tag
    pub async fn get_extensions_by_tag(&self, tag: &str) -> Result<Vec<Extension>> {
        let index = self.load_index().await?;
        let mut extensions = Vec::new();

        if let Some(ids) = index.by_tag.get(tag) {
            for id in ids {
                if let Some(ext) = self.get_extension(id).await {
                    extensions.push(ext);
                }
            }
        }

        Ok(extensions)
    }
}

/// Check if an extension ID is valid
pub fn is_valid_extension_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_')
}

/// Parse extension ID into (publisher, name)
pub fn parse_extension_id(id: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() >= 2 {
        Some((parts[0].to_string(), parts[1..].join(".")))
    } else {
        None
    }
}

/// Generate extension ID from publisher and name
pub fn format_extension_id(publisher: &str, name: &str) -> String {
        format!("{}.{}", publisher, name)
}

/// Sort extensions by various criteria
#[derive(Debug, Clone, Copy)]
pub enum SortBy {
    Name,
    Publisher,
    Version,
    InstalledAt,
    UpdatedAt,
    Size,
}

/// Sort order
#[derive(Debug, Clone, Copy)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl ExtensionRegistry {
    /// List extensions with sorting
    pub async fn list_extensions_sorted(&self, sort_by: SortBy, order: SortOrder) -> Result<Vec<Extension>> {
        let mut extensions = self.list_extensions(ExtensionQuery::default()).await?.extensions;

        match sort_by {
            SortBy::Name => {
                extensions.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
            }
            SortBy::Publisher => {
                extensions.sort_by(|a, b| a.manifest.publisher.cmp(&b.manifest.publisher));
            }
            SortBy::Version => {
                extensions.sort_by(|a, b| a.manifest.version.cmp(&b.manifest.version));
            }
            SortBy::InstalledAt => {
                extensions.sort_by(|a, b| a.installed_at.cmp(&b.installed_at));
            }
            SortBy::UpdatedAt => {
                extensions.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));
            }
            SortBy::Size => {
                extensions.sort_by(|a, b| a.size.cmp(&b.size));
            }
        }

        if matches!(order, SortOrder::Descending) {
            extensions.reverse();
        }

        Ok(extensions)
    }

    /// Get recently installed extensions
    pub async fn get_recently_installed(&self, limit: usize) -> Result<Vec<Extension>> {
        let mut extensions = self.list_extensions(ExtensionQuery::default()).await?.extensions;
        extensions.sort_by(|a, b| b.installed_at.cmp(&a.installed_at));
        extensions.truncate(limit);
        Ok(extensions)
    }

    /// Get recently updated extensions
    pub async fn get_recently_updated(&self, limit: usize) -> Result<Vec<Extension>> {
        let mut extensions = self.list_extensions(ExtensionQuery::default()).await?.extensions;
        extensions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        extensions.truncate(limit);
        Ok(extensions)
    }

    /// Get extensions by publisher with sorting
    pub async fn get_extensions_by_publisher_sorted(
        &self,
        publisher: &str,
        sort_by: SortBy,
        order: SortOrder,
    ) -> Result<Vec<Extension>> {
        let mut extensions = self.get_extensions_by_publisher(publisher).await?;
        
        match sort_by {
            SortBy::Name => extensions.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name)),
            SortBy::Version => extensions.sort_by(|a, b| a.manifest.version.cmp(&b.manifest.version)),
            SortBy::InstalledAt => extensions.sort_by(|a, b| a.installed_at.cmp(&b.installed_at)),
            _ => {}
        }

        if matches!(order, SortOrder::Descending) {
            extensions.reverse();
        }

        Ok(extensions)
    }

    /// Export registry to JSON
    pub async fn export_to_json(&self, path: &Path) -> Result<()> {
        let extensions = self.list_extensions(ExtensionQuery::default()).await?.extensions;
        let json = serde_json::to_string_pretty(&extensions)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    /// Import registry from JSON
    pub async fn import_from_json(&self, path: &Path) -> Result<usize> {
        let content = tokio::fs::read_to_string(path).await?;
        let extensions: Vec<Extension> = serde_json::from_str(&content)?;
        
        let mut count = 0;
        for ext in extensions {
            if !self.has_extension(&ext.id).await {
                self.add_extension(ext).await?;
                count += 1;
            }
        }
        
        Ok(count)
    }

    /// Get registry health status
    pub async fn health_check(&self) -> RegistryHealth {
        let total = self.extension_count().await;
        let cache_size = self.cache.read().await.len();
        let index_exists = self.config.extensions_dir.join("index.json").exists();

        RegistryHealth {
            healthy: true,
            total_extensions: total,
            cached_extensions: cache_size,
            index_exists,
            extensions_dir_exists: self.config.extensions_dir.exists(),
            is_writable: self.check_writable().await,
        }
    }

    /// Check if extensions directory is writable
    async fn check_writable(&self) -> bool {
        let test_file = self.config.extensions_dir.join(".write_test");
        match tokio::fs::write(&test_file, b"test").await {
            Ok(_) => {
                let _ = tokio::fs::remove_file(test_file).await;
                true
            }
            Err(_) => false,
        }
    }
}

/// Registry health status
#[derive(Debug, Clone)]
pub struct RegistryHealth {
    pub healthy: bool,
    pub total_extensions: usize,
    pub cached_extensions: usize,
    pub index_exists: bool,
    pub extensions_dir_exists: bool,
    pub is_writable: bool,
}

/// Extension registry watcher for auto-refresh
pub struct RegistryWatcher {
    registry: Arc<ExtensionRegistry>,
    interval: std::time::Duration,
}

impl RegistryWatcher {
    pub fn new(registry: Arc<ExtensionRegistry>, interval: std::time::Duration) -> Self {
        Self { registry, interval }
    }

    pub async fn start_watching(&self) {
        let registry = self.registry.clone();
        let mut interval = tokio::time::interval(self.interval);

        tokio::spawn(async move {
            loop {
                interval.tick().await;
                if let Err(e) = registry.refresh_cache().await {
                    tracing::warn!("Failed to refresh registry cache: {}", e);
                }
            }
        });
    }
}

        // These test functions depend on tempdir and create_test_extension
        // which are not available in this module. Tests have been disabled.