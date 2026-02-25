//! Extension data stored in WASM store

use std::sync::Arc;
use std::time::Instant;
use anyhow::Result;
use tokio::sync::{RwLock, mpsc};
use wasmtime_wasi::WasiCtx;

use crate::runtime::memory::MemoryUsage;

/// Extension data stored in WASM store
pub struct ExtensionData {
    /// Extension ID
    pub id: String,
    /// Message sender
    pub message_tx: mpsc::UnboundedSender<Vec<u8>>,
    /// Memory usage tracker
    pub memory_usage: Arc<RwLock<MemoryUsage>>,
    /// Last activity time
    pub last_activity: Arc<RwLock<Instant>>,
    /// WASI context (optional)
    pub wasi_ctx: Option<WasiCtx>,
}

impl ExtensionData {
    /// Create new extension data
    pub fn new(
        id: String,
        message_tx: mpsc::UnboundedSender<Vec<u8>>,
        memory_usage: Arc<RwLock<MemoryUsage>>,
        last_activity: Arc<RwLock<Instant>>,
        wasi_ctx: Option<WasiCtx>,
    ) -> Self {
        Self {
            id,
            message_tx,
            memory_usage,
            last_activity,
            wasi_ctx,
        }
    }

    /// Get extension ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Send a message to the extension
    pub fn send_message(&self, data: Vec<u8>) -> Result<()> {
        self.message_tx.send(data)?;
        self.update_activity();
        Ok(())
    }

    /// Get memory usage
    pub fn memory_usage(&self) -> MemoryUsage {
        self.memory_usage.blocking_read().clone()
    }

    /// Get last activity timestamp
    pub fn last_activity(&self) -> Instant {
        *self.last_activity.blocking_read()
    }

    /// Update last activity timestamp
    pub fn update_activity(&self) {
        *self.last_activity.blocking_write() = Instant::now();
    }

    /// Record memory allocation
    pub fn record_allocation(&self, size: usize) {
        let mut usage = self.memory_usage.blocking_write();
        usage.record_allocation(size);
    }

    /// Record memory deallocation
    pub fn record_deallocation(&self, size: usize) {
        let mut usage = self.memory_usage.blocking_write();
        usage.record_deallocation(size);
    }

    /// Check if extension has been inactive for too long
    pub fn is_stale(&self, timeout: std::time::Duration) -> bool {
        self.last_activity.blocking_read().elapsed() > timeout
    }

    /// Get WASI context (if any)
    pub fn wasi_ctx(&self) -> Option<&WasiCtx> {
        self.wasi_ctx.as_ref()
    }

    /// Get mutable WASI context
    pub fn wasi_ctx_mut(&mut self) -> Option<&mut WasiCtx> {
        self.wasi_ctx.as_mut()
    }
}