//! Code snippets library

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::fs;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use regex::Regex;

use crate::{Result, LearningError, ContentId};

/// Snippet language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetLanguage {
    pub name: String,
    pub extensions: Vec<String>,
    pub aliases: Vec<String>,
}

/// Snippet tag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetTag {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
}

/// Snippet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub code: String,
    pub language: String,
    pub tags: Vec<String>,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub source: Option<String>,
    pub stars: i32,
    pub forks: i32,
    pub usage_count: i32,
    pub dependencies: Vec<String>,
    pub examples: Vec<SnippetExample>,
    pub notes: String,
}

/// Snippet example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetExample {
    pub title: String,
    pub description: String,
    pub code: String,
    pub expected_output: Option<String>,
}

/// Snippet category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnippetCategory {
    Common,
    Algorithm,
    DataStructure,
    DesignPattern,
    Utility,
    Framework,
    Configuration,
    Boilerplate,
}

/// Snippet manager
pub struct SnippetManager {
    snippets: Arc<RwLock<HashMap<String, Snippet>>>,
    user_snippets: Arc<RwLock<HashMap<String, Vec<String>>>>,
    favorite_snippets: Arc<RwLock<HashMap<String, Vec<String>>>>,
    languages: Arc<RwLock<HashMap<String, SnippetLanguage>>>,
    tags: Arc<RwLock<HashMap<String, SnippetTag>>>,
    snippets_dir: PathBuf,
    fuzzy_matcher: SkimMatcherV2,
}

impl SnippetManager {
    /// Create new snippet manager
    pub fn new(snippets_dir: PathBuf) -> Self {
        let dir_clone = snippets_dir.clone();
        tokio::spawn(async move {
            let _ = fs::create_dir_all(&dir_clone).await;
        });

        Self {
            snippets: Arc::new(RwLock::new(HashMap::new())),
            user_snippets: Arc::new(RwLock::new(HashMap::new())),
            favorite_snippets: Arc::new(RwLock::new(HashMap::new())),
            languages: Arc::new(RwLock::new(HashMap::new())),
            tags: Arc::new(RwLock::new(HashMap::new())),
            snippets_dir,
            fuzzy_matcher: SkimMatcherV2::default(),
        }
    }

    /// Add snippet
    pub async fn add_snippet(&self, snippet: Snippet, owner: &str) -> Result<String> {
        let mut snippets = self.snippets.write().await;
        snippets.insert(snippet.id.clone(), snippet.clone());

        let mut user_snippets = self.user_snippets.write().await;
        user_snippets.entry(owner.to_string()).or_insert_with(Vec::new).push(snippet.id.clone());

        // Save to disk
        let path = self.snippets_dir.join(format!("{}.json", snippet.id));
        let json = serde_json::to_string_pretty(&snippet)?;
        fs::write(path, json).await?;

        Ok(snippet.id)
    }

    /// Get snippet by ID
    pub async fn get_snippet(&self, id: &str) -> Option<Snippet> {
        self.snippets.read().await.get(id).cloned()
    }

    /// Update snippet
    pub async fn update_snippet(&self, id: &str, mut snippet: Snippet) -> Result<()> {
        snippet.updated_at = Utc::now();
        
        let mut snippets = self.snippets.write().await;
        if snippets.contains_key(id) {
            snippets.insert(id.to_string(), snippet.clone());
            
            // Save to disk
            let path = self.snippets_dir.join(format!("{}.json", id));
            let json = serde_json::to_string_pretty(&snippet)?;
            fs::write(path, json).await?;
        }

        Ok(())
    }

    /// Delete snippet
    pub async fn delete_snippet(&self, id: &str, owner: &str) -> Result<()> {
        self.snippets.write().await.remove(id);
        
        if let Some(user_snippets) = self.user_snippets.write().await.get_mut(owner) {
            user_snippets.retain(|s| s != id);
        }

        // Remove from disk
        let path = self.snippets_dir.join(format!("{}.json", id));
        if path.exists() {
            fs::remove_file(path).await?;
        }

        Ok(())
    }

    /// Favorite snippet
    pub async fn favorite_snippet(&self, id: &str, user: &str) -> Result<()> {
        let mut favorites = self.favorite_snippets.write().await;
        favorites.entry(user.to_string()).or_insert_with(Vec::new).push(id.to_string());

        if let Some(snippet) = self.snippets.write().await.get_mut(id) {
            snippet.stars += 1;
        }

        Ok(())
    }

    /// Unfavorite snippet
    pub async fn unfavorite_snippet(&self, id: &str, user: &str) -> Result<()> {
        if let Some(favorites) = self.favorite_snippets.write().await.get_mut(user) {
            favorites.retain(|s| s != id);
        }

        if let Some(snippet) = self.snippets.write().await.get_mut(id) {
            snippet.stars -= 1;
        }

        Ok(())
    }

    /// Get user's snippets
    pub async fn get_user_snippets(&self, user: &str) -> Vec<Snippet> {
        let snippets = self.snippets.read().await;
        let user_snippets = self.user_snippets.read().await;

        if let Some(ids) = user_snippets.get(user) {
            ids.iter()
                .filter_map(|id| snippets.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get user's favorite snippets
    pub async fn get_favorite_snippets(&self, user: &str) -> Vec<Snippet> {
        let snippets = self.snippets.read().await;
        let favorites = self.favorite_snippets.read().await;

        if let Some(ids) = favorites.get(user) {
            ids.iter()
                .filter_map(|id| snippets.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Search snippets
    pub async fn search(&self, query: &str) -> Result<Vec<Snippet>> {
        let query_lower = query.to_lowercase();
        let snippets = self.snippets.read().await;
        let mut results = Vec::new();

        // Try fuzzy matching first
        for snippet in snippets.values() {
            if let Some(score) = self.fuzzy_matcher.fuzzy_match(&snippet.title, &query_lower) {
                if score > 80 {
                    results.push((score, snippet.clone()));
                }
            } else if snippet.title.to_lowercase().contains(&query_lower) ||
                      snippet.description.as_ref().map_or(false, |d| d.to_lowercase().contains(&query_lower)) ||
                      snippet.tags.iter().any(|t| t.to_lowercase().contains(&query_lower)) ||
                      snippet.language.to_lowercase().contains(&query_lower) {
                results.push((100, snippet.clone()));
            }
        }

        // Sort by relevance
        results.sort_by(|a, b| b.0.cmp(&a.0));
        Ok(results.into_iter().map(|(_, s)| s).collect())
    }

    /// Get snippets by language
    pub async fn get_by_language(&self, language: &str) -> Vec<Snippet> {
        self.snippets.read().await
            .values()
            .filter(|s| s.language.to_lowercase() == language.to_lowercase())
            .cloned()
            .collect()
    }

    /// Get snippets by tag
    pub async fn get_by_tag(&self, tag: &str) -> Vec<Snippet> {
        let tag_lower = tag.to_lowercase();
        self.snippets.read().await
            .values()
            .filter(|s| s.tags.iter().any(|t| t.to_lowercase() == tag_lower))
            .cloned()
            .collect()
    }

    /// Get popular snippets
    pub async fn get_popular(&self, limit: usize) -> Vec<Snippet> {
        let mut snippets: Vec<_> = self.snippets.read().await.values().cloned().collect();
        snippets.sort_by(|a, b| b.stars.cmp(&a.stars));
        snippets.truncate(limit);
        snippets
    }

    /// Get recently added snippets
    pub async fn get_recent(&self, limit: usize) -> Vec<Snippet> {
        let mut snippets: Vec<_> = self.snippets.read().await.values().cloned().collect();
        snippets.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        snippets.truncate(limit);
        snippets
    }

    /// Import snippet from file
    pub async fn import_from_file(&self, path: &Path, owner: &str) -> Result<String> {
        let content = fs::read_to_string(path).await?;
        
        // Try to detect language from extension
        let language = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("txt")
            .to_string();

        let snippet = Snippet {
            id: ContentId::new().0,
            title: path.file_stem().unwrap_or_default().to_string_lossy().to_string(),
            description: None,
            code: content,
            language,
            tags: vec![],
            author: owner.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source: Some(path.to_string_lossy().to_string()),
            stars: 0,
            forks: 0,
            usage_count: 0,
            dependencies: vec![],
            examples: vec![],
            notes: String::new(),
        };

        self.add_snippet(snippet, owner).await
    }

    /// Add language definition
    pub async fn add_language(&self, language: SnippetLanguage) {
        self.languages.write().await.insert(language.name.clone(), language);
    }

    /// Add tag definition
    pub async fn add_tag(&self, tag: SnippetTag) {
        self.tags.write().await.insert(tag.name.clone(), tag);
    }
}