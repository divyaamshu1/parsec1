//! Runtime configuration presets

use std::path::PathBuf;
use crate::runtime::RuntimeConfig;

/// Development preset - permissive for testing
pub fn development() -> RuntimeConfig {
    RuntimeConfig {
        max_memory_per_extension: 100 * 1024 * 1024, // 100MB
        max_total_memory: 1024 * 1024 * 1024,       // 1GB
        max_execution_time_ms: 30000,                // 30 seconds
        enable_sandbox: false,
        enable_networking: true,
        enable_filesystem: true,
        allowed_paths: vec![
            std::env::current_dir().unwrap_or_default(),
            std::env::temp_dir(),
        ],
        allowed_domains: vec!["*".to_string()],
        max_concurrent_extensions: 50,
        cache_compiled: true,
        cache_dir: dirs::cache_dir().map(|d| d.join("parsec-extensions-dev")),
        validate_modules: true,
        security_checks: false,
    }
}

/// Production preset - secure and restricted
pub fn production() -> RuntimeConfig {
    RuntimeConfig {
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

/// Minimal preset - smallest footprint
pub fn minimal() -> RuntimeConfig {
    RuntimeConfig {
        max_memory_per_extension: 10 * 1024 * 1024,  // 10MB
        max_total_memory: 100 * 1024 * 1024,        // 100MB
        max_execution_time_ms: 1000,                 // 1 second
        enable_sandbox: true,
        enable_networking: false,
        enable_filesystem: false,
        allowed_paths: vec![],
        allowed_domains: vec![],
        max_concurrent_extensions: 5,
        cache_compiled: false,
        cache_dir: None,
        validate_modules: true,
        security_checks: true,
    }
}

/// Testing preset - for CI/CD
pub fn testing() -> RuntimeConfig {
    RuntimeConfig {
        max_memory_per_extension: 20 * 1024 * 1024,  // 20MB
        max_total_memory: 200 * 1024 * 1024,        // 200MB
        max_execution_time_ms: 2000,                 // 2 seconds
        enable_sandbox: true,
        enable_networking: false,
        enable_filesystem: false,
        allowed_paths: vec![std::env::temp_dir()],
        allowed_domains: vec!["localhost".to_string()],
        max_concurrent_extensions: 10,
        cache_compiled: false,
        cache_dir: None,
        validate_modules: true,
        security_checks: true,
    }
}