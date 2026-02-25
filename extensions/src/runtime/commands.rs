//! Command handling for extensions

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use serde_json::Value;
use tokio::sync::RwLock;

/// Command handler type
pub type CommandHandler = Arc<dyn Fn(Vec<Value>) -> Result<Value> + Send + Sync>;

/// Command registry for an extension
#[derive(Default)]
pub struct CommandRegistry {
    commands: Arc<RwLock<HashMap<String, CommandHandler>>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a command handler
    pub async fn register<F>(&self, command: &str, handler: F)
    where
        F: Fn(Vec<Value>) -> Result<Value> + Send + Sync + 'static,
    {
        self.commands.write().await.insert(
            command.to_string(),
            Arc::new(handler) as CommandHandler,
        );
    }

    /// Get a command handler
    pub async fn get(&self, command: &str) -> Option<CommandHandler> {
        self.commands.read().await.get(command).cloned()
    }

    /// Check if command exists
    pub async fn has(&self, command: &str) -> bool {
        self.commands.read().await.contains_key(command)
    }

    /// Remove a command handler
    pub async fn remove(&self, command: &str) {
        self.commands.write().await.remove(command);
    }
}