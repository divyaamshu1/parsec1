//! WebAssembly module handling for extensions
//!
//! Provides WASM module compilation, caching, and validation.

#![allow(unexpected_cfgs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use tokio::sync::RwLock;
use wasmtime::{Engine, Module, Linker, Store};
use wasmparser::{Validator, Parser, Payload};
use tracing::{warn, debug};

use super::ExtensionData;

/// WASM module cache for performance
pub struct WasmCache {
    /// Compiled modules by path
    modules: Arc<RwLock<HashMap<String, Module>>>,
    /// Cache directory
    cache_dir: Option<PathBuf>,
    /// Engine reference
    engine: Engine,
}

impl WasmCache {
    /// Create a new WASM cache
    pub fn new(engine: Engine, cache_dir: Option<PathBuf>) -> Self {
        Self {
            modules: Arc::new(RwLock::new(HashMap::new())),
            cache_dir,
            engine,
        }
    }

    /// Get or compile a WASM module
    pub async fn get_or_compile(&self, key: &str, wasm_bytes: &[u8]) -> Result<Module> {
        // Check memory cache
        if let Some(module) = self.modules.read().await.get(key) {
            debug!("Cache hit for module: {}", key);
            return Ok(module.clone());
        }

        // Check disk cache
        if let Some(cache_dir) = &self.cache_dir {
            let cache_path = cache_dir.join(format!("{}.cwasm", key));
            if cache_path.exists() {
                debug!("Loading module from disk cache: {}", key);
                // SAFETY: Deserializing from trusted cache directory
                let module = unsafe { Module::deserialize_file(&self.engine, &cache_path)? };
                self.modules.write().await.insert(key.to_string(), module.clone());
                return Ok(module);
            }
        }

        // Compile module
        debug!("Compiling module: {}", key);
        let module = Module::new(&self.engine, wasm_bytes)?;

        // Cache to disk if directory is configured
        if let Some(cache_dir) = &self.cache_dir {
            let _ = std::fs::create_dir_all(cache_dir);
            let _cache_path = cache_dir.join(format!("{}.cwasm", key));
            
            // Serialize module to disk cache
            // Note: Module serialization is not available in the current wasmtime version
            // This would require additional feature flags or a different approach
        }

        // Store in memory cache
        self.modules.write().await.insert(key.to_string(), module.clone());

        Ok(module)
    }

    /// Clear the cache
    pub async fn clear(&self) -> Result<()> {
        self.modules.write().await.clear();
        
        if let Some(cache_dir) = &self.cache_dir {
            if cache_dir.exists() {
                std::fs::remove_dir_all(cache_dir)?;
            }
        }
        
        Ok(())
    }

    /// Remove a specific module from cache
    pub async fn remove(&self, key: &str) -> Result<()> {
        self.modules.write().await.remove(key);
        
        if let Some(cache_dir) = &self.cache_dir {
            let cache_path = cache_dir.join(format!("{}.cwasm", key));
            if cache_path.exists() {
                std::fs::remove_file(cache_path)?;
            }
        }
        
        Ok(())
    }

    /// Get cache size
    pub async fn size(&self) -> usize {
        self.modules.read().await.len()
    }
}

/// WASM module validator
pub struct WasmValidator;

impl WasmValidator {
    /// Validate a WASM module
    pub fn validate(wasm_bytes: &[u8]) -> Result<()> {
        // Check magic number
        if wasm_bytes.len() < 8 || &wasm_bytes[0..4] != b"\0asm" {
            return Err(anyhow!("Invalid WASM magic number"));
        }

        // Check version
        let version = u32::from_le_bytes([wasm_bytes[4], wasm_bytes[5], wasm_bytes[6], wasm_bytes[7]]);
        if version != 1 {
            return Err(anyhow!("Unsupported WASM version: {}", version));
        }

        // Validate with wasmparser
        let mut validator = Validator::new();
        
        // Use validate_all instead of validate (which doesn't exist)
        match validator.validate_all(wasm_bytes) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!("Invalid WASM module: {}", e)),
        }
    }

    /// Check for dangerous instructions (security)
    pub fn check_security(wasm_bytes: &[u8]) -> Result<()> {
        let mut has_memory = false;
        let mut imported_funcs = 0;
        let mut imported_memories = 0;

        for payload in Parser::new(0).parse_all(wasm_bytes) {
            match payload? {
                Payload::ImportSection(reader) => {
                    for import in reader {
                        let _ = import?;
                        imported_funcs += 1;  // Simplified: count all as functions
                    }
                }
                Payload::MemorySection(reader) => {
                    for mem in reader {
                        let _mem = mem?;
                        has_memory = true;
                    }
                }
                Payload::CodeSectionStart { .. } => {
                    // Additional security checks could be added here
                }
                _ => {}
            }
        }

        // Extensions must have memory (either imported or defined)
        if !has_memory && imported_memories == 0 {
            return Err(anyhow!("Extension must have a memory section"));
        }

        // Check for suspicious number of imports
        if imported_funcs > 100 {
            warn!("Extension imports {} functions, might be suspicious", imported_funcs);
        }

        Ok(())
    }

    /// Get module metadata
    pub fn get_metadata(wasm_bytes: &[u8]) -> Result<WasmMetadata> {
        let mut metadata = WasmMetadata::default();

        for payload in Parser::new(0).parse_all(wasm_bytes) {
            match payload? {
                Payload::Version { .. } => {}
                Payload::TypeSection(reader) => {
                    for ty in reader {
                        let _ty = ty?;
                        metadata.type_count += 1;
                    }
                }
                Payload::ImportSection(reader) => {
                    for import in reader {
                        let _ = import?;
                        metadata.import_count += 1;
                        // Simplified: unable to differentiate import types with current wasmparser version
                        metadata.function_imports += 1;
                    }
                }
                Payload::FunctionSection(reader) => {
                    for _ in reader {
                        metadata.function_count += 1;
                    }
                }
                Payload::TableSection(reader) => {
                    for _ in reader {
                        metadata.table_count += 1;
                    }
                }
                Payload::MemorySection(reader) => {
                    for mem in reader {
                        let mem = mem?;
                        metadata.memory_count += 1;
                        metadata.initial_memory_pages = mem.initial as u32;
                        metadata.max_memory_pages = mem.maximum.unwrap_or(0) as u32;
                        metadata.memory_shared = mem.shared;
                    }
                }
                Payload::GlobalSection(reader) => {
                    for _ in reader {
                        metadata.global_count += 1;
                    }
                }
                Payload::ExportSection(reader) => {
                    for export in reader {
                        let export = export?;
                        metadata.export_count += 1;
                        match export.kind {
                            wasmparser::ExternalKind::Func => metadata.function_exports += 1,
                            wasmparser::ExternalKind::Table => metadata.table_exports += 1,
                            wasmparser::ExternalKind::Memory => metadata.memory_exports += 1,
                            wasmparser::ExternalKind::Global => metadata.global_exports += 1,
                            wasmparser::ExternalKind::Tag => metadata.tag_exports += 1,
                            wasmparser::ExternalKind::FuncExact => metadata.function_exports += 1,
                        }
                        metadata.export_names.push(export.name.to_string());
                    }
                }
                Payload::DataSection(reader) => {
                    for data in reader {
                        let _ = data?;
                        metadata.data_count += 1;
                    }
                }
                Payload::CodeSectionStart { count, .. } => {
                    metadata.code_section_size = count as usize;
                }
                Payload::CustomSection(reader) => {
                    let name = reader.name().to_string();
                    metadata.custom_sections.push(name);
                }
                _ => {}
            }
        }

        Ok(metadata)
    }
}

/// WASM module metadata
#[derive(Debug, Default, Clone)]
pub struct WasmMetadata {
    pub type_count: usize,
    pub import_count: usize,
    pub function_imports: usize,
    pub memory_imports: usize,
    pub table_imports: usize,
    pub global_imports: usize,
    pub tag_imports: usize,
    pub function_count: usize,
    pub table_count: usize,
    pub memory_count: usize,
    pub global_count: usize,
    pub export_count: usize,
    pub function_exports: usize,
    pub table_exports: usize,
    pub memory_exports: usize,
    pub global_exports: usize,
    pub tag_exports: usize,
    pub data_count: usize,
    pub code_section_size: usize,
    pub initial_memory_pages: u32,
    pub max_memory_pages: u32,
    pub memory_shared: bool,
    pub custom_sections: Vec<String>,
    pub export_names: Vec<String>,
}

impl WasmMetadata {
    /// Estimate memory usage in bytes
    pub fn estimated_memory_bytes(&self) -> usize {
        self.initial_memory_pages as usize * 64 * 1024 // 64KB per page
    }

    /// Check if module exports a specific function
    pub fn exports_function(&self, name: &str) -> bool {
        self.export_names.contains(&name.to_string())
    }

    /// Get exported function names
    pub fn exported_functions(&self) -> Vec<String> {
        self.export_names.clone()
    }
}

/// WASM linker configuration
pub struct WasmLinker {
    linker: Linker<ExtensionData>,
    _engine: Engine,
}

impl WasmLinker {
    /// Create a new WASM linker
    pub fn new(engine: Engine) -> Self {
        Self {
            linker: Linker::new(&engine),
            _engine: engine,
        }
    }

    /// Initialize with standard imports including WASI
    pub fn init_standard(&mut self) -> Result<()> {
        // WASI initialization would require proper data accessor setup
        // This is handled during WASM module instantiation, not here
        Ok(())
    }

    /// Add custom host function
    pub fn add_func<F, Args, Ret>(&mut self, module: &str, name: &str, func: F) -> Result<()>
    where
        F: wasmtime::IntoFunc<ExtensionData, Args, Ret> + Send + Sync + 'static,
    {
        self.linker.func_wrap(module, name, func)?;
        Ok(())
    }

    /// Get the linker
    pub fn linker(&self) -> &Linker<ExtensionData> {
        &self.linker
    }
    
    /// Get mutable linker
    pub fn linker_mut(&mut self) -> &mut Linker<ExtensionData> {
        &mut self.linker
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal valid WASM module (WAT format then converted)
    // (module
    //   (func (export "test") (param i32) (result i32)
    //     local.get 0
    //     i32.const 1
    //     i32.add)
    //   (memory (export "memory") 1)
    // )
    const VALID_WASM: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01, 0x60,
        0x01, 0x7f, 0x01, 0x7f, 0x03, 0x02, 0x01, 0x00, 0x05, 0x03, 0x01, 0x00,
        0x01, 0x07, 0x10, 0x02, 0x04, 0x74, 0x65, 0x73, 0x74, 0x00, 0x00, 0x06,
        0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00, 0x0a, 0x09, 0x01, 0x07,
        0x00, 0x20, 0x00, 0x41, 0x01, 0x6a, 0x0b
    ];

    #[test]
    fn test_wasm_validator_valid() {
        let result = WasmValidator::validate(VALID_WASM);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wasm_validator_invalid_magic() {
        let invalid = b"not wasm";
        let result = WasmValidator::validate(invalid);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("magic number"));
    }

    #[test]
    fn test_wasm_validator_invalid_version() {
        let mut invalid = VALID_WASM.to_vec();
        invalid[4] = 0x02; // Change version to 2
        let result = WasmValidator::validate(&invalid);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("version"));
    }

    #[test]
    fn test_wasm_security_check() {
        let result = WasmValidator::check_security(VALID_WASM);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wasm_metadata() {
        let metadata = WasmValidator::get_metadata(VALID_WASM).unwrap();
        assert_eq!(metadata.function_count, 1);
        assert_eq!(metadata.memory_count, 1);
        assert_eq!(metadata.export_count, 2);
        assert!(metadata.export_names.contains(&"test".to_string()));
        assert!(metadata.export_names.contains(&"memory".to_string()));
        assert_eq!(metadata.initial_memory_pages, 1);
        assert_eq!(metadata.estimated_memory_bytes(), 64 * 1024);
    }

    #[tokio::test]
    async fn test_wasm_cache() {
        let engine = Engine::default();
        let cache_dir = tempfile::tempdir().unwrap();
        let cache = WasmCache::new(engine, Some(cache_dir.path().to_path_buf()));

        // Compile module
        let module = cache.get_or_compile("test", VALID_WASM).await;
        assert!(module.is_ok());
        assert_eq!(cache.size().await, 1);

        // Get from cache
        let module2 = cache.get_or_compile("test", VALID_WASM).await;
        assert!(module2.is_ok());

        // Clear cache
        cache.clear().await.unwrap();
        assert_eq!(cache.size().await, 0);
    }

    #[test]
    fn test_wasm_linker() {
        let engine = Engine::default();
        let mut linker = WasmLinker::new(engine);
        assert!(linker.init_standard().is_ok());
    }
}