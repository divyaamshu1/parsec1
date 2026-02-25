//! Extension handle for interacting with loaded extensions

use std::sync::Arc;
use anyhow::Result;
use serde_json::Value;
use crate::runtime::{ExtensionRuntime, ExtensionState};
use crate::runtime::memory::MemoryUsage;
use crate::ExtensionManifest;

/// Extension handle for interacting with loaded extensions
#[derive(Clone)]
pub struct ExtensionHandle {
    /// Extension ID
    id: String,
    /// Runtime reference
    runtime: Arc<ExtensionRuntime>,
}

impl ExtensionHandle {
    /// Create a new extension handle
    pub fn new(id: String, runtime: Arc<ExtensionRuntime>) -> Self {
        Self { id, runtime }
    }

    /// Get extension ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Activate the extension
    pub async fn activate(&self) -> Result<()> {
        self.runtime.activate_extension(&self.id).await
    }

    /// Deactivate the extension
    pub async fn deactivate(&self) -> Result<()> {
        self.runtime.deactivate_extension(&self.id).await
    }

    /// Unload the extension
    pub async fn unload(&self) -> Result<()> {
        self.runtime.unload_extension(&self.id).await
    }

    /// Send a message to the extension
    pub async fn send_message(&self, data: Vec<u8>) -> Result<()> {
        self.runtime.send_message(&self.id, data).await
    }

    /// Receive a message from the extension
    pub async fn receive_message(&self) -> Option<Vec<u8>> {
        self.runtime.receive_message(&self.id).await
    }

    /// Call a command
    pub async fn call_command(
        &self,
        command: &str,
        args: Vec<Value>,
    ) -> Result<Value> {
        self.runtime.call_command(&self.id, command, args).await
    }

    /// Register a command handler
    pub async fn register_command<F>(
        &self,
        command: &str,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(Vec<Value>) -> Result<Value> + Send + Sync + 'static,
    {
        self.runtime.register_command(&self.id, command, handler).await
    }

    /// Get extension state
    pub async fn state(&self) -> Option<ExtensionState> {
        self.runtime.get_extension_state(&self.id).await
    }

    /// Get memory usage
    pub async fn memory_usage(&self) -> Option<MemoryUsage> {
        self.runtime.memory_limiter().get_usage(&self.id).await
    }

    /// Check if extension is active
    pub async fn is_active(&self) -> bool {
        matches!(self.state().await, Some(ExtensionState::Active))
    }

    /// Get the extension's manifest
    pub async fn manifest(&self) -> Option<ExtensionManifest> {
        let instances = self.runtime.instances.read().await;
        instances.get(&self.id).map(|i| i.manifest.clone())
    }

    /// Get the extension's uptime
    pub async fn uptime(&self) -> Option<std::time::Duration> {
        let instances = self.runtime.instances.read().await;
        instances.get(&self.id).map(|i| i.uptime())
    }

    /// Check if the extension has a specific function
    pub async fn has_function(&self, name: &str) -> bool {
        let instances = self.runtime.instances.read().await;
        match instances.get(&self.id) {
            Some(i) => i.has_function(name).await,
            None => false,
        }
    }

    /// Get all exported functions
    pub async fn exported_functions(&self) -> Vec<String> {
        let instances = self.runtime.instances.read().await;
        match instances.get(&self.id) {
            Some(i) => i.exported_functions().await,
            None => Vec::new(),
        }
    }
}