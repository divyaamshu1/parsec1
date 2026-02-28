//! Settings synchronization across devices

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::{RwLock, mpsc};
use tokio::fs;
use serde::{Serialize, Deserialize};
use tracing::{info, warn, debug};

#[cfg(feature = "settings-sync")]
use git2::{Repository, RemoteCallbacks, Cred, PushOptions};
#[cfg(feature = "settings-sync")]
use ssh2::Session;
#[cfg(feature = "cloud-sync")]
use aws_sdk_s3::Client as S3Client;
#[cfg(feature = "cloud-sync")]
use azure_storage_blob::prelude::*;

use crate::{Result, CustomizationError, CustomizationConfig, SyncProviderType};

/// Sync status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    Idle,
    Syncing,
    Success,
    Failed(String),
    Conflict,
}

/// Sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub provider: SyncProviderType,
    pub endpoint: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub path: Option<String>,
    pub encrypt: bool,
    pub compression: bool,
    pub auto_sync: bool,
    pub sync_interval: Option<u64>,
    pub conflict_resolution: ConflictResolution,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            provider: SyncProviderType::Local,
            endpoint: None,
            username: None,
            password: None,
            token: None,
            repository: None,
            branch: Some("main".to_string()),
            path: None,
            encrypt: false,
            compression: true,
            auto_sync: false,
            sync_interval: Some(3600),
            conflict_resolution: ConflictResolution::Ask,
        }
    }
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    Ask,
    LocalWins,
    RemoteWins,
    Merge,
    NewestWins,
    Manual,
}

/// Cloud sync provider
#[cfg(feature = "cloud-sync")]
pub struct CloudSync {
    #[cfg(feature = "aws-sdk-s3")]
    s3_client: Option<S3Client>,
    #[cfg(feature = "azure-storage-blob")]
    blob_client: Option<BlobClient>,
    bucket: Option<String>,
    container: Option<String>,
    path: String,
}

#[cfg(feature = "cloud-sync")]
impl CloudSync {
    /// Create new cloud sync
    pub async fn new(config: &SyncConfig) -> Result<Self> {
        let mut sync = Self {
            #[cfg(feature = "aws-sdk-s3")]
            s3_client: None,
            #[cfg(feature = "azure-storage-blob")]
            blob_client: None,
            bucket: None,
            container: None,
            path: config.path.clone().unwrap_or_else(|| "parsec-settings".to_string()),
        };

        // Initialize AWS S3
        #[cfg(feature = "aws-sdk-s3")]
        if let Some(endpoint) = &config.endpoint {
            let config = aws_config::from_env()
                .endpoint_url(endpoint)
                .load()
                .await;
            sync.s3_client = Some(S3Client::new(&config));
            sync.bucket = Some(config.path.clone().unwrap_or_else(|| "parsec-settings".to_string()));
        }

        // Initialize Azure Blob
        #[cfg(feature = "azure-storage-blob")]
        if let Some(endpoint) = &config.endpoint {
            let credentials = if let (Some(account), Some(key)) = (&config.username, &config.password) {
                StorageCredentials::Key(account.clone(), key.clone())
            } else {
                StorageCredentials::anonymous()
            };
            let client = BlobClient::new(&endpoint, credentials)?;
            sync.blob_client = Some(client);
            sync.container = Some(config.path.clone().unwrap_or_else(|| "parsec-settings".to_string()));
        }

        Ok(sync)
    }

    /// Upload to cloud
    pub async fn upload(&self, data: &[u8], key: &str) -> Result<()> {
        #[cfg(feature = "aws-sdk-s3")]
        if let (Some(client), Some(bucket)) = (&self.s3_client, &self.bucket) {
            client.put_object()
                .bucket(bucket)
                .key(&format!("{}/{}", self.path, key))
                .body(data.to_vec().into())
                .send()
                .await?;
            return Ok(());
        }

        #[cfg(feature = "azure-storage-blob")]
        if let (Some(client), Some(container)) = (&self.blob_client, &self.container) {
            client.put_block_blob()
                .container(container)
                .blob(&format!("{}/{}", self.path, key))
                .body(data.to_vec())
                .send()
                .await?;
            return Ok(());
        }

        Err(CustomizationError::SyncError("No cloud provider configured".to_string()))
    }

    /// Download from cloud
    pub async fn download(&self, key: &str) -> Result<Vec<u8>> {
        #[cfg(feature = "aws-sdk-s3")]
        if let (Some(client), Some(bucket)) = (&self.s3_client, &self.bucket) {
            let response = client.get_object()
                .bucket(bucket)
                .key(&format!("{}/{}", self.path, key))
                .send()
                .await?;
            let data = response.body.collect().await?.to_vec();
            return Ok(data);
        }

        #[cfg(feature = "azure-storage-blob")]
        if let (Some(client), Some(container)) = (&self.blob_client, &self.container) {
            let response = client.get_blob()
                .container(container)
                .blob(&format!("{}/{}", self.path, key))
                .send()
                .await?;
            let data = response.into_body().collect().await?.to_vec();
            return Ok(data);
        }

        Err(CustomizationError::SyncError("No cloud provider configured".to_string()))
    }

    /// List objects in cloud
    pub async fn list(&self) -> Result<Vec<String>> {
        let mut objects = Vec::new();

        #[cfg(feature = "aws-sdk-s3")]
        if let (Some(client), Some(bucket)) = (&self.s3_client, &self.bucket) {
            let response = client.list_objects_v2()
                .bucket(bucket)
                .prefix(&self.path)
                .send()
                .await?;
            
            if let Some(contents) = response.contents {
                for obj in contents {
                    if let Some(key) = obj.key {
                        if let Some(name) = key.strip_prefix(&format!("{}/", self.path)) {
                            objects.push(name.to_string());
                        }
                    }
                }
            }
        }

        #[cfg(feature = "azure-storage-blob")]
        if let (Some(client), Some(container)) = (&self.blob_client, &self.container) {
            let response = client.list_blobs()
                .container(container)
                .prefix(&format!("{}/", self.path))
                .send()
                .await?;
            
            for blob in response.blobs {
                if let Some(name) = blob.name.strip_prefix(&format!("{}/", self.path)) {
                    objects.push(name.to_string());
                }
            }
        }

        Ok(objects)
    }

    /// Delete from cloud
    pub async fn delete(&self, key: &str) -> Result<()> {
        #[cfg(feature = "aws-sdk-s3")]
        if let (Some(client), Some(bucket)) = (&self.s3_client, &self.bucket) {
            client.delete_object()
                .bucket(bucket)
                .key(&format!("{}/{}", self.path, key))
                .send()
                .await?;
            return Ok(());
        }

        #[cfg(feature = "azure-storage-blob")]
        if let (Some(client), Some(container)) = (&self.blob_client, &self.container) {
            client.delete_blob()
                .container(container)
                .blob(&format!("{}/{}", self.path, key))
                .send()
                .await?;
            return Ok(());
        }

        Err(CustomizationError::SyncError("No cloud provider configured".to_string()))
    }
}

/// Git sync provider
#[cfg(feature = "settings-sync")]
pub struct GitSync {
    repository: Option<Repository>,
    repo_path: PathBuf,
    remote_url: String,
    branch: String,
    credentials: Option<(String, String)>,
}

// libgit2 repository pointer is not automatically Sync; we mark the wrapper
// safe because we only use it from a single thread at a time.
#[cfg(feature = "settings-sync")]
unsafe impl Send for GitSync {}
#[cfg(feature = "settings-sync")]
unsafe impl Sync for GitSync {}

#[cfg(feature = "settings-sync")]
impl GitSync {
    /// Create new git sync
    pub fn new(config: &SyncConfig) -> Result<Self> {
        Ok(Self {
            repository: None,
            repo_path: config.path.as_ref().map(PathBuf::from).unwrap_or_else(|| {
                dirs::home_dir().unwrap().join(".parsec").join("settings-repo")
            }),
            remote_url: config.repository.clone().ok_or_else(|| {
                CustomizationError::SyncError("No repository configured".to_string())
            })?,
            branch: config.branch.clone().unwrap_or_else(|| "main".to_string()),
            credentials: config.username.clone().zip(config.password.clone()),
        })
    }

    /// Initialize git repository
    pub async fn init(&mut self) -> Result<()> {
        if self.repo_path.exists() {
            self.repository = Some(Repository::open(&self.repo_path)?);
        } else {
            fs::create_dir_all(&self.repo_path).await?;
            self.repository = Some(Repository::init(&self.repo_path)?);
        }

        // Configure remote
        if let Some(repo) = &self.repository {
            if repo.find_remote("origin").is_err() {
                repo.remote("origin", &self.remote_url)?;
            }
        }

        Ok(())
    }

    /// Clone repository
    pub async fn clone(&mut self) -> Result<()> {
        let mut callbacks = RemoteCallbacks::new();
        
        if let Some((username, password)) = &self.credentials {
            callbacks.credentials(|_url, username_from_url, _allowed_types| {
                Cred::userpass_plaintext(
                    username_from_url.unwrap_or(username),
                    password,
                )
            });
        }

        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        // use a RepoBuilder to apply authentication callbacks and options
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_opts);
        builder.bare(false);

        self.repository = Some(builder.clone(&self.remote_url, &self.repo_path)?);

        Ok(())
    }

    /// Pull latest changes
    pub async fn pull(&self) -> Result<()> {
        if let Some(repo) = &self.repository {
            let mut callbacks = RemoteCallbacks::new();
            
            if let Some((username, password)) = &self.credentials {
                callbacks.credentials(|_url, username_from_url, _allowed_types| {
                    Cred::userpass_plaintext(
                        username_from_url.unwrap_or(username),
                        password,
                    )
                });
            }

            let mut fetch_opts = git2::FetchOptions::new();
            fetch_opts.remote_callbacks(callbacks);

            let mut remote = repo.find_remote("origin")?;
            remote.fetch(&[&self.branch], Some(&mut fetch_opts), None)?;

            let fetch_head = repo.find_reference("FETCH_HEAD")?;
            let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;

            let analysis = repo.merge_analysis(&[&fetch_commit])?;

            if analysis.0.is_fast_forward() {
                // Fast-forward
                let refname = format!("refs/heads/{}", self.branch);
                let mut reference = repo.find_reference(&refname)?;
                reference.set_target(fetch_commit.id(), "Fast-forward")?;
                repo.set_head(&refname)?;
                repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
            } else if analysis.0.is_normal() {
                // Merge
                let merge_commit = repo.find_annotated_commit(fetch_commit.id())?;
                repo.merge(&[&merge_commit], None, Some(git2::build::CheckoutBuilder::default().force()))?;
            }
        }

        Ok(())
    }

    /// Push changes
    pub async fn push(&self, message: &str) -> Result<()> {
        if let Some(repo) = &self.repository {
            // Add all files
            let mut index = repo.index()?;
            index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
            index.write()?;

            // Create commit
            let tree_id = index.write_tree()?;
            let tree = repo.find_tree(tree_id)?;

            let head = repo.head()?;
            let parent = repo.find_commit(head.target().unwrap())?;

            let signature = git2::Signature::now("Parsec User", "user@parsec.local")?;
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &[&parent],
            )?;

            // Push
            let mut callbacks = RemoteCallbacks::new();
            
            if let Some((username, password)) = &self.credentials {
                callbacks.credentials(|_url, username_from_url, _allowed_types| {
                    Cred::userpass_plaintext(
                        username_from_url.unwrap_or(username),
                        password,
                    )
                });
            }

            let mut push_opts = git2::PushOptions::new();
            push_opts.remote_callbacks(callbacks);

            let mut remote = repo.find_remote("origin")?;
            remote.push(&[&format!("refs/heads/{}:refs/heads/{}", self.branch, self.branch)], Some(&mut push_opts))?;
        }

        Ok(())
    }

    /// Get current commit hash
    pub fn current_commit(&self) -> Result<String> {
        if let Some(repo) = &self.repository {
            let head = repo.head()?;
            Ok(head.target().unwrap().to_string())
        } else {
            Ok("none".to_string())
        }
    }
}

/// Local sync provider (file system)
pub struct LocalSync {
    base_path: PathBuf,
}

impl LocalSync {
    /// Create new local sync
    pub fn new(config: &SyncConfig) -> Result<Self> {
        let path = config.path.as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs::home_dir().unwrap().join(".parsec").join("settings"));
        
        Ok(Self { base_path: path })
    }

    /// Write file
    pub async fn write(&self, key: &str, data: &[u8]) -> Result<()> {
        let path = self.base_path.join(key);
        fs::create_dir_all(path.parent().unwrap()).await?;
        fs::write(path, data).await?;
        Ok(())
    }

    /// Read file
    pub async fn read(&self, key: &str) -> Result<Vec<u8>> {
        let path = self.base_path.join(key);
        Ok(fs::read(path).await?)
    }

    /// List files
    pub async fn list(&self) -> Result<Vec<String>> {
        let mut files = Vec::new();
        self.list_recursive(&self.base_path, &mut files)?;
        Ok(files)
    }

    /// List files recursively
    fn list_recursive(&self, dir: &Path, files: &mut Vec<String>) -> Result<()> {
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    self.list_recursive(&path, files)?;
                } else {
                    if let Ok(rel_path) = path.strip_prefix(&self.base_path) {
                        files.push(rel_path.to_string_lossy().to_string());
                    }
                }
            }
        }
        Ok(())
    }

    /// Delete file
    pub async fn delete(&self, key: &str) -> Result<()> {
        let path = self.base_path.join(key);
        if path.exists() {
            fs::remove_file(path).await?;
        }
        Ok(())
    }
}

/// Settings sync manager
pub struct SettingsSync {
    /// Current sync status
    status: Arc<RwLock<SyncStatus>>,
    /// Sync configuration
    config: Arc<RwLock<SyncConfig>>,
    /// Last sync time
    last_sync: Arc<RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
    /// Sync provider
    #[cfg(feature = "cloud-sync")]
    cloud_sync: Option<CloudSync>,
    #[cfg(feature = "settings-sync")]
    git_sync: Option<GitSync>,
    local_sync: Option<LocalSync>,
    /// Configuration
    engine_config: CustomizationConfig,
}

// SettingsSync may contain a GitSync which wraps a raw libgit2 pointer; mark
// it Send/Sync to allow spawning tasks that own the engine.
unsafe impl Send for SettingsSync {}
unsafe impl Sync for SettingsSync {}

impl SettingsSync {
    /// Create new settings sync
    pub async fn new(config: CustomizationConfig) -> Result<Self> {
        let sync_config = SyncConfig::default();
        
        #[cfg(feature = "cloud-sync")]
        let cloud_sync = CloudSync::new(&sync_config).await.ok();
        
        #[cfg(feature = "settings-sync")]
        let git_sync = GitSync::new(&sync_config).ok();
        
        let local_sync = LocalSync::new(&sync_config).ok();

        Ok(Self {
            status: Arc::new(RwLock::new(SyncStatus::Idle)),
            config: Arc::new(RwLock::new(sync_config)),
            last_sync: Arc::new(RwLock::new(None)),
            #[cfg(feature = "cloud-sync")]
            cloud_sync,
            #[cfg(feature = "settings-sync")]
            git_sync,
            local_sync,
            engine_config: config,
        })
    }

    /// Update sync configuration
    pub async fn update_config(&mut self, config: SyncConfig) -> Result<()> {
        *self.config.write().await = config.clone();

        // Reinitialize providers
        #[cfg(feature = "cloud-sync")]
        {
            self.cloud_sync = CloudSync::new(&config).await.ok();
        }

        #[cfg(feature = "settings-sync")]
        {
            self.git_sync = GitSync::new(&config).ok();
        }

        self.local_sync = LocalSync::new(&config).ok();

        Ok(())
    }

    /// Sync settings
    pub async fn sync(&self) -> Result<()> {
        *self.status.write().await = SyncStatus::Syncing;

        let result = match self.config.read().await.provider {
            SyncProviderType::Local => self.sync_local().await,
            SyncProviderType::Git => self.sync_git().await,
            SyncProviderType::Cloud => self.sync_cloud().await,
            SyncProviderType::Custom => self.sync_custom().await,
        };

        *self.last_sync.write().await = Some(chrono::Utc::now());

        match result {
            Ok(_) => {
                *self.status.write().await = SyncStatus::Success;
                Ok(())
            }
            Err(e) => {
                *self.status.write().await = SyncStatus::Failed(e.to_string());
                Err(e)
            }
        }
    }

    /// Sync with local filesystem
    async fn sync_local(&self) -> Result<()> {
        if let Some(local) = &self.local_sync {
            // Save current settings
            let settings = self.collect_settings().await?;
            local.write("settings.json", &settings).await?;

            // Save profiles
            let profiles = self.collect_profiles().await?;
            local.write("profiles.json", &profiles).await?;

            // Save keymaps
            let keymaps = self.collect_keymaps().await?;
            local.write("keymaps.json", &keymaps).await?;

            // Save themes
            let themes = self.collect_themes().await?;
            local.write("themes.json", &themes).await?;

            // Save layouts
            let layouts = self.collect_layouts().await?;
            local.write("layouts.json", &layouts).await?;
        }

        Ok(())
    }

    /// Sync with git
    #[cfg(feature = "settings-sync")]
    async fn sync_git(&self) -> Result<()> {
        if let Some(git) = &self.git_sync {
            // Pull latest changes
            git.pull().await?;

            // Sync settings
            self.sync_local().await?;

            // Push changes
            git.push("Sync settings").await?;
        }

        Ok(())
    }

    /// Sync with cloud (stubbed when feature disabled)
    async fn sync_cloud(&self) -> Result<()> {
        #[cfg(feature = "cloud-sync")]
        {
            if let Some(cloud) = &self.cloud_sync {
                // Collect all settings files
                let settings = self.collect_settings().await?;
                cloud.upload(&settings, "settings.json").await?;

                let profiles = self.collect_profiles().await?;
                cloud.upload(&profiles, "profiles.json").await?;

                let keymaps = self.collect_keymaps().await?;
                cloud.upload(&keymaps, "keymaps.json").await?;

                let themes = self.collect_themes().await?;
                cloud.upload(&themes, "themes.json").await?;

                let layouts = self.collect_layouts().await?;
                cloud.upload(&layouts, "layouts.json").await?;
            }
            return Ok(());
        }

        #[cfg(not(feature = "cloud-sync"))]
        {
            return Err(CustomizationError::SyncError("Cloud sync not enabled".to_string()));
        }
    }

    /// Sync with custom provider
    async fn sync_custom(&self) -> Result<()> {
        // Custom provider would be implemented by extensions
        Err(CustomizationError::SyncError("Custom sync not implemented".to_string()))
    }


    /// Collect all settings
    async fn collect_settings(&self) -> Result<Vec<u8>> {
        let settings = HashMap::<String, serde_json::Value>::new();
        Ok(serde_json::to_vec(&settings)?)
    }

    /// Collect all profiles
    async fn collect_profiles(&self) -> Result<Vec<u8>> {
        let profiles = Vec::<crate::profiles::ConfigurationProfile>::new();
        Ok(serde_json::to_vec(&profiles)?)
    }

    /// Collect all keymaps
    async fn collect_keymaps(&self) -> Result<Vec<u8>> {
        let keymaps = Vec::<crate::keybindings::Keymap>::new();
        Ok(serde_json::to_vec(&keymaps)?)
    }

    /// Collect all themes
    async fn collect_themes(&self) -> Result<Vec<u8>> {
        let themes = Vec::<crate::themes::Theme>::new();
        Ok(serde_json::to_vec(&themes)?)
    }

    /// Collect all layouts
    async fn collect_layouts(&self) -> Result<Vec<u8>> {
        let layouts = Vec::<crate::layouts::WorkspaceLayout>::new();
        Ok(serde_json::to_vec(&layouts)?)
    }

    /// Get sync status
    pub async fn status(&self) -> SyncStatus {
        self.status.read().await.clone()
    }

    /// Get last sync time
    pub async fn last_sync(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        *self.last_sync.read().await
    }

    /// Force sync now
    pub async fn sync_now(&self) -> Result<()> {
        self.sync().await
    }
}