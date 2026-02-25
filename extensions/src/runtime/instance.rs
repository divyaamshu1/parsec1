//! Running extension instance

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use tokio::sync::{RwLock, Mutex, mpsc};
use wasmtime::{Store, Instance, Memory, TypedFunc, Val};

use crate::runtime::{ExtensionData, ExtensionExports, CommandHandler};
use crate::runtime::memory::MemoryUsage;
use crate::{ExtensionManifest, ExtensionState};

/// Running extension instance
pub struct ExtensionInstance {
    /// Extension ID
    pub id: String,
    /// Extension manifest
    pub manifest: ExtensionManifest,
    /// WASM store
    pub(crate) store: Arc<RwLock<Store<ExtensionData>>>,
    /// WASM instance
    pub(crate) instance: Instance,
    /// Extension state
    pub(crate) state: Arc<RwLock<ExtensionState>>,
    /// Message channel to extension
    pub(crate) message_tx: mpsc::UnboundedSender<Vec<u8>>,
    /// Message channel from extension
    pub(crate) message_rx: Arc<Mutex<mpsc::UnboundedReceiver<Vec<u8>>>>,
    /// Memory usage
    pub(crate) memory_usage: Arc<RwLock<MemoryUsage>>,
    /// Start time
    pub(crate) start_time: Instant,
    /// Last activity
    pub(crate) last_activity: Arc<RwLock<Instant>>,
    /// WASM exports
    pub(crate) exports: ExtensionExports,
    /// Command handlers
    pub(crate) commands: Arc<RwLock<HashMap<String, CommandHandler>>>,
}

impl ExtensionInstance {
    /// Create a new extension instance (internal)
    pub(crate) fn new(
        id: String,
        manifest: ExtensionManifest,
        store: Store<ExtensionData>,
        instance: Instance,
        message_tx: mpsc::UnboundedSender<Vec<u8>>,
        message_rx: mpsc::UnboundedReceiver<Vec<u8>>,
        memory_usage: Arc<RwLock<MemoryUsage>>,
        exports: ExtensionExports,
    ) -> Self {
        Self {
            id,
            manifest,
            store: Arc::new(RwLock::new(store)),
            instance,
            state: Arc::new(RwLock::new(ExtensionState::Inactive)),
            message_tx,
            message_rx: Arc::new(Mutex::new(message_rx)),
            memory_usage,
            start_time: Instant::now(),
            last_activity: Arc::new(RwLock::new(Instant::now())),
            exports,
            commands: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if extension is active
    pub fn is_active(&self) -> bool {
        matches!(*self.state.blocking_read(), ExtensionState::Active)
    }

    /// Get extension state
    pub fn state(&self) -> ExtensionState {
        *self.state.blocking_read()
    }

    /// Set extension state (internal)
    pub(crate) fn set_state(&self, state: ExtensionState) {
        *self.state.blocking_write() = state;
    }

    /// Get memory usage
    pub async fn memory_usage(&self) -> MemoryUsage {
        self.memory_usage.read().await.clone()
    }

    /// Get last activity time
    pub fn last_activity(&self) -> Instant {
        *self.last_activity.blocking_read()
    }

    /// Update last activity
    pub fn update_activity(&self) {
        *self.last_activity.blocking_write() = Instant::now();
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Call a function on the extension
    pub async fn call_function(
        &self,
        name: &str,
        args: &[Val],
    ) -> Result<Vec<Val>> {
        self.update_activity();
        
        // Try 0-param function
        if let Ok(func) = self.instance.get_typed_func::<(), ()>(&mut *self.store.write().await, name) {
            func.call(&mut *self.store.write().await, ())?;
            return Ok(Vec::new());
        }
        
        // Try 1-param i32 function
        if let Ok(func) = self.instance.get_typed_func::<i32, ()>(&mut *self.store.write().await, name) {
            if let Some(Val::I32(arg)) = args.first() {
                func.call(&mut *self.store.write().await, *arg)?;
                return Ok(Vec::new());
            }
        }
        
        // Try 2-param i32 function
        if let Ok(func) = self.instance.get_typed_func::<(i32, i32), ()>(&mut *self.store.write().await, name) {
            if args.len() >= 2 {
                if let (Val::I32(a1), Val::I32(a2)) = (&args[0], &args[1]) {
                    func.call(&mut *self.store.write().await, (*a1, *a2))?;
                    return Ok(Vec::new());
                }
            }
        }
        
        Err(anyhow!("Function {} not found or signature mismatch", name))
    }

    /// Get exported memory
    pub async fn memory(&self) -> Option<Memory> {
        self.instance.get_memory(&mut *self.store.write().await, "memory")
    }

    /// Check if extension exports a function
    pub async fn has_function(&self, name: &str) -> bool {
        self.instance.get_export(&mut *self.store.write().await, name).is_some()
    }

    /// Get all exported function names
    pub async fn exported_functions(&self) -> Vec<String> {
        let mut functions = Vec::new();
        let mut store = self.store.write().await;
        for export in self.instance.exports(&mut *store) {
            functions.push(export.name().to_string());
        }
        functions
    }

    /// Register a command handler
    pub async fn register_command<F>(
        &self,
        command: &str,
        handler: F,
    ) where
        F: Fn(Vec<serde_json::Value>) -> Result<serde_json::Value> + Send + Sync + 'static,
    {
        let boxed: Box<dyn Fn(Vec<serde_json::Value>) -> Result<serde_json::Value> + Send + Sync> = Box::new(handler);
        self.commands.write().await.insert(
            command.to_string(),
            Arc::new(boxed) as CommandHandler,
        );
    }

    /// Handle a command (internal)
    pub async fn handle_command(
        &self,
        command: &str,
        args: Vec<serde_json::Value>,
    ) -> Option<Result<serde_json::Value>> {
        let handlers = self.commands.read().await;
        handlers.get(command).map(|handler| handler(args))
    }

    /// Send a message to the extension (host -> extension)
    pub fn send_message(&self, data: Vec<u8>) -> Result<()> {
        self.message_tx.send(data)?;
        self.update_activity();
        Ok(())
    }

    /// Receive a message from the extension (extension -> host)
    pub async fn receive_message(&self) -> Option<Vec<u8>> {
        let mut rx = self.message_rx.lock().await;
        rx.recv().await
    }

    /// Try to receive a message without blocking
    pub fn try_receive_message(&self) -> Option<Vec<u8>> {
        let mut rx = self.message_rx.blocking_lock();
        rx.try_recv().ok()
    }
}