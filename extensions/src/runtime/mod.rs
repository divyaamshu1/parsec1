//! WebAssembly runtime for extensions
//!
//! Provides a secure, sandboxed runtime for executing WASM extensions.

mod core;
mod instance;
mod data;
mod exports;
mod limiter;
mod builder;
mod handle;
mod stats;
mod events;
mod commands;
mod presets;
mod memory;
mod sandbox;
mod scheduler;
mod wasm;

// Re-exports
pub use core::ExtensionRuntime;
pub use core::RuntimeConfig;
pub use instance::ExtensionInstance;
pub use data::ExtensionData;
pub use exports::ExtensionExports;
pub use limiter::ResourceLimiter;
pub use builder::RuntimeBuilder;
pub use handle::ExtensionHandle;
pub use stats::{RuntimeStatistics, RuntimeMetrics};
pub use events::EventListener;
pub use commands::CommandHandler;
pub use memory::MemoryUsage;
pub use wasmtime::Memory as WasmMemory;

// Re-export from parent
pub use crate::{ExtensionState, ExtensionManifest};

// Version constant
pub const VERSION: &str = env!("CARGO_PKG_VERSION");