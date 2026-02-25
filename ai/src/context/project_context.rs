//! Project-wide context for AI

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use tokio::fs;

/// Project context
pub struct ProjectContext {
    root: Arc<RwLock<Option<PathBuf>>>,
    files: Arc<RwLock<HashMap<PathBuf, FileInfo>>>,
    dependencies: Arc<RwLock<Vec<DependencyInfo>>>,
    config_files: Arc<RwLock<HashMap<String, String>>>,
    git_info: Arc<RwLock<Option<GitInfo>>>,
    last_updated: Arc<RwLock<DateTime<Utc>>>,
    enabled: bool,
}

/// File information
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub lines: usize,
    pub language: Option<String>,
    pub last_modified: DateTime<Utc>,
    pub is_binary: bool,
    pub is_ignored: bool,
}

/// Dependency information
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    pub name: String,
    pub version: String,
    pub source: String,
}

/// Git information
#[derive(Debug, Clone)]
pub struct GitInfo {
    pub branch: String,
    pub commit: String,
    pub is_dirty: bool,
    pub remote: Option<String>,
}

/// Project snapshot
#[derive(Debug, Clone)]
pub struct ProjectSnapshot {
    pub root: Option<PathBuf>,
    pub file_count: usize,
    pub total_lines: usize,
    pub languages: HashMap<String, usize>,
    pub dependencies: Vec<DependencyInfo>,
    pub git: Option<GitInfo>,
    pub last_updated: DateTime<Utc>,
}

impl ProjectContext {
    pub fn new(enabled: bool) -> Self {
        Self {
            root: Arc::new(RwLock::new(None)),
            files: Arc::new(RwLock::new(HashMap::new())),
            dependencies: Arc::new(RwLock::new(Vec::new())),
            config_files: Arc::new(RwLock::new(HashMap::new())),
            git_info: Arc::new(RwLock::new(None)),
            last_updated: Arc::new(RwLock::new(Utc::now())),
            enabled,
        }
    }

    /// Set project root
    pub async fn set_root(&self, path: PathBuf) -> Result<()> {
        *self.root.write() = Some(path);
        self.refresh().await
    }

    /// Refresh project context
    pub async fn refresh(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let root = match self.root.read().as_ref() {
            Some(r) => r.clone(),
            None => return Ok(()),
        };

        let mut files = HashMap::new();
        self.scan_directory(&root, &mut files).await?;

        let deps = self.parse_dependencies(&root).await?;
        let git = self.get_git_info(&root).await?;

        *self.files.write() = files;
        *self.dependencies.write() = deps;
        *self.git_info.write() = git;
        *self.last_updated.write() = Utc::now();

        Ok(())
    }

    /// Recursively scan directory for files
    async fn scan_directory(&self, start_dir: &Path, files: &mut HashMap<PathBuf, FileInfo>) -> Result<()> {
        let mut stack = vec![start_dir.to_path_buf()];
        
        while let Some(dir) = stack.pop() {
            let mut read_dir = fs::read_dir(&dir).await?;
            
            loop {
                match read_dir.next_entry().await {
                    Ok(Some(entry)) => {
                        let path = entry.path();
                        
                        if path.is_file() {
                            if let Ok(metadata) = entry.metadata().await {
                                let size = metadata.len();
                                let lines = self.count_lines(&path).await.unwrap_or(0);
                                let language = self.detect_language(&path);
                                
                                files.insert(path.clone(), FileInfo {
                                    path: path.clone(),
                                    size,
                                    lines,
                                    language,
                                    last_modified: Utc::now(),
                                    is_binary: self.is_binary(&path).await.unwrap_or(false),
                                    is_ignored: false,
                                });
                            }
                        } else if path.is_dir() {
                            // Skip hidden directories and common non-essential dirs
                            let name = path.file_name().unwrap_or_default().to_string_lossy();
                            if !name.starts_with('.') && name != "node_modules" && name != "target" {
                                stack.push(path);
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
        }
        Ok(())
    }

    /// Get project snapshot
    pub async fn get_snapshot(&self) -> ProjectSnapshot {
        let files = self.files.read();
        
        ProjectSnapshot {
            root: self.root.read().clone(),
            file_count: files.len(),
            total_lines: files.values().map(|f| f.lines).sum(),
            languages: self.get_language_counts(&files),
            dependencies: self.dependencies.read().clone(),
            git: self.git_info.read().clone(),
            last_updated: *self.last_updated.read(),
        }
    }

    /// Count lines in file
    async fn count_lines(&self, path: &Path) -> Result<usize> {
        let content = fs::read_to_string(path).await?;
        Ok(content.lines().count())
    }

    /// Detect language
    fn detect_language(&self, path: &Path) -> Option<String> {
        let ext = path.extension()?.to_str()?;
        match ext {
            "rs" => Some("Rust".to_string()),
            "py" => Some("Python".to_string()),
            "js" => Some("JavaScript".to_string()),
            "ts" => Some("TypeScript".to_string()),
            "html" => Some("HTML".to_string()),
            "css" => Some("CSS".to_string()),
            "json" => Some("JSON".to_string()),
            "md" => Some("Markdown".to_string()),
            _ => None,
        }
    }

    /// Check if binary
    async fn is_binary(&self, path: &Path) -> Result<bool> {
        let content = fs::read(path).await?;
        Ok(content.iter().any(|&b| b == 0))
    }

    /// Parse dependencies
    async fn parse_dependencies(&self, root: &Path) -> Result<Vec<DependencyInfo>> {
        let mut deps: Vec<DependencyInfo> = Vec::new();

        // Cargo.toml - simplified parsing without toml crate
        let cargo_path = root.join("Cargo.toml");
        if cargo_path.exists() {
            if let Ok(content) = fs::read_to_string(cargo_path).await {
                // Simple regex-based parsing for dependencies section
                let mut in_dependencies = false;
                for line in content.lines() {
                    if line.trim() == "[dependencies]" {
                        in_dependencies = true;
                        continue;
                    }
                    if line.starts_with('[') {
                        in_dependencies = false;
                    }
                    if in_dependencies && line.contains('=') {
                        if let Some((name, version_part)) = line.split_once('=') {
                            let name = name.trim().to_string();
                            let version = version_part.trim().trim_matches('"').to_string();
                            deps.push(DependencyInfo {
                                name,
                                version,
                                source: "cargo".to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(deps)
    }

    /// Get git info
    async fn get_git_info(&self, root: &Path) -> Result<Option<GitInfo>> {
        let git_dir = root.join(".git");
        if !git_dir.exists() {
            return Ok(None);
        }

        let head_path = git_dir.join("HEAD");
        if !head_path.exists() {
            return Ok(None);
        }

        let head = fs::read_to_string(head_path).await?;
        let branch = if head.starts_with("ref: refs/heads/") {
            head[16..].trim().to_string()
        } else {
            "detached".to_string()
        };

        Ok(Some(GitInfo {
            branch,
            commit: "unknown".to_string(),
            is_dirty: false,
            remote: None,
        }))
    }

    /// Get language counts
    fn get_language_counts(&self, files: &HashMap<PathBuf, FileInfo>) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for file in files.values() {
            if let Some(lang) = &file.language {
                *counts.entry(lang.clone()).or_insert(0) += 1;
            }
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_project_context() {
        let ctx = ProjectContext::new(true);
        let snapshot = ctx.get_snapshot().await;
        assert_eq!(snapshot.file_count, 0);
    }
}