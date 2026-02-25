//! Runtime builder for configuration

use std::path::PathBuf;
use anyhow::Result;
use crate::runtime::{ExtensionRuntime, RuntimeConfig};

/// Runtime builder for configuration
pub struct RuntimeBuilder {
    config: RuntimeConfig,
}

impl RuntimeBuilder {
    /// Create a new runtime builder
    pub fn new() -> Self {
        Self {
            config: RuntimeConfig::default(),
        }
    }

    /// Set maximum memory per extension
    pub fn max_memory_per_extension(mut self, bytes: usize) -> Self {
        self.config.max_memory_per_extension = bytes;
        self
    }

    /// Set maximum total memory
    pub fn max_total_memory(mut self, bytes: usize) -> Self {
        self.config.max_total_memory = bytes;
        self
    }

    /// Set maximum execution time
    pub fn max_execution_time(mut self, ms: u64) -> Self {
        self.config.max_execution_time_ms = ms;
        self
    }

    /// Enable sandboxing
    pub fn enable_sandbox(mut self, enable: bool) -> Self {
        self.config.enable_sandbox = enable;
        self
    }

    /// Enable networking
    pub fn enable_networking(mut self, enable: bool) -> Self {
        self.config.enable_networking = enable;
        self
    }

    /// Enable filesystem access
    pub fn enable_filesystem(mut self, enable: bool) -> Self {
        self.config.enable_filesystem = enable;
        self
    }

    /// Add allowed path
    pub fn add_allowed_path(mut self, path: PathBuf) -> Self {
        self.config.allowed_paths.push(path);
        self
    }

    /// Add allowed domain
    pub fn add_allowed_domain(mut self, domain: String) -> Self {
        self.config.allowed_domains.push(domain);
        self
    }

    /// Set cache directory
    pub fn cache_dir(mut self, dir: PathBuf) -> Self {
        self.config.cache_dir = Some(dir);
        self
    }

    /// Enable module validation
    pub fn validate_modules(mut self, enable: bool) -> Self {
        self.config.validate_modules = enable;
        self
    }

    /// Enable security checks
    pub fn security_checks(mut self, enable: bool) -> Self {
        self.config.security_checks = enable;
        self
    }

    /// Set max concurrent extensions
    pub fn max_concurrent_extensions(mut self, max: usize) -> Self {
        self.config.max_concurrent_extensions = max;
        self
    }

    /// Enable compiled module caching
    pub fn cache_compiled(mut self, enable: bool) -> Self {
        self.config.cache_compiled = enable;
        self
    }

    /// Build the runtime
    pub fn build(self) -> Result<ExtensionRuntime> {
        ExtensionRuntime::new(self.config)
    }

    /// Build with default configuration
    pub fn build_default() -> Result<ExtensionRuntime> {
        Self::default().build()
    }

    /// Build with custom configuration
    pub fn build_with_config(config: RuntimeConfig) -> Result<ExtensionRuntime> {
        ExtensionRuntime::new(config)
    }
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}