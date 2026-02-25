//! Resource limiter for WASM instances

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use anyhow::Result;
use tracing::warn;

/// Resource limiter for WASM instances
#[derive(Clone)]
pub struct ResourceLimiter {
    /// Maximum memory in bytes
    max_memory: usize,
    /// Maximum table size
    max_table_size: u32,
    /// Current memory usage
    current_memory: Arc<AtomicUsize>,
}

impl ResourceLimiter {
    pub fn new(max_memory: usize) -> Self {
        Self {
            max_memory,
            max_table_size: 10000,
            current_memory: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl wasmtime::ResourceLimiter for ResourceLimiter {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool> {
        let current_total = self.current_memory.load(Ordering::Relaxed);
        let new_total = current_total + (desired - current);
        
        if new_total > self.max_memory {
            warn!("Memory limit exceeded: {} > {}", new_total, self.max_memory);
            return Ok(false);
        }
        
        self.current_memory.store(new_total, Ordering::Relaxed);
        Ok(true)
    }

    fn table_growing(&mut self, _current: usize, desired: usize, _maximum: Option<usize>) -> Result<bool> {
        Ok(desired <= self.max_table_size as usize)
    }
}