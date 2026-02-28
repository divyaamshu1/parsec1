//! Cheat sheets for quick reference

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::fs;
use serde::{Serialize, Deserialize};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use markdown::to_html;

use crate::{Result, LearningError};

/// Cheat sheet category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatCategory {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub order: u32,
}

/// Cheat entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatEntry {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub code: String,
    pub language: String,
    pub category: String,
    pub tags: Vec<String>,
    pub examples: Vec<CheatExample>,
    pub see_also: Vec<String>,
    pub notes: String,
    pub version: String,
    pub added_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Cheat example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatExample {
    pub description: String,
    pub code: String,
    pub output: Option<String>,
}

/// Cheat sheet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatSheet {
    pub id: String,
    pub title: String,
    pub description: String,
    pub language: String,
    pub version: String,
    pub author: String,
    pub categories: Vec<CheatCategory>,
    pub entries: Vec<CheatEntry>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub popularity: u32,
}

/// Cheat sheet manager
pub struct CheatSheetManager {
    cheat_sheets: Arc<RwLock<HashMap<String, CheatSheet>>>,
    content_dir: PathBuf,
    fuzzy_matcher: SkimMatcherV2,
}

impl CheatSheetManager {
    /// Create new cheat sheet manager
    pub fn new(content_dir: PathBuf) -> Self {
        let dir_clone = content_dir.clone();
        tokio::spawn(async move {
            let _ = fs::create_dir_all(&dir_clone).await;
        });

        Self {
            cheat_sheets: Arc::new(RwLock::new(HashMap::new())),
            content_dir,
            fuzzy_matcher: SkimMatcherV2::default(),
        }
    }

    /// Load cheat sheet from file
    pub async fn load_cheat_sheet(&self, path: &Path) -> Result<String> {
        let content = fs::read_to_string(path).await?;
        let mut sheet: CheatSheet = if path.extension().and_then(|e| e.to_str()) == Some("yaml") ||
                                      path.extension().and_then(|e| e.to_str()) == Some("yml") {
            serde_yaml::from_str(&content).map_err(LearningError::Yaml)?
        } else {
            serde_json::from_str(&content).map_err(LearningError::Serialization)?
        };

        // Render markdown in descriptions
        for entry in &mut sheet.entries {
            if let Some(desc) = &entry.description {
                entry.description = Some(to_html(desc));
            }
        }

        sheet.id = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        self.cheat_sheets.write().await.insert(sheet.id.clone(), sheet.clone());

        Ok(sheet.id)
    }

    /// Load all cheat sheets from directory
    pub async fn load_all(&self) -> Result<Vec<String>> {
        let mut loaded = Vec::new();
        let mut read_dir = fs::read_dir(&self.content_dir).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Ok(id) = self.load_cheat_sheet(&path).await {
                    loaded.push(id);
                }
            }
        }

        Ok(loaded)
    }

    /// Get cheat sheet by ID
    pub async fn get_cheat_sheet(&self, id: &str) -> Option<CheatSheet> {
        self.cheat_sheets.read().await.get(id).cloned()
    }

    /// List all cheat sheets
    pub async fn list(&self) -> Result<Vec<CheatSheet>> {
        Ok(self.cheat_sheets.read().await.values().cloned().collect())
    }

    /// Get cheat sheets by language
    pub async fn get_by_language(&self, language: &str) -> Vec<CheatSheet> {
        self.cheat_sheets.read().await
            .values()
            .filter(|s| s.language.to_lowercase() == language.to_lowercase())
            .cloned()
            .collect()
    }

    /// Get entries by category
    pub async fn get_entries_by_category(&self, sheet_id: &str, category_id: &str) -> Option<Vec<CheatEntry>> {
        let sheet = self.cheat_sheets.read().await.get(sheet_id)?.clone();
        Some(sheet.entries.into_iter()
            .filter(|e| e.category == category_id)
            .collect())
    }

    /// Search cheat sheets
    pub async fn search(&self, query: &str) -> Result<Vec<CheatSheet>> {
        let query_lower = query.to_lowercase();
        let sheets = self.cheat_sheets.read().await;
        let mut results = Vec::new();

        for sheet in sheets.values() {
            if let Some(score) = self.fuzzy_matcher.fuzzy_match(&sheet.title, &query_lower) {
                if score > 70 {
                    results.push((score, sheet.clone()));
                }
            } else if sheet.title.to_lowercase().contains(&query_lower) ||
                      sheet.description.to_lowercase().contains(&query_lower) ||
                      sheet.tags.iter().any(|t| t.to_lowercase().contains(&query_lower)) {
                results.push((100, sheet.clone()));
            } else {
                // Search entries
                for entry in &sheet.entries {
                    if entry.title.to_lowercase().contains(&query_lower) ||
                       entry.description.as_ref().map_or(false, |d| d.to_lowercase().contains(&query_lower)) ||
                       entry.tags.iter().any(|t| t.to_lowercase().contains(&query_lower)) {
                        results.push((80, sheet.clone()));
                        break;
                    }
                }
            }
        }

        results.sort_by(|a, b| b.0.cmp(&a.0));
        Ok(results.into_iter().map(|(_, s)| s).collect())
    }

    /// Get entry by ID
    pub async fn get_entry(&self, sheet_id: &str, entry_id: &str) -> Option<CheatEntry> {
        let sheet = self.cheat_sheets.read().await.get(sheet_id)?.clone();
        sheet.entries.into_iter().find(|e| e.id == entry_id)
    }

    /// Render cheat sheet as HTML
    pub async fn render_html(&self, sheet: &CheatSheet) -> Result<String> {
        let mut html = String::new();
        
        html.push_str(&format!("<!DOCTYPE html>\n<html>\n<head>\n"));
        html.push_str(&format!("<title>{}</title>\n", sheet.title));
        html.push_str(&format!("<meta charset=\"utf-8\">\n"));
        html.push_str(&format!("<style>\n"));
        html.push_str(""); // stylesheet template unavailable
        html.push_str(&format!("</style>\n"));
        html.push_str(&format!("</head>\n<body>\n"));
        
        html.push_str(&format!("<h1>{}</h1>\n", sheet.title));
        html.push_str(&format!("<p class=\"description\">{}</p>\n", sheet.description));
        
        // TOC
        html.push_str(&format!("<div class=\"toc\">\n<h2>Contents</h2>\n<ul>\n"));
        for category in &sheet.categories {
            html.push_str(&format!("<li><a href=\"#{}\">{}</a></li>\n", category.id, category.name));
        }
        html.push_str(&format!("</ul>\n</div>\n"));
        
        // Categories and entries
        for category in &sheet.categories {
            html.push_str(&format!("<div class=\"category\" id=\"{}\">\n", category.id));
            html.push_str(&format!("<h2>{}</h2>\n", category.name));
            if let Some(desc) = &category.description {
                html.push_str(&format!("<p class=\"cat-desc\">{}</p>\n", desc));
            }
            
            html.push_str(&format!("<div class=\"entries\">\n"));
            
            let entries: Vec<_> = sheet.entries.iter()
                .filter(|e| e.category == category.id)
                .collect();
            
            for entry in entries {
                html.push_str(&format!("<div class=\"entry\" id=\"{}\">\n", entry.id));
                html.push_str(&format!("<h3>{}</h3>\n", entry.title));
                if let Some(desc) = &entry.description {
                    html.push_str(&format!("<p class=\"entry-desc\">{}</p>\n", desc));
                }
                
                html.push_str(&format!("<pre><code class=\"language-{}\">{}</code></pre>\n", 
                    entry.language, entry.code));
                
                if !entry.examples.is_empty() {
                    html.push_str(&format!("<div class=\"examples\">\n<h4>Examples</h4>\n"));
                    for example in &entry.examples {
                        html.push_str(&format!("<div class=\"example\">\n"));
                        html.push_str(&format!("<p>{}</p>\n", example.description));
                        html.push_str(&format!("<pre><code>{}</code></pre>\n", example.code));
                        if let Some(output) = &example.output {
                            html.push_str(&format!("<pre class=\"output\">{}</pre>\n", output));
                        }
                        html.push_str(&format!("</div>\n"));
                    }
                    html.push_str(&format!("</div>\n"));
                }
                
                if !entry.see_also.is_empty() {
                    html.push_str(&format!("<p class=\"see-also\">See also: {}</p>\n", entry.see_also.join(", ")));
                }
                
                html.push_str(&format!("</div>\n"));
            }
            
            html.push_str(&format!("</div>\n"));
            html.push_str(&format!("</div>\n"));
        }
        
        html.push_str(&format!("</body>\n</html>"));
        
        Ok(html)
    }

    /// Get popular cheat sheets
    pub async fn get_popular(&self, limit: usize) -> Vec<CheatSheet> {
        let mut sheets: Vec<_> = self.cheat_sheets.read().await.values().cloned().collect();
        sheets.sort_by(|a, b| b.popularity.cmp(&a.popularity));
        sheets.truncate(limit);
        sheets
    }
}