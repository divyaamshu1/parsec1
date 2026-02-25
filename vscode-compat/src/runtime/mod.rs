//! JavaScript and Node.js runtimes for VS Code extensions
//!
//! Provides runtimes for executing VS Code extensions written in
//! pure JavaScript or requiring Node.js APIs.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use tokio::sync::{Mutex, RwLock};
use tokio::process::Command;

use crate::api::VSCodeAPI;
use crate::LoadedExtension;

#[cfg(feature = "js-runtime")]
mod js_runtime;
#[cfg(feature = "js-runtime")]
pub use js_runtime::JSRuntime;

mod node_runtime;
pub use node_runtime::NodeRuntime;

/// Cancellation token for async operations
#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<tokio::sync::watch::Sender<bool>>,
    receiver: Arc<tokio::sync::watch::Receiver<bool>>,
}

impl CancellationToken {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::watch::channel(false);
        Self {
            cancelled: Arc::new(tx),
            receiver: Arc::new(rx),
        }
    }

    pub fn cancel(&self) {
        let _ = self.cancelled.send(true);
    }

    pub fn is_cancelled(&self) -> bool {
        *self.receiver.borrow()
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension context passed to activation function
pub struct ExtensionContext {
    /// Extension ID
    pub id: String,
    /// VS Code API
    pub api: Arc<VSCodeAPI>,
    /// Workspace state
    pub workspace_state: serde_json::Value,
    /// Global state
    pub global_state: serde_json::Value,
    /// Subscriptions for cleanup
    subscriptions: Arc<Mutex<Vec<Box<dyn crate::api::Disposable>>>>,
    /// Cancellation token
    pub cancellation_token: CancellationToken,
}

impl ExtensionContext {
    pub fn new(id: String, api: Arc<VSCodeAPI>) -> Self {
        Self {
            id,
            api,
            workspace_state: serde_json::Value::Null,
            global_state: serde_json::Value::Null,
            subscriptions: Arc::new(Mutex::new(Vec::new())),
            cancellation_token: CancellationToken::new(),
        }
    }

    /// Subscribe a disposable for cleanup
    pub async fn subscribe(&self, disposable: impl crate::api::Disposable + 'static) {
        self.subscriptions.lock().await.push(Box::new(disposable));
    }

    /// Dispose all subscriptions
    pub async fn dispose(&self) {
        let mut subs = self.subscriptions.lock().await;
        for sub in subs.drain(..) {
            sub.dispose();
        }
    }
}

/// JavaScript value (simplified)
#[derive(Debug, Clone)]
pub enum JSValue {
    Null,
    Undefined,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JSValue>),
    Object(HashMap<String, JSValue>),
    Function(String), // Function body as string
}

impl From<serde_json::Value> for JSValue {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => JSValue::Null,
            serde_json::Value::Bool(b) => JSValue::Bool(b),
            serde_json::Value::Number(n) => JSValue::Number(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::String(s) => JSValue::String(s),
            serde_json::Value::Array(a) => JSValue::Array(a.into_iter().map(JSValue::from).collect()),
            serde_json::Value::Object(o) => {
                let mut map = HashMap::new();
                for (k, v) in o {
                    map.insert(k, JSValue::from(v));
                }
                JSValue::Object(map)
            }
        }
    }
}

/// Extension exports (activate/deactivate functions)
#[derive(Debug, Clone)]
pub struct ExtensionExports {
    pub activate: Option<String>,
    pub deactivate: Option<String>,
}

/// Common runtime functionality
pub trait Runtime {
    /// Execute an extension
    fn execute_extension(&self, extension: &LoadedExtension) -> Result<()>;

    /// Call a function in the runtime
    fn call_function(&self, name: &str, args: Vec<JSValue>) -> Result<JSValue>;

    /// Get a value from the runtime
    fn get_value(&self, name: &str) -> Result<JSValue>;

    /// Set a value in the runtime
    fn set_value(&self, name: &str, value: JSValue) -> Result<()>;
}

/// Helper to find Node.js executable
pub fn find_node() -> Option<String> {
    // Check common locations
    let paths = if cfg!(windows) {
        vec![
            "C:\\Program Files\\nodejs\\node.exe",
            "C:\\Program Files (x86)\\nodejs\\node.exe",
        ]
    } else {
        vec![
            "/usr/bin/node",
            "/usr/local/bin/node",
            "/opt/homebrew/bin/node",
        ]
    };

    for path in paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    // Try which command
    if let Ok(path) = which::which("node") {
        return Some(path.to_string_lossy().to_string());
    }

    None
}

/// Check if Node.js is available
pub fn is_node_available() -> bool {
    find_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancellation_token() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
    }
}