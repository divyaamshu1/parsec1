//! Git integration module for Parsec IDE
//!
//! Provides complete Git functionality including repository management,
//! status tracking, branching, committing, and remote operations.

mod repository;
mod status;
mod branch;
mod commit;
mod remote;
mod diff;
mod stash;

pub use repository::*;
pub use status::*;
pub use branch::*;
pub use commit::*;
pub use remote::*;
pub use diff::*;
pub use stash::*;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use anyhow::Result;
use tokio::sync::{RwLock, mpsc};
use chrono::{DateTime, Utc};

/// Main Git manager for handling repository operations
pub struct GitManager {
    /// Current repository
    repository: Option<Arc<RwLock<Repository>>>,
    /// Repository cache
    cache: Arc<RwLock<GitCache>>,
    /// File watcher for auto-refresh
    watcher: Option<notify::RecommendedWatcher>,
    /// Event sender for Git events
    event_tx: mpsc::UnboundedSender<GitEvent>,
    /// Configuration
    config: GitConfig,
}

/// Git configuration
#[derive(Debug, Clone)]
pub struct GitConfig {
    pub user_name: Option<String>,
    pub user_email: Option<String>,
    pub default_branch: String,
    pub auto_fetch: bool,
    pub auto_refresh: bool,
    pub sign_commits: bool,
    pub signing_key: Option<String>,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            user_name: None,
            user_email: None,
            default_branch: "main".to_string(),
            auto_fetch: true,
            auto_refresh: true,
            sign_commits: false,
            signing_key: None,
        }
    }
}

/// Git events for UI updates
#[derive(Debug, Clone)]
pub enum GitEvent {
    RepositoryOpened(PathBuf),
    RepositoryClosed,
    BranchChanged(String),
    StatusChanged(Vec<FileStatus>),
    CommitCreated(Commit),
    StashCreated(String),
    RemoteOperation(RemoteEvent),
    Error(String),
}

/// Remote operation events
#[derive(Debug, Clone)]
pub enum RemoteEvent {
    Pushed { remote: String, branch: String },
    Pulled { remote: String, branch: String, commits: usize },
    Fetched { remote: String, updates: usize },
}

/// Git cache for performance
#[derive(Debug, Default)]
struct GitCache {
    status: Option<Vec<FileStatus>>,
    branches: Option<Vec<Branch>>,
    commits: Vec<Commit>,
    last_refresh: Option<DateTime<Utc>>,
}

impl GitManager {
    /// Create a new Git manager
    pub fn new(config: GitConfig) -> Self {
        let (event_tx, _) = mpsc::unbounded_channel();
        
        Self {
            repository: None,
            cache: Arc::new(RwLock::new(GitCache::default())),
            watcher: None,
            event_tx,
            config,
        }
    }

    /// Open a repository at the given path
    pub async fn open_repository<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        let repo = Repository::open(path)?;
        
        self.repository = Some(Arc::new(RwLock::new(repo)));
        
        // Setup file watcher
        self.setup_watcher(path).await?;
        
        // Initial status refresh
        self.refresh_status().await?;
        
        self.event_tx.send(GitEvent::RepositoryOpened(path.to_path_buf())).ok();
        
        Ok(())
    }

    /// Initialize a new repository
    pub async fn init_repository<P: AsRef<Path>>(&mut self, path: P, bare: bool) -> Result<()> {
        let path = path.as_ref();
        let repo = Repository::init(path, bare)?;
        
        self.repository = Some(Arc::new(RwLock::new(repo)));
        
        self.event_tx.send(GitEvent::RepositoryOpened(path.to_path_buf())).ok();
        
        Ok(())
    }

    /// Close current repository
    pub async fn close_repository(&mut self) -> Result<()> {
        self.repository = None;
        self.cache.write().await.status = None;
        self.watcher = None;
        
        self.event_tx.send(GitEvent::RepositoryClosed).ok();
        
        Ok(())
    }

    /// Setup file watcher for auto-refresh
    async fn setup_watcher<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        use notify::{Watcher, RecursiveMode};
        
        let path = path.as_ref().to_path_buf();
        let cache = self.cache.clone();
        let event_tx = self.event_tx.clone();
        
        let watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            match res {
                Ok(event) => {
                    if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                        // Invalidate cache and trigger refresh
                        let cache = cache.clone();
                        let event_tx = event_tx.clone();
                        tokio::spawn(async move {
                            cache.write().await.status = None;
                            event_tx.send(GitEvent::StatusChanged(vec![])).ok();
                        });
                    }
                }
                Err(e) => log::error!("Git watch error: {}", e),
            }
        })?;
        
        let mut watcher = watcher;
        watcher.watch(&path, RecursiveMode::Recursive)?;
        self.watcher = Some(watcher);
        
        Ok(())
    }

    /// Get current repository
    pub fn repository(&self) -> Option<Arc<RwLock<Repository>>> {
        self.repository.clone()
    }

    /// Check if repository is open
    pub fn has_repository(&self) -> bool {
        self.repository.is_some()
    }

    /// Refresh repository status
    pub async fn refresh_status(&self) -> Result<Vec<FileStatus>> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Ok(vec![]),
        };
        
        let repo = repo.read().await;
        let status = repo.status()?;
        
        // Update cache
        let mut cache = self.cache.write().await;
        cache.status = Some(status.clone());
        cache.last_refresh = Some(Utc::now());
        
        self.event_tx.send(GitEvent::StatusChanged(status.clone())).ok();
        
        Ok(status)
    }

    /// Get cached status
    pub async fn get_status(&self) -> Option<Vec<FileStatus>> {
        self.cache.read().await.status.clone()
    }

    /// Stage files
    pub async fn stage_files(&self, paths: &[PathBuf]) -> Result<()> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Ok(()),
        };
        
        {
            let mut repo = repo.write().await;
            repo.stage(paths)?;
        }
        
        self.refresh_status().await?;
        
        Ok(())
    }

    /// Unstage files
    pub async fn unstage_files(&self, paths: &[PathBuf]) -> Result<()> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Ok(()),
        };
        
        {
            let mut repo = repo.write().await;
            repo.unstage(paths)?;
        }
        
        self.refresh_status().await?;
        
        Ok(())
    }

    /// Commit changes
    pub async fn commit(&self, message: &str, amend: bool) -> Result<Commit> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Err(anyhow::anyhow!("No repository open")),
        };
        
        let commit = {
            let mut repo = repo.write().await;
            repo.commit(message, amend, self.config.user_name.as_deref(), self.config.user_email.as_deref())?
        };
        
        self.refresh_status().await?;
        self.event_tx.send(GitEvent::CommitCreated(commit.clone())).ok();
        
        Ok(commit)
    }

    /// Create branch
    pub async fn create_branch(&self, name: &str, checkout: bool) -> Result<Branch> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Err(anyhow::anyhow!("No repository open")),
        };
        
        let branch = {
            let mut repo = repo.write().await;
            repo.create_branch(name, checkout)?
        };
        
        if checkout {
            self.event_tx.send(GitEvent::BranchChanged(name.to_string())).ok();
        }
        
        self.refresh_status().await?;
        
        Ok(branch)
    }

    /// Checkout branch
    pub async fn checkout_branch(&self, name: &str) -> Result<()> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Err(anyhow::anyhow!("No repository open")),
        };
        
        {
            let mut repo = repo.write().await;
            repo.checkout_branch(name)?;
        }
        
        self.event_tx.send(GitEvent::BranchChanged(name.to_string())).ok();
        self.refresh_status().await?;
        
        Ok(())
    }

    /// Delete branch
    pub async fn delete_branch(&self, name: &str, force: bool) -> Result<()> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Err(anyhow::anyhow!("No repository open")),
        };
        
        let mut repo = repo.write().await;
        repo.delete_branch(name, force)
    }

    /// Get branches
    pub async fn get_branches(&self) -> Result<Vec<Branch>> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Ok(vec![]),
        };
        
        let repo = repo.read().await;
        repo.branches()
    }

    /// Get current branch
    pub async fn current_branch(&self) -> Option<String> {
        let repo = match &self.repository {
            Some(r) => r.read().await,
            None => return None,
        };
        
        repo.current_branch()
    }

    /// Get commit history
    pub async fn get_commits(&self, limit: usize) -> Result<Vec<Commit>> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Ok(vec![]),
        };
        
        let repo = repo.read().await;
        repo.commits(limit)
    }

    /// Get commit details
    pub async fn get_commit(&self, id: &str) -> Result<Commit> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Err(anyhow::anyhow!("No repository open")),
        };
        
        let repo = repo.read().await;
        repo.get_commit(id)
    }

    /// Push to remote
    pub async fn push(&self, remote: &str, branch: &str, force: bool) -> Result<()> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Err(anyhow::anyhow!("No repository open")),
        };
        
        {
            let repo = repo.read().await;
            repo.push(remote, branch, force)?;
        }
        
        self.event_tx.send(GitEvent::RemoteOperation(RemoteEvent::Pushed {
            remote: remote.to_string(),
            branch: branch.to_string(),
        })).ok();
        
        Ok(())
    }

    /// Pull from remote
    pub async fn pull(&self, remote: &str, branch: &str, rebase: bool) -> Result<()> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Err(anyhow::anyhow!("No repository open")),
        };
        
        {
            let mut repo = repo.write().await;
            repo.pull(remote, branch, rebase)?;
        }
        
        self.refresh_status().await?;
        
        Ok(())
    }

    /// Fetch from remote
    pub async fn fetch(&self, remote: &str, prune: bool) -> Result<()> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Err(anyhow::anyhow!("No repository open")),
        };
        
        {
            let repo = repo.read().await;
            repo.fetch(remote, prune)?;
        }
        
        Ok(())
    }

    /// Get remotes
    pub async fn get_remotes(&self) -> Result<Vec<Remote>> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Ok(vec![]),
        };
        
        let repo = repo.read().await;
        repo.remotes()
    }

    /// Add remote
    pub async fn add_remote(&self, name: &str, url: &str) -> Result<()> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Err(anyhow::anyhow!("No repository open")),
        };
        
        let repo = repo.read().await;
        repo.add_remote(name, url)
    }

    /// Remove remote
    pub async fn remove_remote(&self, name: &str) -> Result<()> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Err(anyhow::anyhow!("No repository open")),
        };
        
        let repo = repo.read().await;
        repo.remove_remote(name)
    }

    /// Create stash
    pub async fn stash_create(&self, message: Option<&str>) -> Result<Stash> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Err(anyhow::anyhow!("No repository open")),
        };
        
        let stash = {
            let mut repo = repo.write().await;
            repo.stash_create(message)?
        };
        
        self.refresh_status().await?;
        self.event_tx.send(GitEvent::StashCreated(stash.message.clone())).ok();
        
        Ok(stash)
    }

    /// Apply stash
    pub async fn stash_apply(&self, index: usize) -> Result<()> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Err(anyhow::anyhow!("No repository open")),
        };
        
        {
            let mut repo = repo.write().await;
            repo.stash_apply(index)?;
        }
        
        self.refresh_status().await?;
        
        Ok(())
    }

    /// Drop stash
    pub async fn stash_drop(&self, index: usize) -> Result<()> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Err(anyhow::anyhow!("No repository open")),
        };
        
        let mut repo = repo.write().await;
        repo.stash_drop(index)
    }

    /// Get stashes
    pub async fn get_stashes(&self) -> Result<Vec<Stash>> {
        let repo = match &self.repository {
            Some(r) => r.clone(),
            None => return Ok(vec![]),
        };
        
        let mut repo = repo.write().await;
        repo.stashes()
    }

    /// Get Git statistics
    pub async fn statistics(&self) -> GitStatistics {
        let mut repo = match &self.repository {
            Some(r) => r.write().await,
            None => return GitStatistics::default(),
        };
        
        GitStatistics {
            has_repository: true,
            current_branch: repo.current_branch(),
            commits_ahead: repo.commits_ahead(),
            commits_behind: repo.commits_behind(),
            unstaged_changes: self.cache.read().await.status.as_ref().map(|s| s.len()).unwrap_or(0),
            stashes: repo.stashes().map(|s| s.len()).unwrap_or(0),
            remotes: repo.remotes().map(|r| r.len()).unwrap_or(0),
            branches: repo.branches().map(|b| b.len()).unwrap_or(0),
        }
    }

    /// Get next event
    pub async fn next_event(&mut self) -> Option<GitEvent> {
        // This would need an actual receiver
        None
    }
}

/// Git statistics
#[derive(Debug, Clone, Default)]
pub struct GitStatistics {
    pub has_repository: bool,
    pub current_branch: Option<String>,
    pub commits_ahead: usize,
    pub commits_behind: usize,
    pub unstaged_changes: usize,
    pub stashes: usize,
    pub remotes: usize,
    pub branches: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_git_manager() -> Result<()> {
        let dir = tempdir()?;
        let mut manager = GitManager::new(GitConfig::default());
        
        manager.init_repository(dir.path(), false).await?;
        assert!(manager.has_repository());
        
        Ok(())
    }
}