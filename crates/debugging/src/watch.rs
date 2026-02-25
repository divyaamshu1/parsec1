//! Watch expressions

use std::collections::HashMap;

use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};

/// Watch expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchExpression {
    pub id: usize,
    pub expression: String,
    pub last_value: Option<String>,
    pub enabled: bool,
    pub expanded: bool,
}

/// Watch manager
pub struct WatchManager {
    watches: Arc<tokio::sync::RwLock<HashMap<usize, WatchExpression>>>,
    next_id: Arc<tokio::sync::Mutex<usize>>,
}

impl WatchManager {
    /// Create new watch manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            watches: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            next_id: Arc::new(tokio::sync::Mutex::new(1)),
        })
    }

    /// Add watch expression
    pub async fn add_watch(&self, expression: String) -> Result<usize> {
        let mut next_id = self.next_id.lock().await;
        let id = *next_id;
        *next_id += 1;

        let watch = WatchExpression {
            id,
            expression,
            last_value: None,
            enabled: true,
            expanded: false,
        };

        self.watches.write().await.insert(id, watch);
        Ok(id)
    }

    /// Remove watch
    pub async fn remove_watch(&self, id: usize) -> Result<()> {
        self.watches.write().await.remove(&id);
        Ok(())
    }

    /// Get watch
    pub async fn get_watch(&self, id: usize) -> Option<WatchExpression> {
        self.watches.read().await.get(&id).cloned()
    }

    /// Get all watches
    pub async fn get_watches(&self) -> Vec<WatchExpression> {
        self.watches.read().await.values().cloned().collect()
    }

    /// Update watch value
    pub async fn update_value(&self, id: usize, value: String) -> Result<()> {
        if let Some(watch) = self.watches.write().await.get_mut(&id) {
            watch.last_value = Some(value);
        }
        Ok(())
    }

    /// Enable watch
    pub async fn enable_watch(&self, id: usize) -> Result<()> {
        if let Some(watch) = self.watches.write().await.get_mut(&id) {
            watch.enabled = true;
        }
        Ok(())
    }

    /// Disable watch
    pub async fn disable_watch(&self, id: usize) -> Result<()> {
        if let Some(watch) = self.watches.write().await.get_mut(&id) {
            watch.enabled = false;
        }
        Ok(())
    }

    /// Toggle expanded
    pub async fn toggle_expanded(&self, id: usize) -> Result<()> {
        if let Some(watch) = self.watches.write().await.get_mut(&id) {
            watch.expanded = !watch.expanded;
        }
        Ok(())
    }

    /// Clear all watches
    pub async fn clear(&self) {
        self.watches.write().await.clear();
    }
}