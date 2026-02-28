//! Configuration profiles management

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::fs;
use serde::{Serialize, Deserialize};

use crate::{Result, CustomizationError, CustomizationConfig};

/// Profile scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProfileScope {
    Global,
    Workspace,
    Language,
    Project,
    Temporary,
}

/// Configuration profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationProfile {
    pub name: String,
    pub description: Option<String>,
    pub keymap: Option<String>,
    pub theme: Option<String>,
    pub layout: Option<String>,
    pub settings: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub version: String,
}

/// Profile metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMetadata {
    pub name: String,
    pub description: Option<String>,
    pub scope: ProfileScope,
    pub size: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
    pub version: String,
    pub dependencies: Vec<String>,
}

/// Profile diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDiff {
    pub profile_a: String,
    pub profile_b: String,
    pub added_settings: Vec<String>,
    pub removed_settings: Vec<String>,
    pub changed_settings: Vec<ChangedSetting>,
    pub keymap_changed: bool,
    pub theme_changed: bool,
    pub layout_changed: bool,
}

/// Changed setting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedSetting {
    pub key: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
}

/// Profile manager
pub struct ProfileManager {
    /// Available profiles
    profiles: Arc<RwLock<HashMap<String, ConfigurationProfile>>>,
    /// Profile metadata
    metadata: Arc<RwLock<HashMap<String, ProfileMetadata>>>,
    /// Active profile
    active_profile: Arc<RwLock<String>>,
    /// Configuration
    config: CustomizationConfig,
    /// Change listeners
    listeners: Arc<RwLock<Vec<tokio::sync::mpsc::UnboundedSender<ProfileEvent>>>>,
}

/// Profile event
#[derive(Debug, Clone)]
pub enum ProfileEvent {
    Created(String),
    Updated(String),
    Deleted(String),
    Activated(String),
    Deactivated(String),
    Imported(String),
    Exported(String),
}

impl ProfileManager {
    /// Create new profile manager
    pub async fn new(config: CustomizationConfig) -> Result<Self> {
        let manager = Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            active_profile: Arc::new(RwLock::new(config.default_profile.clone())),
            config: config.clone(),
            listeners: Arc::new(RwLock::new(Vec::new())),
        };

        // Load default profile
        manager.load_default_profile().await?;

        // Scan profiles directory
        manager.scan_profiles().await?;

        Ok(manager)
    }

    /// Load default profile
    async fn load_default_profile(&self) -> Result<()> {
        let profile = ConfigurationProfile {
            name: "default".to_string(),
            description: Some("Default configuration profile".to_string()),
            keymap: Some("default".to_string()),
            theme: Some("dark".to_string()),
            layout: Some("default".to_string()),
            settings: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let metadata = ProfileMetadata {
            name: "default".to_string(),
            description: Some("Default configuration profile".to_string()),
            scope: ProfileScope::Global,
            size: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: vec!["default".to_string()],
            version: env!("CARGO_PKG_VERSION").to_string(),
            dependencies: Vec::new(),
        };

        self.profiles.write().await.insert("default".to_string(), profile);
        self.metadata.write().await.insert("default".to_string(), metadata);

        Ok(())
    }

    /// Scan profiles directory
    async fn scan_profiles(&self) -> Result<()> {
        let mut read_dir = fs::read_dir(&self.config.profiles_dir).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(profile) = serde_json::from_str::<ConfigurationProfile>(&content) {
                        let name = profile.name.clone();
                        let metadata = self.create_metadata(&profile).await;
                        
                        self.profiles.write().await.insert(name.clone(), profile);
                        self.metadata.write().await.insert(name, metadata);
                    }
                }
            }
        }

        Ok(())
    }

    /// Create metadata for profile
    async fn create_metadata(&self, profile: &ConfigurationProfile) -> ProfileMetadata {
        ProfileMetadata {
            name: profile.name.clone(),
            description: profile.description.clone(),
            scope: ProfileScope::Global,
            size: std::mem::size_of_val(profile),
            created_at: profile.created_at,
            updated_at: profile.updated_at,
            tags: Vec::new(),
            version: profile.version.clone(),
            dependencies: Vec::new(),
        }
    }

    /// Create new profile
    pub async fn create_profile(&self, name: &str, description: Option<String>) -> Result<ConfigurationProfile> {
        let profile = ConfigurationProfile {
            name: name.to_string(),
            description,
            keymap: None,
            theme: None,
            layout: None,
            settings: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        self.save_profile(&profile).await?;
        self.notify(ProfileEvent::Created(name.to_string())).await;

        Ok(profile)
    }

    /// Save profile
    pub async fn save_profile(&self, profile: &ConfigurationProfile) -> Result<()> {
        let mut profiles = self.profiles.write().await;
        let mut metadata = self.metadata.write().await;

        let name = profile.name.clone();
        let meta = self.create_metadata(profile).await;

        profiles.insert(name.clone(), profile.clone());
        metadata.insert(name.clone(), meta);

        // Save to disk
        let path = self.config.profiles_dir.join(format!("{}.json", name));
        let json = serde_json::to_string_pretty(profile)?;
        fs::write(path, json).await?;

        drop(profiles);
        drop(metadata);

        self.notify(ProfileEvent::Updated(name)).await;

        Ok(())
    }

    /// Load profile
    pub async fn load_profile(&self, name: &str) -> Result<ConfigurationProfile> {
        let profiles = self.profiles.read().await;
        
        if let Some(profile) = profiles.get(name) {
            Ok(profile.clone())
        } else {
            // Try to load from disk
            let path = self.config.profiles_dir.join(format!("{}.json", name));
            if path.exists() {
                let content = fs::read_to_string(path).await?;
                let profile: ConfigurationProfile = serde_json::from_str(&content)?;
                
                // Cache in memory
                drop(profiles);
                self.profiles.write().await.insert(name.to_string(), profile.clone());
                
                Ok(profile)
            } else {
                Err(CustomizationError::ProfileError(format!("Profile not found: {}", name)))
            }
        }
    }

    /// Delete profile
    pub async fn delete_profile(&self, name: &str) -> Result<()> {
        self.profiles.write().await.remove(name);
        self.metadata.write().await.remove(name);

        let path = self.config.profiles_dir.join(format!("{}.json", name));
        if path.exists() {
            fs::remove_file(path).await?;
        }

        self.notify(ProfileEvent::Deleted(name.to_string())).await;

        Ok(())
    }

    /// List profiles
    pub async fn list_profiles(&self) -> Vec<ProfileMetadata> {
        self.metadata.read().await.values().cloned().collect()
    }

    /// Set active profile
    pub async fn set_active(&self, name: &str) -> Result<()> {
        if !self.profiles.read().await.contains_key(name) {
            return Err(CustomizationError::ProfileError(format!("Profile not found: {}", name)));
        }

        let old = self.active_profile.read().await.clone();
        *self.active_profile.write().await = name.to_string();

        if old != name {
            self.notify(ProfileEvent::Deactivated(old)).await;
            self.notify(ProfileEvent::Activated(name.to_string())).await;
        }

        Ok(())
    }

    /// Get active profile
    pub async fn active_profile(&self) -> String {
        self.active_profile.read().await.clone()
    }

    /// Get profile metadata
    pub async fn get_metadata(&self, name: &str) -> Option<ProfileMetadata> {
        self.metadata.read().await.get(name).cloned()
    }

    /// Update profile setting
    pub async fn update_setting(&self, name: &str, key: &str, value: serde_json::Value) -> Result<()> {
        let mut profiles = self.profiles.write().await;
        
        if let Some(profile) = profiles.get_mut(name) {
            profile.settings.insert(key.to_string(), value);
            profile.updated_at = chrono::Utc::now();
            
            // Update metadata
            if let Some(meta) = self.metadata.write().await.get_mut(name) {
                meta.updated_at = profile.updated_at;
                meta.size = std::mem::size_of_val(profile);
            }

            self.notify(ProfileEvent::Updated(name.to_string())).await;
        }

        Ok(())
    }

    /// Remove setting from profile
    pub async fn remove_setting(&self, name: &str, key: &str) -> Result<()> {
        let mut profiles = self.profiles.write().await;
        
        if let Some(profile) = profiles.get_mut(name) {
            profile.settings.remove(key);
            profile.updated_at = chrono::Utc::now();
            
            // Update metadata
            if let Some(meta) = self.metadata.write().await.get_mut(name) {
                meta.updated_at = profile.updated_at;
                meta.size = std::mem::size_of_val(profile);
            }

            self.notify(ProfileEvent::Updated(name.to_string())).await;
        }

        Ok(())
    }

    /// Clone profile
    pub async fn clone_profile(&self, source: &str, target: &str) -> Result<ConfigurationProfile> {
        let mut profile = self.load_profile(source).await?;
        profile.name = target.to_string();
        profile.created_at = chrono::Utc::now();
        profile.updated_at = chrono::Utc::now();

        self.save_profile(&profile).await?;
        Ok(profile)
    }

    /// Export profile
    pub async fn export_profile(&self, name: &str, format: &str) -> Result<Vec<u8>> {
        let profile = self.load_profile(name).await?;

        match format {
            "json" => Ok(serde_json::to_vec_pretty(&profile)?),
            "yaml" => Ok(serde_yaml::to_string(&profile)?.into_bytes()),
            "toml" => Ok(toml::to_string(&profile)?.into_bytes()),
            _ => Err(CustomizationError::ProfileError(format!("Unsupported format: {}", format))),
        }
    }

    /// Import profile
    pub async fn import_profile(&self, data: &[u8], format: &str) -> Result<ConfigurationProfile> {
        let profile: ConfigurationProfile = match format {
            "json" => serde_json::from_slice(data)?,
            "yaml" => serde_yaml::from_slice(data)?,
            "toml" => toml::from_slice(data)?,
            _ => return Err(CustomizationError::ProfileError(format!("Unsupported format: {}", format))),
        };

        self.save_profile(&profile).await?;
        self.notify(ProfileEvent::Imported(profile.name.clone())).await;

        Ok(profile)
    }

    /// Compare profiles
    pub async fn diff(&self, profile_a: &str, profile_b: &str) -> Result<ProfileDiff> {
        let a = self.load_profile(profile_a).await?;
        let b = self.load_profile(profile_b).await?;

        let a_keys: HashSet<_> = a.settings.keys().collect();
        let b_keys: HashSet<_> = b.settings.keys().collect();

        let added = b_keys.difference(&a_keys).map(|k| (*k).clone()).collect();
        let removed = a_keys.difference(&b_keys).map(|k| (*k).clone()).collect();

        let mut changed = Vec::new();
        for key in a_keys.intersection(&b_keys) {
            let a_val = a.settings.get(*key);
            let b_val = b.settings.get(*key);
            if a_val != b_val {
                changed.push(ChangedSetting {
                    key: (*key).clone(),
                    old_value: a_val.cloned(),
                    new_value: b_val.cloned(),
                });
            }
        }

        Ok(ProfileDiff {
            profile_a: profile_a.to_string(),
            profile_b: profile_b.to_string(),
            added_settings: added,
            removed_settings: removed,
            changed_settings: changed,
            keymap_changed: a.keymap != b.keymap,
            theme_changed: a.theme != b.theme,
            layout_changed: a.layout != b.layout,
        })
    }

    /// Merge profiles
    pub async fn merge(&self, profiles: Vec<&str>, strategy: MergeStrategy) -> Result<ConfigurationProfile> {
        let mut merged = ConfigurationProfile {
            name: format!("merged-{}", chrono::Utc::now().timestamp()),
            description: Some("Merged profile".to_string()),
            keymap: None,
            theme: None,
            layout: None,
            settings: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let loaded_profiles: Vec<ConfigurationProfile> = futures::future::join_all(
            profiles.iter().map(|&name| self.load_profile(name))
        ).await.into_iter().collect::<Result<Vec<_>>>()?;

        match strategy {
            MergeStrategy::FirstWins => {
                if let Some(first) = loaded_profiles.first() {
                    merged.keymap = first.keymap.clone();
                    merged.theme = first.theme.clone();
                    merged.layout = first.layout.clone();
                    merged.settings = first.settings.clone();
                }
            }
            MergeStrategy::LastWins => {
                if let Some(last) = loaded_profiles.last() {
                    merged.keymap = last.keymap.clone();
                    merged.theme = last.theme.clone();
                    merged.layout = last.layout.clone();
                    merged.settings = last.settings.clone();
                }
            }
            MergeStrategy::Union => {
                for profile in &loaded_profiles {
                    if profile.keymap.is_some() {
                        merged.keymap = profile.keymap.clone();
                    }
                    if profile.theme.is_some() {
                        merged.theme = profile.theme.clone();
                    }
                    if profile.layout.is_some() {
                        merged.layout = profile.layout.clone();
                    }
                    merged.settings.extend(profile.settings.clone());
                }
            }
            MergeStrategy::Intersection => {
                if let Some(first) = loaded_profiles.first() {
                    merged.keymap = first.keymap.clone();
                    merged.theme = first.theme.clone();
                    merged.layout = first.layout.clone();
                    
                    let common_keys: HashSet<_> = loaded_profiles.iter()
                        .map(|p| p.settings.keys().collect::<HashSet<_>>())
                        .reduce(|a, b| a.intersection(&b).cloned().collect())
                        .unwrap_or_default();

                    for key in common_keys {
                        if let Some(value) = first.settings.get(key) {
                            merged.settings.insert(key.clone(), value.clone());
                        }
                    }
                }
            }
        }

        Ok(merged)
    }

    /// Add event listener
    pub async fn add_listener(&self) -> tokio::sync::mpsc::UnboundedReceiver<ProfileEvent> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.listeners.write().await.push(tx);
        rx
    }

    /// Notify listeners
    async fn notify(&self, event: ProfileEvent) {
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            let _ = listener.send(event.clone());
        }
    }
}

/// Merge strategy
#[derive(Debug, Clone, Copy)]
pub enum MergeStrategy {
    FirstWins,
    LastWins,
    Union,
    Intersection,
}