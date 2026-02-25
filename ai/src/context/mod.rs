//! AI context management for code understanding
//!
//! Provides context extraction and management for AI-powered features.

mod code_context;
mod project_context;
mod memory;

pub use code_context::*;
pub use project_context::*;
pub use memory::*;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

/// Context configuration
#[derive(Debug, Clone)]
pub struct ContextConfig {
    pub max_tokens: usize,
    pub max_files: usize,
    pub max_line_length: usize,
    pub include_comments: bool,
    pub include_imports: bool,
    pub include_functions: bool,
    pub include_classes: bool,
    pub include_dependencies: bool,
    pub cache_size: usize,
    pub cache_ttl_seconds: u64,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_tokens: 4000,
            max_files: 20,
            max_line_length: 200,
            include_comments: true,
            include_imports: true,
            include_functions: true,
            include_classes: true,
            include_dependencies: false,
            cache_size: 100,
            cache_ttl_seconds: 3600,
        }
    }
}

/// Context source type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextSource {
    CurrentFile,
    OpenFiles,
    Project,
    Workspace,
    Definition,
    Reference,
    Search,
    Related,
}

/// Context item
#[derive(Debug, Clone)]
pub struct ContextItem {
    pub source: ContextSource,
    pub path: PathBuf,
    pub content: String,
    pub language: Option<String>,
    pub relevance: f32,
    pub tokens: usize,
    pub line_start: usize,
    pub line_end: usize,
}

/// Context query
#[derive(Debug, Clone)]
pub struct ContextQuery {
    pub text: String,
    pub cursor_line: Option<usize>,
    pub cursor_col: Option<usize>,
    pub file_path: Option<PathBuf>,
    pub max_items: usize,
    pub min_relevance: f32,
    pub include_sources: Vec<ContextSource>,
}

/// Context manager
pub struct ContextManager {
    code_context: Arc<CodeContextCache>,
    project_context: Arc<ProjectContext>,
    memory_manager: Arc<MemoryManager>,
    config: ContextConfig,
    cache: Arc<RwLock<HashMap<String, CachedContext>>>,
}

struct CachedContext {
    items: Vec<ContextItem>,
    timestamp: DateTime<Utc>,
    query_hash: String,
}

impl ContextManager {
    pub async fn new(config: ContextConfig) -> Result<Self> {
        Ok(Self {
            code_context: Arc::new(CodeContextCache::new(config.cache_size)),
            project_context: Arc::new(ProjectContext::new(true)),
            memory_manager: Arc::new(MemoryManager::new()),
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get context for a query
    pub async fn get_context(&self, query: ContextQuery) -> Result<Vec<ContextItem>> {
        let query_hash = self.hash_query(&query);
        
        // Check cache
        if let Some(cached) = self.cache.read().await.get(&query_hash) {
            let elapsed = (chrono::Utc::now() - cached.timestamp).num_seconds();
            if elapsed < self.config.cache_ttl_seconds as i64 {
                return Ok(cached.items.clone());
            }
        }

        let mut items = Vec::new();

        // Get code context
        if let Some(path) = &query.file_path {
            if let Some(ctx) = self.code_context.get_context(path).await {
                items.push(ContextItem {
                    source: ContextSource::CurrentFile,
                    path: path.clone(),
                    content: ctx.content,
                    language: ctx.language,
                    relevance: 1.0,
                    tokens: ctx.tokens,
                    line_start: 0,
                    line_end: ctx.line_count,
                });
            }
        }

        // Get project context
        if query.include_sources.contains(&ContextSource::Project) {
            let snapshot = self.project_context.get_snapshot().await;
            // Add project context as items
        }

        // Filter by relevance
        items.retain(|i| i.relevance >= query.min_relevance);
        items.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());
        items.truncate(query.max_items);

        // Cache result
        let hash_copy = query_hash.clone();
        self.cache.write().await.insert(hash_copy, CachedContext {
            items: items.clone(),
            timestamp: Utc::now(),
            query_hash,
        });

        Ok(items)
    }

    /// Add code context
    pub async fn add_code_context(&self, path: PathBuf, content: String, language: Option<String>) -> Result<()> {
        self.code_context.add_context(path, content, language).await
    }

    /// Set project root
    pub async fn set_project_root(&self, path: PathBuf) -> Result<()> {
        self.project_context.set_root(path).await
    }

    /// Store in memory
    pub async fn remember(&self, key: &str, value: &str, metadata: HashMap<String, String>) -> Result<()> {
        self.memory_manager.store(key, value, metadata).await
    }

    /// Recall from memory
    pub async fn recall(&self, key: &str) -> Result<Option<String>> {
        self.memory_manager.retrieve(key).await
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }

    fn hash_query(&self, query: &ContextQuery) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.text.hash(&mut hasher);
        query.max_items.hash(&mut hasher);
        query.min_relevance.to_bits().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_manager() {
        let config = ContextConfig::default();
        let manager = ContextManager::new(config).await.unwrap();

        let query = ContextQuery {
            text: "test".to_string(),
            cursor_line: None,
            cursor_col: None,
            file_path: None,
            max_items: 10,
            min_relevance: 0.5,
            include_sources: vec![],
        };

        let items = manager.get_context(query).await.unwrap();
        assert!(items.is_empty());
    }
}