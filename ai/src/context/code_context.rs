//! Code context extraction from source files

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use regex::Regex;

/// Code context cache
pub struct CodeContextCache {
    cache: Arc<RwLock<HashMap<PathBuf, CachedCode>>>,
    max_size: usize,
    access_order: Arc<RwLock<VecDeque<PathBuf>>>,
}

struct CachedCode {
    context: CodeContext,
    last_accessed: DateTime<Utc>,
}

/// Code context for a file
#[derive(Debug, Clone)]
pub struct CodeContext {
    pub path: PathBuf,
    pub content: String,
    pub language: Option<String>,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<String>,
    pub tokens: usize,
    pub line_count: usize,
    pub last_modified: DateTime<Utc>,
}

/// Function information
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    pub doc: Option<String>,
    pub body: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// Class information
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub methods: Vec<FunctionInfo>,
    pub properties: Vec<String>,
    pub doc: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
}

impl CodeContextCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::with_capacity(max_size))),
            max_size,
            access_order: Arc::new(RwLock::new(VecDeque::with_capacity(max_size))),
        }
    }

    /// Add context to cache
    pub async fn add_context(&self, path: PathBuf, content: String, language: Option<String>) -> Result<()> {
        let tokens = content.len() / 4; // approximate token count
        
        let context = CodeContext {
            path: path.clone(),
            content: content.clone(),
            language,
            functions: self.extract_functions(&content)?,
            classes: self.extract_classes(&content)?,
            imports: self.extract_imports(&content)?,
            tokens,
            line_count: content.lines().count(),
            last_modified: Utc::now(),
        };

        let cached = CachedCode {
            context,
            last_accessed: Utc::now(),
        };

        let mut cache = self.cache.write();
        let mut order = self.access_order.write();

        if cache.len() >= self.max_size {
            if let Some(oldest) = order.pop_front() {
                cache.remove(&oldest);
            }
        }

        cache.insert(path.clone(), cached);
        order.push_back(path);

        Ok(())
    }

    /// Get context from cache
    pub async fn get_context(&self, path: &Path) -> Option<CodeContext> {
        let mut cache = self.cache.write();
        let mut order = self.access_order.write();

        if let Some(cached) = cache.get_mut(path) {
            cached.last_accessed = Utc::now();

            if let Some(pos) = order.iter().position(|p| p == path) {
                order.remove(pos);
                order.push_back(path.to_path_buf());
            }

            Some(cached.context.clone())
        } else {
            None
        }
    }

    /// Extract functions from code
    fn extract_functions(&self, content: &str) -> Result<Vec<FunctionInfo>> {
        let mut functions = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Simple regex-based extraction (would use tree-sitter in production)
        let fn_regex = Regex::new(r"(?m)^\s*(?:pub\s+)?fn\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*([^{]+))?\s*\{").unwrap();

        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = fn_regex.captures(line) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                let params = caps.get(2).map(|m| m.as_str().split(',').map(|s| s.trim().to_string()).collect()).unwrap_or_default();
                let return_type = caps.get(3).map(|m| m.as_str().trim().to_string());

                // Find function end (simplified)
                let mut brace_count = 1;
                let mut end_line = i;
                for (j, l) in lines[i+1..].iter().enumerate() {
                    for c in l.chars() {
                        if c == '{' { brace_count += 1; }
                        if c == '}' { brace_count -= 1; }
                    }
                    if brace_count == 0 {
                        end_line = i + 1 + j;
                        break;
                    }
                }

                functions.push(FunctionInfo {
                    name,
                    parameters: params,
                    return_type,
                    doc: None,
                    body: lines[i..=end_line].join("\n"),
                    start_line: i,
                    end_line,
                });
            }
        }

        Ok(functions)
    }

    /// Extract classes from code
    fn extract_classes(&self, content: &str) -> Result<Vec<ClassInfo>> {
        let mut classes = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        let class_regex = Regex::new(r"(?m)^\s*(?:pub\s+)?(?:struct|class|enum|trait)\s+(\w+)").unwrap();

        for (i, line) in lines.iter().enumerate() {
            if class_regex.is_match(line) {
                classes.push(ClassInfo {
                    name: "Class".to_string(),
                    methods: Vec::new(),
                    properties: Vec::new(),
                    doc: None,
                    start_line: i,
                    end_line: i + 10, // approximate
                });
            }
        }

        Ok(classes)
    }

    /// Extract imports from code
    fn extract_imports(&self, content: &str) -> Result<Vec<String>> {
        let mut imports = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("import ") || trimmed.starts_with("use ") || trimmed.starts_with("#include") {
                imports.push(trimmed.to_string());
            }
        }

        Ok(imports)
    }

    /// Clear cache
    pub async fn clear(&self) {
        self.cache.write().clear();
        self.access_order.write().clear();
    }

    /// Cache size
    pub fn size(&self) -> usize {
        self.cache.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_code_cache() {
        let cache = CodeContextCache::new(10);
        
        let path = PathBuf::from("test.rs");
        let content = "fn main() { println!(\"Hello\"); }".to_string();
        
        cache.add_context(path.clone(), content, Some("rust".to_string())).await.unwrap();
        
        let retrieved = cache.get_context(&path).await;
        assert!(retrieved.is_some());
    }
}