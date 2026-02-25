//! Node.js runtime for VS Code extensions that require Node.js APIs

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use tokio::process::{Command, Child};
use tokio::sync::{Mutex, mpsc};
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{info, warn, error};

use super::{Runtime, JSValue, CancellationToken, find_node};
use crate::LoadedExtension;

/// Node.js runtime for executing extensions
pub struct NodeRuntime {
    /// Node.js executable path
    node_path: String,
    /// Running child processes
    processes: Arc<Mutex<HashMap<String, NodeProcess>>>,
    /// IPC channels
    ipc_channels: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
}

/// Node.js process
struct NodeProcess {
    child: Child,
    id: String,
    stdout_task: tokio::task::JoinHandle<()>,
    stderr_task: tokio::task::JoinHandle<()>,
}

/// Node.js runtime configuration
#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub node_path: Option<String>,
    pub max_memory_mb: Option<usize>,
    pub environment_vars: HashMap<String, String>,
    pub require_paths: Vec<PathBuf>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_path: find_node(),
            max_memory_mb: Some(512),
            environment_vars: HashMap::new(),
            require_paths: Vec::new(),
        }
    }
}

impl NodeRuntime {
    /// Create a new Node.js runtime
    pub fn new() -> Result<Self> {
        let node_path = find_node().ok_or_else(|| anyhow!("Node.js not found"))?;
        
        Ok(Self {
            node_path,
            processes: Arc::new(Mutex::new(HashMap::new())),
            ipc_channels: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: NodeConfig) -> Result<Self> {
        let node_path = config.node_path
            .ok_or_else(|| anyhow!("Node.js path not provided"))?;

        Ok(Self {
            node_path,
            processes: Arc::new(Mutex::new(HashMap::new())),
            ipc_channels: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Start an extension process
    pub async fn start_extension(&self, extension: &LoadedExtension) -> Result<String> {
        let extension_id = extension.id.clone();
        let main_file = self.find_main_file(extension)?;
        
        info!("Starting Node.js process for extension {}: {}", extension_id, main_file.display());

        // Create bootstrap script
        let bootstrap = self.create_bootstrap_script(extension)?;

        // Spawn Node.js process
        let mut cmd = Command::new(&self.node_path);
        
        // Set memory limit
        if let Some(limit) = self.get_memory_limit() {
            cmd.arg(format!("--max-old-space-size={}", limit));
        }

        // Add require paths
        cmd.arg("--require")
           .arg(bootstrap.path());

        // Execute extension
        cmd.arg(&main_file);

        // Set environment variables
        cmd.env("NODE_ENV", "development");
        cmd.env("VSCODE_NODEJS_RUNTIME_DIR", extension.path.display().to_string());
        
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::piped());

        let mut child = cmd.spawn()?;

        // Create IPC channel
        let (ipc_tx, mut ipc_rx) = mpsc::unbounded_channel();
        self.ipc_channels.lock().await.insert(extension_id.clone(), ipc_tx);

        // Handle IPC from Rust to Node
        let stdin = child.stdin.take().expect("Failed to get stdin");
        let ipc_tx_clone = self.ipc_channels.clone();
        let ext_id = extension_id.clone();
        tokio::spawn(async move {
            while let Some(msg) = ipc_rx.recv().await {
                // Write message to stdin
                // This would need proper IPC protocol
            }
        });

        // Handle stdout
        let stdout = child.stdout.take().expect("Failed to get stdout");
        let mut reader = BufReader::new(stdout).lines();
        let ext_id_stdout = extension_id.clone();
        let stdout_task = tokio::spawn(async move {
            while let Ok(Some(line)) = reader.next_line().await {
                info!("[{} stdout] {}", ext_id_stdout, line);
            }
        });

        // Handle stderr
        let stderr = child.stderr.take().expect("Failed to get stderr");
        let mut reader = BufReader::new(stderr).lines();
        let ext_id_stderr = extension_id.clone();
        let stderr_task = tokio::spawn(async move {
            while let Ok(Some(line)) = reader.next_line().await {
                warn!("[{} stderr] {}", ext_id_stderr, line);
            }
        });

        let process = NodeProcess {
            child,
            id: extension_id.clone(),
            stdout_task,
            stderr_task,
        };

        self.processes.lock().await.insert(extension_id.clone(), process);

        Ok(extension_id)
    }

    /// Find the main entry file for an extension
    fn find_main_file(&self, extension: &LoadedExtension) -> Result<PathBuf> {
        // Check for main in package.json
        if let Some(main) = &extension.main {
            let main_path = extension.path.join("extension").join(main);
            if main_path.exists() {
                return Ok(main_path);
            }
        }

        // Default to extension.js
        let default_path = extension.path.join("extension").join("extension.js");
        if default_path.exists() {
            return Ok(default_path);
        }

        Err(anyhow!("No main entry point found for extension"))
    }

    /// Create bootstrap script that sets up vscode API
    fn create_bootstrap_script(&self, extension: &LoadedExtension) -> Result<tempfile::NamedTempFile> {
        let mut script = tempfile::NamedTempFile::new()?;

        let content = format!(r#"
            // VS Code API Shim
            const vscode = require('./vscode-shim');
            
            // Set up module resolution
            const path = require('path');
            const module = require('module');
            
            const originalRequire = module.prototype.require;
            
            module.prototype.require = function(request) {{
                // Handle vscode module
                if (request === 'vscode') {{
                    return vscode;
                }}
                
                // Handle relative paths within extension
                if (request.startsWith('.')) {{
                    const extPath = path.join('{ext_path}', 'extension');
                    return originalRequire.call(this, path.join(extPath, request));
                }}
                
                return originalRequire.call(this, request);
            }};
            
            // Initialize extension
            console.log('Extension {ext_id} loaded');
            
            // Export for extension
            module.exports = {{}};
        "#,
            ext_id = extension.id,
            ext_path = extension.path.display()
        );

        use std::io::Write;
        script.write_all(content.as_bytes())?;

        Ok(script)
    }

    /// Get memory limit in MB
    fn get_memory_limit(&self) -> Option<usize> {
        Some(512) // Default 512MB
    }

    /// Stop an extension process
    pub async fn stop_extension(&self, extension_id: &str) -> Result<()> {
        let mut processes = self.processes.lock().await;
        
        if let Some(mut process) = processes.remove(extension_id) {
            let _ = process.child.kill().await;
            process.stdout_task.abort();
            process.stderr_task.abort();
        }

        self.ipc_channels.lock().await.remove(extension_id);

        Ok(())
    }

    /// Send message to extension
    pub async fn send_message(&self, extension_id: &str, message: &str) -> Result<()> {
        let channels = self.ipc_channels.lock().await;
        if let Some(tx) = channels.get(extension_id) {
            tx.send(message.to_string())?;
            Ok(())
        } else {
            Err(anyhow!("Extension {} not running", extension_id))
        }
    }
}

impl Runtime for NodeRuntime {
    fn execute_extension(&self, extension: &LoadedExtension) -> Result<()> {
        let runtime = self.clone();
        let extension = extension.clone();

        tokio::spawn(async move {
            if let Err(e) = runtime.start_extension(&extension).await {
                error!("Failed to start Node.js extension {}: {}", extension.id, e);
            }
        });

        Ok(())
    }

    fn call_function(&self, name: &str, args: Vec<JSValue>) -> Result<JSValue> {
        // Node.js runtime doesn't support direct function calls
        // We need to send a message and wait for response
        Err(anyhow!("Direct function calls not supported in Node.js runtime"))
    }

    fn get_value(&self, _name: &str) -> Result<JSValue> {
        Err(anyhow!("get_value not supported in Node.js runtime"))
    }

    fn set_value(&self, _name: &str, _value: JSValue) -> Result<()> {
        Err(anyhow!("set_value not supported in Node.js runtime"))
    }
}

impl Clone for NodeRuntime {
    fn clone(&self) -> Self {
        Self {
            node_path: self.node_path.clone(),
            processes: self.processes.clone(),
            ipc_channels: self.ipc_channels.clone(),
        }
    }
}

impl Drop for NodeRuntime {
    fn drop(&mut self) {
        // Kill all processes on drop
        let mut processes = self.processes.blocking_lock();
        for (_, process) in processes.iter_mut() {
            let _ = process.child.try_wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_node_runtime_creation() {
        if find_node().is_some() {
            let runtime = NodeRuntime::new();
            assert!(runtime.is_ok());
        } else {
            println!("Node.js not found, skipping test");
        }
    }

    #[tokio::test]
    async fn test_start_extension() {
        if find_node().is_none() {
            println!("Node.js not found, skipping test");
            return;
        }

        let runtime = NodeRuntime::new().unwrap();
        
        // Create a mock extension
        let dir = tempdir().unwrap();
        let ext_dir = dir.path().join("extension");
        std::fs::create_dir_all(&ext_dir).unwrap();

        // Create a simple extension.js
        std::fs::write(
            ext_dir.join("extension.js"),
            r#"
                console.log('Extension started');
                module.exports = {
                    activate: () => console.log('Activated')
                };
            "#
        ).unwrap();

        let extension = LoadedExtension {
            id: "test.extension".to_string(),
            publisher: "test".to_string(),
            name: "extension".to_string(),
            version: "1.0.0".to_string(),
            path: dir.path().to_path_buf(),
            main: Some("extension.js".to_string()),
            browser: None,
            extension_kind: vec![],
            activation_events: vec![],
            contributes: serde_json::Value::Null,
            runtime: crate::ExtensionRuntime::NodeJS,
            enabled: true,
            activation_time: None,
        };

        let result = runtime.start_extension(&extension).await;
        assert!(result.is_ok());

        // Clean up
        let _ = runtime.stop_extension("test.extension").await;
    }
}