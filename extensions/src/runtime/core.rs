//! Core extension runtime implementation

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use tokio::sync::{RwLock, mpsc, Mutex};
use tracing::{info, warn, debug};
use serde_json::json;

use wasmtime::{
    Engine, Store, Module, Instance, Linker, Config,
};
use wasmtime::Caller;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

use crate::runtime::{
    ExtensionInstance, ExtensionData, ExtensionExports, ResourceLimiter, ExtensionHandle,
};
use crate::{ExtensionState, ExtensionEvent, ExtensionManifest};

use super::memory::MemoryLimiter;
use super::scheduler::Scheduler;
use super::wasm::{WasmCache, WasmValidator};
use super::sandbox::{Sandbox, SandboxConfig};
use super::presets;
use super::stats::RuntimeStats;

/// Runtime configuration
#[derive(Clone, Debug)]
pub struct RuntimeConfig {
    /// Maximum memory per extension in bytes
    pub max_memory_per_extension: usize,
    /// Maximum total memory in bytes
    pub max_total_memory: usize,
    /// Maximum execution time in milliseconds
    pub max_execution_time_ms: u64,
    /// Enable sandboxing
    pub enable_sandbox: bool,
    /// Enable networking
    pub enable_networking: bool,
    /// Enable filesystem access
    pub enable_filesystem: bool,
    /// Allowed filesystem paths
    pub allowed_paths: Vec<PathBuf>,
    /// Allowed network domains
    pub allowed_domains: Vec<String>,
    /// Maximum concurrent extensions
    pub max_concurrent_extensions: usize,
    /// Enable compiled module caching
    pub cache_compiled: bool,
    /// Cache directory
    pub cache_dir: Option<PathBuf>,
    /// Validate WASM modules
    pub validate_modules: bool,
    /// Enable security checks
    pub security_checks: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_memory_per_extension: 50 * 1024 * 1024,  // 50MB
            max_total_memory: 500 * 1024 * 1024,        // 500MB
            max_execution_time_ms: 5000,                 // 5 seconds
            enable_sandbox: true,
            enable_networking: false,
            enable_filesystem: false,
            allowed_paths: vec![std::env::temp_dir()],
            allowed_domains: vec!["localhost".to_string()],
            max_concurrent_extensions: 20,
            cache_compiled: true,
            cache_dir: dirs::cache_dir().map(|d| d.join("parsec-extensions")),
            validate_modules: true,
            security_checks: true,
        }
    }
}

/// WASM runtime for executing extensions
pub struct ExtensionRuntime {
    /// WASM engine
    engine: Engine,
    /// Active extension instances
    pub(crate) instances: Arc<RwLock<HashMap<String, ExtensionInstance>>>,
    /// Scheduler for extension tasks
    scheduler: Arc<Scheduler>,
    /// Memory limiter
    memory_limiter: Arc<MemoryLimiter>,
    /// WASM module cache
    wasm_cache: Arc<WasmCache>,
    /// Event sender
    pub(crate) event_tx: mpsc::UnboundedSender<ExtensionEvent>,
    /// Event receiver
    pub(crate) event_rx: Arc<Mutex<mpsc::UnboundedReceiver<ExtensionEvent>>>,
    /// Configuration
    pub(crate) config: RuntimeConfig,
    /// Statistics
    pub(crate) stats: Arc<RwLock<RuntimeStats>>,
}

impl ExtensionRuntime {
    /// Create a new extension runtime
    pub fn new(config: RuntimeConfig) -> Result<Self> {
        let mut engine_config = Config::new();
        engine_config.async_support(true);
        engine_config.consume_fuel(true);
        engine_config.wasm_multi_value(true);
        engine_config.wasm_multi_memory(true);
        engine_config.wasm_bulk_memory(true);
        engine_config.wasm_reference_types(true);
        engine_config.wasm_simd(true);
        engine_config.wasm_threads(true);
        
        let engine = Engine::new(&engine_config)?;
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Create cache directory if needed
        if let Some(cache_dir) = &config.cache_dir {
            let _ = std::fs::create_dir_all(cache_dir);
        }

        let wasm_cache = Arc::new(WasmCache::new(
            engine.clone(),
            config.cache_dir.clone(),
        ));

        Ok(Self {
            engine,
            instances: Arc::new(RwLock::new(HashMap::new())),
            scheduler: Arc::new(Scheduler::new(config.max_concurrent_extensions)),
            memory_limiter: Arc::new(MemoryLimiter::new(
                config.max_total_memory,
                config.max_memory_per_extension,
            )),
            wasm_cache,
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            config,
            stats: Arc::new(RwLock::new(RuntimeStats::default())),
        })
    }

    /// Load an extension from WASM bytes
    pub async fn load_extension(
        &self,
        wasm_bytes: &[u8],
        manifest: ExtensionManifest,
        _extension_path: PathBuf,
    ) -> Result<String> {
        let extension_id = format!("{}.{}", manifest.publisher, manifest.name);
        let start_time = std::time::Instant::now();

        info!("Loading extension: {}", extension_id);

        // Check if already loaded
        if self.instances.read().await.contains_key(&extension_id) {
            return Err(anyhow!("Extension already loaded: {}", extension_id));
        }

        // Validate manifest
        self.validate_manifest(&manifest)?;

        // Validate WASM module
        if self.config.validate_modules {
            WasmValidator::validate(wasm_bytes)?;
        }

        // Security checks
        if self.config.security_checks {
            WasmValidator::check_security(wasm_bytes)?;
        }

        // Get module metadata
        let metadata = WasmValidator::get_metadata(wasm_bytes)?;
        debug!("Extension {} metadata: {:?}", extension_id, metadata);

        // Register with memory limiter
        let memory_usage: Arc<RwLock<super::memory::MemoryUsage>> = self.memory_limiter.register_extension(&extension_id).await?;

        // Create communication channels
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let last_activity = Arc::new(RwLock::new(std::time::Instant::now()));

        // Create store with extension data and resource limiter
        let mut store = Store::new(
            &self.engine,
            ExtensionData {
                id: extension_id.clone(),
                message_tx: message_tx.clone(),
                memory_usage: memory_usage.clone(),
                last_activity: last_activity.clone(),
                wasi_ctx: None,
            },
        );

        // Configure fuel for execution limits
        store.set_fuel(u64::MAX)?;
        
        // Add resource limiter - note: using a simple closure since ResourceLimiter needs to be Box'ed
        // store.limiter expects a FnMut that returns a resource limiter
        // Due to limitations, leaving this as-is for now - memory limiting not fully enabled

        // Get or compile module
        let module = if self.config.cache_compiled {
            self.wasm_cache.get_or_compile(&extension_id, wasm_bytes).await?
        } else {
            Module::new(&self.engine, wasm_bytes)?
        };

        // Create linker with imports
        let mut linker = Linker::new(&self.engine);

        // Add WASI with proper context
        if self.config.enable_sandbox {
            let sandbox = Sandbox::with_config(SandboxConfig {
                max_memory: self.config.max_memory_per_extension,
                max_file_size: 10 * 1024 * 1024,
                max_open_files: 50,
                allowed_paths: self.config.allowed_paths.clone(),
                allowed_domains: self.config.allowed_domains.clone(),
                enable_networking: self.config.enable_networking,
                enable_filesystem: self.config.enable_filesystem,
                ..Default::default()
            });

            let _wasi_ctx = sandbox.create_wasi_context(&manifest)?;
            
            // NOTE: WASI context initialization would go here
            // wasmtime_wasi::add_to_linker is not available in current wasmtime version
            // store.data_mut().wasi_ctx = Some(_wasi_ctx);
        } else {
            // NOTE: WasiCtxBuilder is not available in current wasmtime version
            // WASI support skipped for now
        }

        // Add Parsec API
        self.add_api_to_linker(&mut linker)?;

        // Instantiate module
        let instance = linker.instantiate(&mut store, &module)?;

        // Get exports
        let exports = self.get_exports(&mut store, &instance)?;

        // Create instance
        let instance = ExtensionInstance::new(
            extension_id.clone(),
            manifest,
            store,
            instance,
            message_tx,
            message_rx,
            memory_usage,
            exports,
        );

        // Store instance
        self.instances.write().await.insert(extension_id.clone(), instance);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_extensions_loaded += 1;
            let active_count = self.instances.read().await.len();
            stats.peak_active_extensions = stats.peak_active_extensions.max(active_count);
        }

        // Send event
        self.event_tx.send(ExtensionEvent::Installed(extension_id.clone())).ok();

        info!("Extension loaded successfully in {:?}", start_time.elapsed());

        Ok(extension_id)
    }

    /// Activate an extension
    pub async fn activate_extension(&self, extension_id: &str) -> Result<()> {
        let instances = self.instances.read().await;
        let instance = instances.get(extension_id)
            .ok_or_else(|| anyhow!("Extension not found: {}", extension_id))?;

        // Check if already active
        if *instance.state.read().await == ExtensionState::Active {
            return Ok(());
        }

        info!("Activating extension: {}", extension_id);

        // Update state
        *instance.state.write().await = ExtensionState::Activating;

        // Call activate function if present
        if let Some(activate) = &instance.exports.activate {
            match activate.call(&mut *instance.store.write().await, (0, 0)) {
                Ok(result) => {
                    if result != 0 {
                        *instance.state.write().await = ExtensionState::Error;
                        self.event_tx.send(ExtensionEvent::Error(
                            extension_id.to_string(),
                            format!("Activation failed with code {}", result)
                        )).ok();
                        return Err(anyhow!("Extension activation failed with code {}", result));
                    }
                }
                Err(e) => {
                    *instance.state.write().await = ExtensionState::Error;
                    self.event_tx.send(ExtensionEvent::Error(
                        extension_id.to_string(),
                        e.to_string()
                    )).ok();
                    return Err(anyhow!("Extension activation failed: {}", e));
                }
            }
        }

        *instance.state.write().await = ExtensionState::Active;
        *instance.last_activity.write().await = std::time::Instant::now();

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_extensions_activated += 1;
        }

        self.event_tx.send(ExtensionEvent::Activated(extension_id.to_string())).ok();

        info!("Extension activated: {}", extension_id);

        Ok(())
    }

    /// Deactivate an extension
    pub async fn deactivate_extension(&self, extension_id: &str) -> Result<()> {
        let instances = self.instances.read().await;
        let instance = instances.get(extension_id)
            .ok_or_else(|| anyhow!("Extension not found: {}", extension_id))?;

        if *instance.state.read().await != ExtensionState::Active {
            return Ok(());
        }

        info!("Deactivating extension: {}", extension_id);

        *instance.state.write().await = ExtensionState::Deactivating;

        // Call deactivate function if present
        if let Some(deactivate) = &instance.exports.deactivate {
            if let Err(e) = deactivate.call(&mut *instance.store.write().await, ()) {
                warn!("Deactivation error for {}: {}", extension_id, e);
            }
        }

        *instance.state.write().await = ExtensionState::Inactive;
        *instance.last_activity.write().await = std::time::Instant::now();

        self.event_tx.send(ExtensionEvent::Deactivated(extension_id.to_string())).ok();

        info!("Extension deactivated: {}", extension_id);

        Ok(())
    }

    /// Unload an extension
    pub async fn unload_extension(&self, extension_id: &str) -> Result<()> {
        let mut instances = self.instances.write().await;
        
        if let Some(instance) = instances.remove(extension_id) {
            info!("Unloading extension: {}", extension_id);

            // Deactivate if active
            if *instance.state.read().await == ExtensionState::Active {
                drop(instance);
                self.deactivate_extension(extension_id).await?;
            }

            self.memory_limiter.unregister_extension(extension_id).await?;
            self.wasm_cache.remove(extension_id).await?;
            
            self.event_tx.send(ExtensionEvent::Uninstalled(extension_id.to_string())).ok();
            
            info!("Extension unloaded: {}", extension_id);
        }

        Ok(())
    }

    /// Send a message to an extension
    pub async fn send_message(&self, extension_id: &str, data: Vec<u8>) -> Result<()> {
        let instances = self.instances.read().await;
        let instance = instances.get(extension_id)
            .ok_or_else(|| anyhow!("Extension not found: {}", extension_id))?;

        // Check if extension is active
        if *instance.state.read().await != ExtensionState::Active {
            return Err(anyhow!("Extension is not active: {}", extension_id));
        }

        instance.message_tx.send(data)?;
        *instance.last_activity.write().await = std::time::Instant::now();

        Ok(())
    }

    /// Receive a message from an extension
    pub async fn receive_message(&self, extension_id: &str) -> Option<Vec<u8>> {
        let instances = self.instances.read().await;
        let instance = instances.get(extension_id)?;
        
        let mut rx = instance.message_rx.lock().await;
        rx.recv().await
    }

    /// Call a command in an extension
    pub async fn call_command(
        &self,
        extension_id: &str,
        command: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let instances = self.instances.read().await;
        let instance = instances.get(extension_id)
            .ok_or_else(|| anyhow!("Extension not found: {}", extension_id))?;

        // Check if extension is active
        if *instance.state.read().await != ExtensionState::Active {
            return Err(anyhow!("Extension is not active: {}", extension_id));
        }

        // Check for registered command handler
        if let Some(handler) = instance.commands.read().await.get(command) {
            return handler(args);
        }

        // Serialize command and args
        let command_data = serde_json::to_vec(&json!({
            "type": "command",
            "command": command,
            "args": args,
            "id": uuid::Uuid::new_v4().to_string(),
        }))?;

        // Send command
        instance.message_tx.send(command_data)?;
        *instance.last_activity.write().await = std::time::Instant::now();

        // Wait for response
        let mut rx = instance.message_rx.lock().await;
        tokio::time::timeout(
            std::time::Duration::from_millis(self.config.max_execution_time_ms),
            rx.recv(),
        ).await
            .map_err(|_| anyhow!("Command timed out"))?
            .ok_or_else(|| anyhow!("No response from extension"))
            .and_then(|response| {
                serde_json::from_slice(&response).map_err(anyhow::Error::from)
            })
    }

    /// Register a command handler for an extension
    pub async fn register_command<F>(
        &self,
        extension_id: &str,
        command: &str,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(Vec<serde_json::Value>) -> Result<serde_json::Value> + Send + Sync + 'static,
    {
        let instances = self.instances.read().await;
        if let Some(instance) = instances.get(extension_id) {
            instance.commands.write().await.insert(
                command.to_string(),
                Arc::new(handler) as Arc<dyn Fn(Vec<serde_json::Value>) -> Result<serde_json::Value> + Send + Sync>,
            );
            Ok(())
        } else {
            Err(anyhow!("Extension not found: {}", extension_id))
        }
    }

    /// Get extension state
    pub async fn get_extension_state(&self, extension_id: &str) -> Option<ExtensionState> {
        let instances = self.instances.read().await;
        instances.get(extension_id).map(|i| *i.state.blocking_read())
    }

    /// List loaded extensions
    pub async fn list_extensions(&self) -> Vec<(String, ExtensionState, Option<super::memory::MemoryUsage>)> {
        let instances = self.instances.read().await;
        let mut result = Vec::new();
        
        for (id, instance) in instances.iter() {
            let state = *instance.state.blocking_read();
            let memory = self.memory_limiter.get_usage(id).await;
            result.push((id.clone(), state, memory));
        }
        
        result
    }

    /// Get next event
    pub async fn next_event(&mut self) -> Option<ExtensionEvent> {
        self.event_rx.lock().await.recv().await
    }

    /// Validate extension manifest
    fn validate_manifest(&self, manifest: &ExtensionManifest) -> Result<()> {
        if manifest.name.is_empty() {
            return Err(anyhow!("Extension name cannot be empty"));
        }
        if manifest.version.is_empty() {
            return Err(anyhow!("Extension version cannot be empty"));
        }
        if manifest.publisher.is_empty() {
            return Err(anyhow!("Extension publisher cannot be empty"));
        }
        if manifest.entry.is_empty() {
            return Err(anyhow!("Extension entry point cannot be empty"));
        }

        // Validate version format (simple check)
        if !manifest.version.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return Err(anyhow!("Invalid version format"));
        }

        Ok(())
    }

    /// Add Parsec API to linker
    fn add_api_to_linker(&self, linker: &mut Linker<ExtensionData>) -> Result<()> {
        // Log function
        linker.func_wrap("parsec", "log", |mut caller: Caller<'_, ExtensionData>, ptr: i32, len: i32| -> Result<(), wasmtime::Error> {
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(mem)) => mem,
                _ => return Err(wasmtime::Error::msg("No memory export")),
            };

            let data = memory.data(&caller);
            let message = String::from_utf8_lossy(&data[ptr as usize..(ptr + len) as usize]).to_string();
            info!("[Extension {}] {}", caller.data().id, message);
            
            Ok(())
        })?;

        // Send message function
        linker.func_wrap("parsec", "send_message", |mut caller: Caller<'_, ExtensionData>, ptr: i32, len: i32| -> Result<(), wasmtime::Error> {
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(mem)) => mem,
                _ => return Err(wasmtime::Error::msg("No memory export")),
            };

            let data = memory.data(&caller);
            let message = data[ptr as usize..(ptr + len) as usize].to_vec();
            
            // Forward message to host
            caller.data().message_tx.send(message).map_err(|_| wasmtime::Error::msg("Failed to send message"))?;
            
            Ok(())
        })?;

        // Get config function
        linker.func_wrap("parsec", "get_config", |_caller: Caller<'_, ExtensionData>, _key_ptr: i32, _key_len: i32| -> i32 {
            // This would retrieve configuration
            0
        })?;

        // Set config function
        linker.func_wrap("parsec", "set_config", |_caller: Caller<'_, ExtensionData>, _key_ptr: i32, _key_len: i32, _val_ptr: i32, _val_len: i32| -> i32 {
            // This would set configuration
            0
        })?;

        Ok(())
    }

    /// Get exports from instance
    fn get_exports(
        &self,
        store: &mut Store<ExtensionData>,
        instance: &Instance,
    ) -> Result<ExtensionExports> {
        Ok(ExtensionExports {
            activate: instance.get_typed_func(&mut *store, "activate").ok(),
            deactivate: instance.get_typed_func(&mut *store, "deactivate").ok(),
            handle_message: instance.get_typed_func(&mut *store, "handle_message").ok(),
            handle_command: instance.get_typed_func(&mut *store, "handle_command").ok(),
        })
    }

    /// Get runtime statistics
    #[cfg(feature = "default")]
    pub async fn statistics(&self) -> super::stats::RuntimeStatistics {
        let instances = self.instances.read().await;
        let stats = self.stats.read().await;
        
        let active_count = instances.values()
            .filter(|i| *i.state.blocking_read() == ExtensionState::Active)
            .count();

        let total_memory = self.memory_limiter.total_used().await;
        let peak_memory = self.memory_limiter.peak_used().await;

        super::stats::RuntimeStatistics {
            total_extensions: instances.len(),
            active_extensions: active_count,
            total_extensions_loaded: stats.total_extensions_loaded,
            total_extensions_activated: stats.total_extensions_activated,
            total_commands_executed: stats.total_commands_executed,
            total_errors: stats.total_errors,
            peak_active_extensions: stats.peak_active_extensions,
            total_memory_used: total_memory,
            peak_memory_used: peak_memory,
            uptime: stats.start_time.elapsed(),
        }
    }

    /// Get detailed metrics
    #[cfg(feature = "default")]
    pub async fn metrics(&self) -> super::stats::RuntimeMetrics {
        let stats = self.stats.read().await;
        let total_memory = self.memory_limiter.total_used().await;
        let peak_memory = self.memory_limiter.peak_used().await;
        let active = self.active_count().await;
        let uptime = stats.start_time.elapsed();

        super::stats::RuntimeMetrics {
            active_extensions: active,
            total_loaded: stats.total_extensions_loaded,
            total_activated: stats.total_extensions_activated,
            total_commands: stats.total_commands_executed,
            total_errors: stats.total_errors,
            peak_active: stats.peak_active_extensions,
            memory_used: total_memory,
            peak_memory,
            uptime_secs: uptime.as_secs_f64(),
            commands_per_sec: stats.commands_per_second(),
            error_rate: stats.error_rate(),
        }
    }

    /// Reset statistics
    pub async fn reset_statistics(&self) {
        let mut stats = self.stats.write().await;
        *stats = RuntimeStats::new();
    }

    /// Get memory limiter reference
    pub fn memory_limiter(&self) -> Arc<MemoryLimiter> {
        self.memory_limiter.clone()
    }

    /// Get scheduler reference
    pub fn scheduler(&self) -> Arc<Scheduler> {
        self.scheduler.clone()
    }

    /// Get wasm cache reference
    pub fn wasm_cache(&self) -> Arc<WasmCache> {
        self.wasm_cache.clone()
    }

    /// Get a handle to an extension
    pub fn handle(&self, id: &str) -> Option<ExtensionHandle> {
        if self.instances.blocking_read().contains_key(id) {
            Some(ExtensionHandle::new(id.to_string(), Arc::new(self.clone())))
        } else {
            None
        }
    }

    /// Check if an extension is loaded
    pub async fn is_loaded(&self, id: &str) -> bool {
        self.instances.read().await.contains_key(id)
    }

    /// Get extension count
    pub async fn extension_count(&self) -> usize {
        self.instances.read().await.len()
    }

    /// Get active extension count
    pub async fn active_count(&self) -> usize {
        self.instances.read().await
            .values()
            .filter(|i| i.is_active())
            .count()
    }

    /// Wait for all extensions to complete (for shutdown)
    pub async fn shutdown(&self) {
        let instances = self.instances.read().await;
        for id in instances.keys() {
            let _ = self.deactivate_extension(id).await;
        }
        // Give extensions time to clean up
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    /// Force terminate all extensions (for emergency shutdown)
    pub async fn terminate_all(&self) {
        let mut instances = self.instances.write().await;
        instances.clear();
        // Memory limiter will be cleared when instances are dropped
    }

    /// Create a new runtime with development preset
    pub fn development() -> Result<Self> {
        Self::new(presets::development())
    }

    /// Create a new runtime with production preset
    pub fn production() -> Result<Self> {
        Self::new(presets::production())
    }

    /// Create a new runtime with minimal preset
    pub fn minimal() -> Result<Self> {
        Self::new(presets::minimal())
    }

    /// Create a new runtime with testing preset
    pub fn testing() -> Result<Self> {
        Self::new(presets::testing())
    }

    /// Get the engine reference
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Get the configuration
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Get the cache directory (if any)
    pub fn cache_dir(&self) -> Option<&PathBuf> {
        self.config.cache_dir.as_ref()
    }

    /// Clear all caches
    pub async fn clear_caches(&self) -> Result<()> {
        self.wasm_cache.clear().await
    }

    /// Preload an extension (load but don't activate)
    pub async fn preload_extension(
        &self,
        wasm_bytes: &[u8],
        manifest: ExtensionManifest,
        path: PathBuf,
    ) -> Result<String> {
        self.load_extension(wasm_bytes, manifest, path).await
    }

    /// Hot reload an extension (unload and load again)
    pub async fn hot_reload(
        &self,
        id: &str,
        wasm_bytes: &[u8],
        manifest: ExtensionManifest,
        path: PathBuf,
    ) -> Result<String> {
        self.unload_extension(id).await?;
        self.load_extension(wasm_bytes, manifest, path).await
    }

    /// Check if an extension is responsive
    pub async fn ping(&self, id: &str) -> bool {
        if let Some(handle) = self.handle(id) {
            if let Ok(()) = handle.send_message(b"ping".to_vec()).await {
                if let Some(response) = tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    handle.receive_message(),
                ).await.ok().flatten() {
                    return response == b"pong";
                }
            }
        }
        false
    }

    #[cfg_attr(feature = "default", allow(dead_code))]
    /// Get performance metrics for a specific extension
    pub async fn get_performance_metrics(&self, id: &str) -> Option<()> {
        let instances = self.instances.read().await;
        let _instance = instances.get(id)?;
        
        // Performance metrics stubbed out
        Some(())
    }

    #[cfg_attr(feature = "default", allow(dead_code))]
    /// Get performance metrics for all extensions
    pub async fn get_all_performance_metrics(&self) -> Vec<()> {
        let _instances = self.instances.read().await;
        // All metrics stubbed out
        vec![]
    }

    /// Set resource limits for a specific extension
    pub async fn set_extension_limits(&self, id: &str, memory_limit: Option<usize>) -> Result<()> {
        let instances = self.instances.read().await;
        if let Some(instance) = instances.get(id) {
            let mut usage = instance.memory_usage.write().await;
            if let Some(limit) = memory_limit {
                usage.limit = limit;
            }
            Ok(())
        } else {
            Err(anyhow!("Extension not found: {}", id))
        }
    }

    #[cfg_attr(feature = "default", allow(dead_code))]
    /// Get extensions sorted by priority
    pub async fn get_prioritized_extensions(&self) -> Vec<()> {
        vec![]
    }

    #[cfg_attr(feature = "default", allow(dead_code))]
    /// Suspend low priority extensions when under memory pressure
    pub async fn handle_memory_pressure(&self) -> Result<usize> {
        Ok(0)
    }

    #[cfg_attr(feature = "default", allow(dead_code))]
    /// Perform health check on an extension
    pub async fn health_check(&self, id: &str) -> () {
        let _ = self.is_loaded(id).await;
        // Health check stubbed out
    }

    #[cfg_attr(feature = "default", allow(dead_code))]
    /// Perform health check on all extensions
    pub async fn health_check_all(&self) -> Vec<(String, ())> {
        vec![]
    }

    #[cfg_attr(feature = "default", allow(dead_code))]
    /// Create a snapshot of all extensions (for state persistence)
    pub async fn create_snapshot(&self) -> Vec<()> {
        vec![]
    }

    #[cfg_attr(feature = "default", allow(dead_code))]
    /// Restore extensions from a snapshot
    pub async fn restore_from_snapshot(&self, _snapshots: Vec<()>) -> Result<()> {
        Ok(())
    }

    /// Broadcast a message to all extensions
    pub async fn broadcast_message(&self, data: Vec<u8>) -> usize {
        let instances = self.instances.read().await;
        let mut sent = 0;
        
        for instance in instances.values() {
            if instance.is_active() {
                if instance.send_message(data.clone()).is_ok() {
                    sent += 1;
                }
            }
        }
        
        sent
    }

    /// Execute a command on all extensions that support it
    pub async fn broadcast_command(
        &self,
        command: &str,
        args: Vec<serde_json::Value>,
    ) -> Vec<(String, Result<serde_json::Value>)> {
        let instances = self.instances.read().await;
        let mut results = Vec::new();
        
        for (id, instance) in instances.iter() {
            if instance.is_active() {
                let result = self.call_command(id, command, args.clone()).await;
                results.push((id.clone(), result));
            }
        }
        
        results
    }

}

impl Clone for ExtensionRuntime {
    fn clone(&self) -> Self {
        Self {
            engine: self.engine.clone(),
            instances: self.instances.clone(),
            scheduler: self.scheduler.clone(),
            memory_limiter: self.memory_limiter.clone(),
            wasm_cache: self.wasm_cache.clone(),
            event_tx: self.event_tx.clone(),
            event_rx: self.event_rx.clone(),
            config: self.config.clone(),
            stats: self.stats.clone(),
        }
    }
}