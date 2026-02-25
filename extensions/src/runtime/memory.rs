//! Memory management for extensions
//!
//! Provides memory tracking, limits, and allocation monitoring.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use tokio::sync::{RwLock, Mutex};
use serde::{Serialize, Deserialize};

/// Memory usage tracker for a single extension
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// Current memory usage in bytes
    pub current: usize,
    /// Peak memory usage in bytes
    pub peak: usize,
    /// Memory limit in bytes
    pub limit: usize,
    /// Number of allocations
    pub allocations: u64,
    /// Number of deallocations
    pub deallocations: u64,
    /// Allocation sites (for debugging)
    #[cfg(debug_assertions)]
    pub allocation_sites: Vec<AllocationSite>,
}

#[cfg(debug_assertions)]
#[derive(Debug, Clone, Serialize)]
pub struct AllocationSite {
    pub size: usize,
    pub stack_trace: String,
    #[serde(skip_serializing, skip_deserializing, default = "now_instant")]
    pub timestamp: std::time::Instant,
}

fn now_instant() -> std::time::Instant {
    std::time::Instant::now()
}

// Custom Deserialize to avoid requiring serde support for std::time::Instant
#[cfg(debug_assertions)]
impl<'de> serde::Deserialize<'de> for AllocationSite {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Helper {
            size: usize,
            stack_trace: String,
        }

        let h = Helper::deserialize(deserializer)?;
        Ok(AllocationSite {
            size: h.size,
            stack_trace: h.stack_trace,
            timestamp: now_instant(),
        })
    }
}

impl MemoryUsage {
    /// Record an allocation
    pub fn record_allocation(&mut self, size: usize) {
        self.current += size;
        self.peak = self.peak.max(self.current);
        self.allocations += 1;
    }

    /// Record a deallocation
    pub fn record_deallocation(&mut self, size: usize) {
        self.current = self.current.saturating_sub(size);
        self.deallocations += 1;
    }

    /// Check if within limit
    pub fn within_limit(&self) -> bool {
        self.current <= self.limit
    }

    /// Get usage percentage (0.0 - 1.0)
    pub fn usage_percent(&self) -> f32 {
        if self.limit == 0 {
            0.0
        } else {
            self.current as f32 / self.limit as f32
        }
    }
}

/// Global memory limiter for all extensions
pub struct MemoryLimiter {
    /// Memory usage per extension
    extensions: Arc<RwLock<HashMap<String, Arc<RwLock<MemoryUsage>>>>>,
    /// Total memory limit
    total_limit: usize,
    /// Per-extension limit
    per_extension_limit: usize,
    /// Current total usage
    total_used: Arc<RwLock<usize>>,
    /// Peak total usage
    peak_used: Arc<RwLock<usize>>,
}

impl MemoryLimiter {
    /// Create a new memory limiter
    pub fn new(total_limit: usize, per_extension_limit: usize) -> Self {
        Self {
            extensions: Arc::new(RwLock::new(HashMap::new())),
            total_limit,
            per_extension_limit,
            total_used: Arc::new(RwLock::new(0)),
            peak_used: Arc::new(RwLock::new(0)),
        }
    }

    /// Register a new extension
    pub async fn register_extension(&self, extension_id: &str) -> Result<Arc<RwLock<MemoryUsage>>> {
        let usage = Arc::new(RwLock::new(MemoryUsage {
            limit: self.per_extension_limit,
            ..Default::default()
        }));

        self.extensions.write().await.insert(extension_id.to_string(), usage.clone());

        Ok(usage)
    }

    /// Unregister an extension
    pub async fn unregister_extension(&self, extension_id: &str) -> Result<()> {
        if let Some(usage) = self.extensions.write().await.remove(extension_id) {
            let current = usage.read().await.current;
            *self.total_used.write().await -= current;
        }
        Ok(())
    }

    /// Allocate memory for an extension
    pub async fn allocate(&self, extension_id: &str, size: usize) -> Result<()> {
        // Check total limit first
        {
            let total = *self.total_used.read().await;
            if total + size > self.total_limit {
                return Err(anyhow!("Total memory limit exceeded"));
            }
        }

        // Check extension limit
        let extensions = self.extensions.read().await;
        if let Some(usage) = extensions.get(extension_id) {
            let mut usage = usage.write().await;
            if usage.current + size > usage.limit {
                return Err(anyhow!("Extension memory limit exceeded"));
            }

            usage.record_allocation(size);
        }

        // Update total
        let mut total = self.total_used.write().await;
        *total += size;
        if *total > *self.peak_used.read().await {
            let mut peak = self.peak_used.write().await;
            *peak = *total;
        }

        Ok(())
    }

    /// Deallocate memory for an extension
    pub async fn deallocate(&self, extension_id: &str, size: usize) {
        // Update extension
        let extensions = self.extensions.read().await;
        if let Some(usage) = extensions.get(extension_id) {
            let mut usage = usage.write().await;
            usage.record_deallocation(size);
        }

        // Update total
        let mut total = self.total_used.write().await;
        *total = total.saturating_sub(size);
    }

    /// Get memory usage for an extension
    pub async fn get_usage(&self, extension_id: &str) -> Option<MemoryUsage> {
        let extensions = self.extensions.read().await;
        match extensions.get(extension_id) {
            Some(u) => Some(u.read().await.clone()),
            None => None,
        }
    }

    /// Get total memory used
    pub async fn total_used(&self) -> usize {
        *self.total_used.read().await
    }

    /// Get peak memory used
    pub async fn peak_used(&self) -> usize {
        *self.peak_used.read().await
    }

    /// Get total limit
    pub fn total_limit(&self) -> usize {
        self.total_limit
    }

    /// Get per-extension limit
    pub fn per_extension_limit(&self) -> usize {
        self.per_extension_limit
    }

    /// Check if an extension has enough memory
    pub async fn has_enough_memory(&self, extension_id: &str, size: usize) -> bool {
        let extensions = self.extensions.read().await;
        if let Some(usage) = extensions.get(extension_id) {
            let usage = usage.read().await;
            usage.current + size <= usage.limit
        } else {
            false
        }
    }

    /// Get memory pressure (0.0 - 1.0)
    pub async fn memory_pressure(&self) -> f32 {
        let total = *self.total_used.read().await;
        total as f32 / self.total_limit as f32
    }

    /// Reset peak memory for an extension
    pub async fn reset_peak(&self, extension_id: &str) {
        let extensions = self.extensions.read().await;
        if let Some(usage) = extensions.get(extension_id) {
            let mut usage = usage.write().await;
            usage.peak = usage.current;
        }
    }

    /// Get all memory statistics
    pub async fn statistics(&self) -> MemoryStatistics {
        let extensions = self.extensions.read().await;
        let total = *self.total_used.read().await;
        let peak = *self.peak_used.read().await;

        let mut ext_stats = HashMap::new();
        for (id, usage) in extensions.iter() {
            ext_stats.insert(id.clone(), usage.read().await.clone());
        }

        MemoryStatistics {
            total_used: total,
            peak_used: peak,
            total_limit: self.total_limit,
            per_extension_limit: self.per_extension_limit,
            extensions: ext_stats,
            pressure: total as f32 / self.total_limit as f32,
        }
    }
}

/// Memory statistics snapshot
#[derive(Debug, Clone)]
pub struct MemoryStatistics {
    pub total_used: usize,
    pub peak_used: usize,
    pub total_limit: usize,
    pub per_extension_limit: usize,
    pub extensions: HashMap<String, MemoryUsage>,
    pub pressure: f32,
}

/// WASM memory allocator for tracking
pub struct WasmAllocator {
    /// Memory instance
    memory: wasmtime::Memory,
    /// Current allocation pointer
    next_addr: Arc<Mutex<usize>>,
    /// Allocation tracking
    allocations: Arc<Mutex<HashMap<usize, usize>>>,
    /// Memory usage tracker
    usage: Arc<RwLock<MemoryUsage>>,
}

impl WasmAllocator {
    /// Create a new allocator
    pub fn new(memory: wasmtime::Memory, usage: Arc<RwLock<MemoryUsage>>) -> Self {
        Self {
            memory,
            next_addr: Arc::new(Mutex::new(0)),
            allocations: Arc::new(Mutex::new(HashMap::new())),
            usage,
        }
    }

    /// Allocate memory
    pub async fn allocate(&self, size: usize) -> Result<usize> {
        let mut next = self.next_addr.lock().await;
        let addr = *next;
        *next += size;
        
        // Simple bounds check - assume 1GB max virtual memory
        const MAX_MEMORY: usize = 1024 * 1024 * 1024;
        if addr + size > MAX_MEMORY {
            return Err(anyhow!("Out of memory"));
        }

        // Track allocation
        self.allocations.lock().await.insert(addr, size);

        // Record in usage
        self.usage.write().await.record_allocation(size);

        Ok(addr)
    }

    /// Deallocate memory
    pub async fn deallocate(&self, addr: usize) -> Result<()> {
        if let Some(size) = self.allocations.lock().await.remove(&addr) {
            // Memory can't be freed in linear memory, but we can track it
            self.usage.write().await.record_deallocation(size);
        }
        Ok(())
    }

    /// Write data to memory
    pub async fn write_data(&self, addr: usize, data: &[u8]) -> Result<()> {
        const MAX_MEMORY: usize = 1024 * 1024 * 1024;
        if addr + data.len() > MAX_MEMORY {
            return Err(anyhow!("Write out of bounds"));
        }

        // Note: Actual memory write would require Store context
        // This is a placeholder that validates bounds
        let mut allocations = self.allocations.lock().await;
        allocations.insert(addr, data.len());

        Ok(())
    }

    /// Read data from memory
    pub async fn read_data(&self, addr: usize, len: usize) -> Result<Vec<u8>> {
        const MAX_MEMORY: usize = 1024 * 1024 * 1024;
        if addr + len > MAX_MEMORY {
            return Err(anyhow!("Read out of bounds"));
        }

        // Return placeholder data  
        // Actual memory read would require Store context
        Ok(vec![0u8; len])
    }

    /// Read string from memory (null-terminated)
    pub async fn read_string(&self, _addr: usize) -> Result<String> {
        // Would require Store context to access actual memory data
        // Return empty string as placeholder
        Ok(String::new())
    }

    /// Write string to memory (with null terminator)
    pub async fn write_string(&self, addr: usize, s: &str) -> Result<usize> {
        const MAX_MEMORY: usize = 1024 * 1024 * 1024;
        let bytes = s.as_bytes();
        
        // Check bounds
        if addr + bytes.len() + 1 > MAX_MEMORY {
            return Err(anyhow!("Write out of bounds"));
        }

        // Would require Store context for actual write
        Ok(bytes.len() + 1)
    }

    /// Get current memory usage
    pub async fn usage(&self) -> MemoryUsage {
        self.usage.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_usage() {
        let mut usage = MemoryUsage::default();
        usage.record_allocation(1024);
        assert_eq!(usage.current, 1024);
        assert_eq!(usage.peak, 1024);

        usage.record_allocation(512);
        assert_eq!(usage.current, 1536);
        assert_eq!(usage.peak, 1536);

        usage.record_deallocation(512);
        assert_eq!(usage.current, 1024);
        assert_eq!(usage.peak, 1536);
    }

    #[tokio::test]
    async fn test_memory_limiter() {
        let limiter = MemoryLimiter::new(10_000, 5_000);

        // Register extension
        let usage = limiter.register_extension("test1").await.unwrap();
        assert_eq!(usage.read().await.limit, 5_000);

        // Allocate memory
        assert!(limiter.allocate("test1", 2_000).await.is_ok());
        assert_eq!(limiter.total_used().await, 2_000);

        // Check extension usage
        let usage = limiter.get_usage("test1").await.unwrap();
        assert_eq!(usage.current, 2_000);

        // Try to exceed limit
        assert!(limiter.allocate("test1", 4_000).await.is_err());

        // Deallocate
        limiter.deallocate("test1", 1_000).await;
        assert_eq!(limiter.total_used().await, 1_000);

        // Unregister
        limiter.unregister_extension("test1").await.unwrap();
        assert_eq!(limiter.total_used().await, 0);
    }

    #[tokio::test]
    async fn test_multiple_extensions() {
        let limiter = MemoryLimiter::new(10_000, 3_000);

        limiter.register_extension("ext1").await.unwrap();
        limiter.register_extension("ext2").await.unwrap();

        assert!(limiter.allocate("ext1", 3_000).await.is_ok());
        assert!(limiter.allocate("ext2", 3_000).await.is_ok());

        // Total used should be 6,000
        assert_eq!(limiter.total_used().await, 6_000);

        // Try to exceed total limit
        assert!(limiter.allocate("ext1", 5_000).await.is_err());

        let stats = limiter.statistics().await;
        assert_eq!(stats.total_used, 6_000);
        assert_eq!(stats.extensions.len(), 2);
    }

    #[tokio::test]
    async fn test_memory_pressure() {
        let limiter = MemoryLimiter::new(10_000, 2_000);

        limiter.register_extension("test").await.unwrap();
        limiter.allocate("test", 5_000).await.unwrap();

        let pressure = limiter.memory_pressure().await;
        assert!((pressure - 0.5).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_peak_memory() {
        let limiter = MemoryLimiter::new(10_000, 5_000);

        limiter.register_extension("test").await.unwrap();

        limiter.allocate("test", 3_000).await.unwrap();
        assert_eq!(limiter.peak_used().await, 3_000);

        limiter.allocate("test", 2_000).await.unwrap();
        assert_eq!(limiter.peak_used().await, 5_000);

        limiter.deallocate("test", 2_000).await;
        assert_eq!(limiter.peak_used().await, 5_000); // Peak doesn't decrease
    }

    // WASM allocator tests would need actual WASM memory
    // These are placeholders
    #[test]
    fn test_wasm_allocator_creation() {
        // Would need a real wasmtime::Memory
    }
}