//! Long-term memory storage for AI

use std::collections::HashMap;
use std::sync::Arc;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Memory manager for persistent storage
pub struct MemoryManager {
    storage: Arc<RwLock<HashMap<String, MemoryItem>>>,
}

/// Memory item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub key: String,
    pub value: String,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub accessed_at: DateTime<Utc>,
    pub access_count: usize,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a memory
    pub async fn store(&self, key: &str, value: &str, metadata: HashMap<String, String>) -> Result<()> {
        let now = Utc::now();
        let item = MemoryItem {
            key: key.to_string(),
            value: value.to_string(),
            metadata,
            created_at: now,
            accessed_at: now,
            access_count: 1,
        };

        self.storage.write().insert(key.to_string(), item);
        Ok(())
    }

    /// Retrieve a memory
    pub async fn retrieve(&self, key: &str) -> Result<Option<String>> {
        let mut storage = self.storage.write();
        
        if let Some(item) = storage.get_mut(key) {
            item.accessed_at = Utc::now();
            item.access_count += 1;
            Ok(Some(item.value.clone()))
        } else {
            Ok(None)
        }
    }

    /// Search memories by prefix
    pub async fn search(&self, prefix: &str) -> Result<Vec<MemoryItem>> {
        let storage = self.storage.read();
        let mut results = Vec::new();

        for (key, item) in storage.iter() {
            if key.starts_with(prefix) {
                results.push(item.clone());
            }
        }

        Ok(results)
    }

    /// Delete a memory
    pub async fn delete(&self, key: &str) -> Result<()> {
        self.storage.write().remove(key);
        Ok(())
    }

    /// Clear all memories
    pub async fn clear(&self) -> Result<()> {
        self.storage.write().clear();
        Ok(())
    }

    /// Get memory count
    pub fn len(&self) -> usize {
        self.storage.read().len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.storage.read().is_empty()
    }

    /// Get most recently accessed
    pub async fn get_recent(&self, limit: usize) -> Vec<MemoryItem> {
        let storage = self.storage.read();
        let mut items: Vec<_> = storage.values().cloned().collect();
        items.sort_by(|a, b| b.accessed_at.cmp(&a.accessed_at));
        items.truncate(limit);
        items
    }

    /// Get most frequently accessed
    pub async fn get_frequent(&self, limit: usize) -> Vec<MemoryItem> {
        let storage = self.storage.read();
        let mut items: Vec<_> = storage.values().cloned().collect();
        items.sort_by(|a, b| b.access_count.cmp(&a.access_count));
        items.truncate(limit);
        items
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory() {
        let mem = MemoryManager::new();
        
        mem.store("test", "value", HashMap::new()).await.unwrap();
        
        let retrieved = mem.retrieve("test").await.unwrap();
        assert_eq!(retrieved, Some("value".to_string()));

        let search = mem.search("te").await.unwrap();
        assert_eq!(search.len(), 1);

        mem.delete("test").await.unwrap();
        assert_eq!(mem.len(), 0);
    }

    #[tokio::test]
    async fn test_recent_and_frequent() {
        let mem = MemoryManager::new();
        
        mem.store("a", "1", HashMap::new()).await.unwrap();
        mem.store("b", "2", HashMap::new()).await.unwrap();
        
        // Access 'a' twice
        mem.retrieve("a").await.unwrap();
        mem.retrieve("a").await.unwrap();
        
        let recent = mem.get_recent(2).await;
        assert_eq!(recent.len(), 2);
        
        let frequent = mem.get_frequent(2).await;
        assert_eq!(frequent[0].key, "a"); // Most accessed
        }
}

/// Memory statistics for monitoring
#[derive(Debug, Default)]
pub struct MemoryStats {
    pub total_items: usize,
    pub total_accesses: usize,
    pub oldest_item: Option<DateTime<Utc>>,
    pub newest_item: Option<DateTime<Utc>>,
    pub most_accessed: Option<String>,
    pub total_memory_bytes: usize,
}

impl MemoryManager {
    /// Get memory statistics
    pub async fn stats(&self) -> MemoryStats {
        let storage = self.storage.read();
        let mut stats = MemoryStats::default();
        
        stats.total_items = storage.len();
        
        let mut total_accesses = 0;
        let mut oldest = None;
        let mut newest = None;
        let mut most_accessed = 0;
        let mut most_accessed_key = None;

        for item in storage.values() {
            total_accesses += item.access_count;
            
            if oldest.is_none() || item.created_at < oldest.unwrap() {
                oldest = Some(item.created_at);
            }
            if newest.is_none() || item.created_at > newest.unwrap() {
                newest = Some(item.created_at);
            }
            if item.access_count > most_accessed {
                most_accessed = item.access_count;
                most_accessed_key = Some(item.key.clone());
            }
            
            stats.total_memory_bytes += item.value.len() + item.key.len();
            for (k, v) in &item.metadata {
                stats.total_memory_bytes += k.len() + v.len();
            }
        }

        stats.total_accesses = total_accesses;
        stats.oldest_item = oldest;
        stats.newest_item = newest;
        stats.most_accessed = most_accessed_key;

        stats
    }

    /// Prune old memories
    pub async fn prune(&self, older_than: chrono::Duration) -> Result<usize> {
        let mut storage = self.storage.write();
        let cutoff = Utc::now() - older_than;
        let before = storage.len();
        
        storage.retain(|_, item| item.accessed_at > cutoff);
        
        Ok(before - storage.len())
    }

    /// Export memories to JSON
    pub async fn export(&self) -> Result<String> {
        let storage = self.storage.read();
        let items: Vec<_> = storage.values().collect();
        Ok(serde_json::to_string_pretty(&items)?)
    }

    /// Import memories from JSON
    pub async fn import(&self, json: &str) -> Result<usize> {
        let items: Vec<MemoryItem> = serde_json::from_str(json)?;
        let mut storage = self.storage.write();
        
        for item in items {
            storage.insert(item.key.clone(), item);
        }
        
        Ok(storage.len())
    }

    /// Batch store multiple memories
    pub async fn store_batch(&self, items: Vec<(String, String, HashMap<String, String>)>) -> Result<usize> {
        let mut storage = self.storage.write();
        let now = Utc::now();
        
        for (key, value, metadata) in items {
            let mem_key = key.clone();
            storage.insert(key, MemoryItem {
                key: mem_key,
                value,
                metadata,
                created_at: now,
                accessed_at: now,
                access_count: 1,
            });
        }
        
        Ok(storage.len())
    }

    /// Find memories by metadata value
    pub async fn find_by_metadata(&self, meta_key: &str, meta_value: &str) -> Vec<MemoryItem> {
        let storage = self.storage.read();
        let mut results = Vec::new();
        
        for item in storage.values() {
            if let Some(value) = item.metadata.get(meta_key) {
                if value == meta_value {
                    results.push(item.clone());
                }
            }
        }
        
        results
    }
}

/// Memory TTL manager for auto-expiration
pub struct MemoryTTL {
    ttl: chrono::Duration,
    check_interval: chrono::Duration,
}

impl MemoryTTL {
    pub fn new(ttl: chrono::Duration, check_interval: chrono::Duration) -> Self {
        Self { ttl, check_interval }
    }

    /// Start TTL monitoring
    pub async fn start_monitoring(&self, manager: Arc<MemoryManager>) {
        let ttl = self.ttl;
        let mut interval = tokio::time::interval(self.check_interval.to_std().unwrap());
        
        tokio::spawn(async move {
            loop {
                interval.tick().await;
                let _ = manager.prune(ttl).await;
            }
        });
    }
}

/// Memory namespaces for different contexts
pub struct MemoryNamespace {
    manager: Arc<MemoryManager>,
    namespace: String,
}

impl MemoryNamespace {
    pub fn new(manager: Arc<MemoryManager>, namespace: &str) -> Self {
        Self {
            manager,
            namespace: namespace.to_string(),
        }
    }

    fn make_key(&self, key: &str) -> String {
        format!("{}:{}", self.namespace, key)
    }

    pub async fn store(&self, key: &str, value: &str, metadata: HashMap<String, String>) -> Result<()> {
        self.manager.store(&self.make_key(key), value, metadata).await
    }

    pub async fn retrieve(&self, key: &str) -> Result<Option<String>> {
        self.manager.retrieve(&self.make_key(key)).await
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        self.manager.delete(&self.make_key(key)).await
    }

    pub async fn list(&self) -> Result<Vec<MemoryItem>> {
        let prefix = &format!("{}:", self.namespace);
        self.manager.search(prefix).await
    }

    pub async fn clear(&self) -> Result<()> {
        let items = self.list().await?;
        for item in items {
            self.manager.delete(&item.key).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod advanced_tests {
    use super::*;

    #[tokio::test]
    async fn test_namespace() {
        let manager = Arc::new(MemoryManager::new());
        let ns1 = MemoryNamespace::new(manager.clone(), "user1");
        let ns2 = MemoryNamespace::new(manager.clone(), "user2");

        ns1.store("key", "value1", HashMap::new()).await.unwrap();
        ns2.store("key", "value2", HashMap::new()).await.unwrap();

        let v1 = ns1.retrieve("key").await.unwrap();
        let v2 = ns2.retrieve("key").await.unwrap();

        assert_eq!(v1, Some("value1".to_string()));
        assert_eq!(v2, Some("value2".to_string()));
    }

    #[tokio::test]
    async fn test_stats() {
        let mem = MemoryManager::new();
        
        mem.store("a", "1", HashMap::new()).await.unwrap();
        mem.store("b", "2", HashMap::new()).await.unwrap();
        
        mem.retrieve("a").await.unwrap();
        mem.retrieve("a").await.unwrap();

        let stats = mem.stats().await;
        assert_eq!(stats.total_items, 2);
        assert_eq!(stats.total_accesses, 3);
        assert_eq!(stats.most_accessed, Some("a".to_string()));
        assert!(stats.total_memory_bytes > 0);
    }

    #[tokio::test]
    async fn test_prune() {
        let mem = MemoryManager::new();
        
        mem.store("old", "1", HashMap::new()).await.unwrap();
        
        // Wait a bit
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        
        mem.store("new", "2", HashMap::new()).await.unwrap();

        let pruned = mem.prune(chrono::Duration::milliseconds(5)).await.unwrap();
        assert_eq!(pruned, 1); // 'old' should be pruned
        assert_eq!(mem.len(), 1);
    }

    #[tokio::test]
    async fn test_export_import() {
        let mem1 = MemoryManager::new();
        mem1.store("test", "value", HashMap::new()).await.unwrap();

        let json = mem1.export().await.unwrap();
        
        let mem2 = MemoryManager::new();
        mem2.import(&json).await.unwrap();
        
        let v = mem2.retrieve("test").await.unwrap();
        assert_eq!(v, Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_metadata_search() {
        let mem = MemoryManager::new();
        
        let mut meta = HashMap::new();
        meta.insert("type".to_string(), "config".to_string());
        mem.store("file1", "content1", meta).await.unwrap();

        let mut meta = HashMap::new();
        meta.insert("type".to_string(), "code".to_string());
        mem.store("file2", "content2", meta).await.unwrap();

        let results = mem.find_by_metadata("type", "config").await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "file1");
    }

    #[tokio::test]
    async fn test_batch_store() {
        let mem = MemoryManager::new();
        
        let items = vec![
            ("a".to_string(), "1".to_string(), HashMap::new()),
            ("b".to_string(), "2".to_string(), HashMap::new()),
            ("c".to_string(), "3".to_string(), HashMap::new()),
        ];

        mem.store_batch(items).await.unwrap();
        assert_eq!(mem.len(), 3);
    }
}

/// Memory persistence layer for disk storage
pub struct PersistentMemory {
    memory: Arc<MemoryManager>,
    storage_path: PathBuf,
    auto_save: bool,
}

impl PersistentMemory {
    pub fn new(memory: Arc<MemoryManager>, storage_path: PathBuf) -> Self {
        Self {
            memory,
            storage_path,
            auto_save: true,
        }
    }

    /// Save memories to disk
    pub async fn save(&self) -> Result<()> {
        let json = self.memory.export().await?;
        tokio::fs::write(&self.storage_path, json).await?;
        Ok(())
    }

    /// Load memories from disk
    pub async fn load(&self) -> Result<usize> {
        if !self.storage_path.exists() {
            return Ok(0);
        }

        let json = tokio::fs::read_to_string(&self.storage_path).await?;
        self.memory.import(&json).await
    }

    /// Enable/disable auto-save
    pub fn set_auto_save(&mut self, enabled: bool) {
        self.auto_save = enabled;
    }

    /// Auto-save on modifications
    async fn maybe_save(&self) -> Result<()> {
        if self.auto_save {
            self.save().await?;
        }
        Ok(())
    }

    pub async fn store(&self, key: &str, value: &str, metadata: HashMap<String, String>) -> Result<()> {
        self.memory.store(key, value, metadata).await?;
        self.maybe_save().await?;
        Ok(())
    }

    pub async fn store_batch(&self, items: Vec<(String, String, HashMap<String, String>)>) -> Result<usize> {
        let count = self.memory.store_batch(items).await?;
        self.maybe_save().await?;
        Ok(count)
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        self.memory.delete(key).await?;
        self.maybe_save().await?;
        Ok(())
    }
}

/// Memory query builder for complex searches
pub struct MemoryQuery {
    prefix: Option<String>,
    min_access_count: Option<usize>,
    older_than: Option<DateTime<Utc>>,
    newer_than: Option<DateTime<Utc>>,
    metadata_filters: HashMap<String, String>,
    limit: usize,
}

impl MemoryQuery {
    pub fn new() -> Self {
        Self {
            prefix: None,
            min_access_count: None,
            older_than: None,
            newer_than: None,
            metadata_filters: HashMap::new(),
            limit: 100,
        }
    }

    pub fn with_prefix(mut self, prefix: &str) -> Self {
        self.prefix = Some(prefix.to_string());
        self
    }

    pub fn min_access(mut self, count: usize) -> Self {
        self.min_access_count = Some(count);
        self
    }

    pub fn older_than(mut self, time: DateTime<Utc>) -> Self {
        self.older_than = Some(time);
        self
    }

    pub fn newer_than(mut self, time: DateTime<Utc>) -> Self {
        self.newer_than = Some(time);
        self
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata_filters.insert(key.to_string(), value.to_string());
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Execute query against memory manager
    pub async fn execute(&self, manager: &MemoryManager) -> Vec<MemoryItem> {
        let storage = manager.storage.read();
        let mut results = Vec::new();

        for item in storage.values() {
            // Apply prefix filter
            if let Some(prefix) = &self.prefix {
                if !item.key.starts_with(prefix) {
                    continue;
                }
            }

            // Apply access count filter
            if let Some(min) = self.min_access_count {
                if item.access_count < min {
                    continue;
                }
            }

            // Apply time filters
            if let Some(older) = self.older_than {
                if item.created_at > older {
                    continue;
                }
            }
            if let Some(newer) = self.newer_than {
                if item.created_at < newer {
                    continue;
                }
            }

            // Apply metadata filters
            let mut matches_metadata = true;
            for (k, v) in &self.metadata_filters {
                if let Some(val) = item.metadata.get(k) {
                    if val != v {
                        matches_metadata = false;
                        break;
                    }
                } else {
                    matches_metadata = false;
                    break;
                }
            }
            if !matches_metadata {
                continue;
            }

            results.push(item.clone());
            if results.len() >= self.limit {
                break;
            }
        }

        results
    }
}

impl Default for MemoryQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory watcher for monitoring changes
pub struct MemoryWatcher {
    manager: Arc<MemoryManager>,
    callbacks: Vec<Box<dyn Fn(MemoryEvent) + Send + Sync>>,
}

#[derive(Debug, Clone)]
pub enum MemoryEvent {
    ItemAdded(String),
    ItemUpdated(String),
    ItemAccessed(String),
    ItemRemoved(String),
    Cleared,
}

impl MemoryWatcher {
    pub fn new(manager: Arc<MemoryManager>) -> Self {
        Self {
            manager,
            callbacks: Vec::new(),
        }
    }

    pub fn on_event<F>(&mut self, callback: F)
    where
        F: Fn(MemoryEvent) + Send + Sync + 'static,
    {
        self.callbacks.push(Box::new(callback));
    }

    fn notify(&self, event: MemoryEvent) {
        for callback in &self.callbacks {
            callback(event.clone());
        }
    }

    // These would need to be integrated with MemoryManager methods
    // This is a simplified version
}

/// Memory backup manager
pub struct MemoryBackup {
    manager: Arc<MemoryManager>,
    backup_dir: PathBuf,
    max_backups: usize,
}

impl MemoryBackup {
    pub fn new(manager: Arc<MemoryManager>, backup_dir: PathBuf, max_backups: usize) -> Self {
        Self {
            manager,
            backup_dir,
            max_backups,
        }
    }

    /// Create a backup
    pub async fn create_backup(&self) -> Result<PathBuf> {
        tokio::fs::create_dir_all(&self.backup_dir).await?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = self.backup_dir.join(format!("memory_{}.json", timestamp));

        let json = self.manager.export().await?;
        tokio::fs::write(&backup_path, json).await?;

        // Clean up old backups
        self.cleanup_old_backups().await?;

        Ok(backup_path)
    }

    /// Restore from backup
    pub async fn restore(&self, backup_path: &Path) -> Result<usize> {
        if !backup_path.exists() {
            return Err(anyhow!("Backup file not found"));
        }

        let json = tokio::fs::read_to_string(backup_path).await?;
        self.manager.import(&json).await
    }

    /// List available backups
    pub async fn list_backups(&self) -> Result<Vec<PathBuf>> {
        let mut backups: Vec<PathBuf> = Vec::new();

        if !self.backup_dir.exists() {
            return Ok(backups);
        }

        let mut read_dir = tokio::fs::read_dir(&self.backup_dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
                backups.push(path);
            }
        }

        backups.sort_by(|a: &PathBuf, b: &PathBuf| b.cmp(a)); // Newest first
        Ok(backups)
    }

    /// Clean up old backups
    async fn cleanup_old_backups(&self) -> Result<()> {
        let mut backups = self.list_backups().await?;
        if backups.len() <= self.max_backups {
            return Ok(());
        }

        for old_backup in backups.drain(self.max_backups..) {
            let _ = tokio::fs::remove_file(old_backup).await;
        }

        Ok(())
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_persistent_memory() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("memory.json");

        let mem = Arc::new(MemoryManager::new());
        let persistent = PersistentMemory::new(mem.clone(), path.clone());

        persistent.store("test", "value", HashMap::new()).await.unwrap();
        assert!(path.exists());

        let mem2 = Arc::new(MemoryManager::new());
        let persistent2 = PersistentMemory::new(mem2.clone(), path);
        persistent2.load().await.unwrap();

        let v = mem2.retrieve("test").await.unwrap();
        assert_eq!(v, Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_memory_query() {
        let mem = MemoryManager::new();

        let mut meta = HashMap::new();
        meta.insert("type".to_string(), "config".to_string());
        mem.store("user:1:config", "value1", meta).await.unwrap();

        let mut meta = HashMap::new();
        meta.insert("type".to_string(), "data".to_string());
        mem.store("user:1:data", "value2", meta).await.unwrap();

        let query = MemoryQuery::new()
            .with_prefix("user:1:")
            .with_metadata("type", "config")
            .limit(10);

        let results = query.execute(&mem).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "user:1:config");
    }

    #[tokio::test]
    async fn test_memory_backup() {
        let dir = tempdir().unwrap();
        let mem = Arc::new(MemoryManager::new());

        mem.store("test", "value", HashMap::new()).await.unwrap();

        let backup = MemoryBackup::new(mem.clone(), dir.path().to_path_buf(), 3);
        let backup_path = backup.create_backup().await.unwrap();
        assert!(backup_path.exists());

        mem.delete("test").await.unwrap();
        assert!(mem.retrieve("test").await.unwrap().is_none());

        backup.restore(&backup_path).await.unwrap();
        assert!(mem.retrieve("test").await.unwrap().is_some());
    }
}

/// Re-export all memory types
pub mod prelude {
    pub use super::{
        MemoryManager,
        MemoryItem,
        MemoryStats,
        MemoryTTL,
        MemoryNamespace,
        PersistentMemory,
        MemoryQuery,
        MemoryEvent,
        MemoryWatcher,
        MemoryBackup,
    };
}

/// Version constant
pub const VERSION: &str = env!("CARGO_PKG_VERSION");