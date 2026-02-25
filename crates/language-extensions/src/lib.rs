//! Language Extensions for Parsec
//!
//! This crate provides WASM-based language extensions that can be installed
//! from the Parsec extension registry. Each language extension contains:
//!
//! - Tree-sitter grammar (compiled to WASM)
//! - Highlight queries
//! - Indentation rules
//! - Comment syntax

mod types;
mod loader;
mod registry;
mod wasm;

pub use types::*;
pub use loader::LanguageExtensionLoader;
pub use registry::LanguageRegistry;

/// Re-export tree-sitter for convenience
pub use tree_sitter;

/// Version constant
pub const VERSION: &str = env!("CARGO_PKG_VERSION");