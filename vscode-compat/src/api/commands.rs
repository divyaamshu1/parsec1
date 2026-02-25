//! VS Code Commands API Implementation
//!
//! Implements vscode.commands.* API for registering and executing commands.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use tokio::sync::RwLock;
use serde_json::Value;

use super::{Disposable, VSCodeAPI};

/// Command handler function type
pub type CommandHandler = Box<dyn Fn(Vec<Value>) -> Result<Value> + Send + Sync>;

/// Commands API implementation
pub struct CommandsAPI {
    /// Registered commands
    commands: Arc<RwLock<HashMap<String, CommandHandler>>>,
    /// Command aliases
    aliases: Arc<RwLock<HashMap<String, String>>>,
    /// Command history
    history: Arc<RwLock<Vec<CommandExecution>>>,
}

/// Command execution record
#[derive(Debug, Clone)]
pub struct CommandExecution {
    pub command: String,
    pub args: Vec<Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl CommandsAPI {
    /// Create a new commands API
    pub fn new() -> Self {
        Self {
            commands: Arc::new(RwLock::new(HashMap::new())),
            aliases: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::with_capacity(100))),
        }
    }

    /// Register a command
    pub async fn register_command<F>(&self, command: &str, handler: F) -> impl Disposable
    where
        F: Fn(Vec<Value>) -> Result<Value> + Send + Sync + 'static,
    {
        let command = command.to_string();
        let mut commands = self.commands.write().await;
        commands.insert(command.clone(), Box::new(handler));

        // Create disposable for cleanup
        let commands = self.commands.clone();
        tokio::spawn(async move {
            // This would be returned as a disposable
        });

        // Return a simple disposable
        struct CommandDisposable {
            command: String,
            commands: Arc<RwLock<HashMap<String, CommandHandler>>>,
        }

        impl Disposable for CommandDisposable {
            fn dispose(&self) {
                let commands = self.commands.clone();
                let command = self.command.clone();
                tokio::spawn(async move {
                    commands.write().await.remove(&command);
                });
            }
        }

        CommandDisposable {
            command,
            commands: self.commands.clone(),
        }
    }

    /// Execute a command
    pub async fn execute_command(&self, command: &str, args: Vec<Value>) -> Result<Value> {
        let start = std::time::Instant::now();
        let command_str = command.to_string();

        // Check for alias
        let actual_command = self.aliases.read().await.get(command).cloned().unwrap_or_else(|| command.to_string());

        // Look up command
        let handler = {
            let commands = self.commands.read().await;
            commands.get(&actual_command).cloned()
        };

        let (result, error) = match handler {
            Some(handler) => {
                match handler(args.clone()) {
                    Ok(value) => (Some(value), None),
                    Err(e) => (None, Some(e.to_string())),
                }
            }
            None => {
                // Try built-in commands
                match self.execute_builtin(&actual_command, args.clone()).await {
                    Ok(value) => (Some(value), None),
                    Err(e) => (None, Some(e.to_string())),
                }
            }
        };

        // Record execution
        let duration = start.elapsed();
        let execution = CommandExecution {
            command: actual_command,
            args,
            timestamp: chrono::Utc::now(),
            result: result.clone(),
            error: error.clone(),
            duration_ms: duration.as_millis() as u64,
        };

        self.history.write().await.push(execution);
        if self.history.read().await.len() > 1000 {
            self.history.write().await.remove(0);
        }

        match (result, error) {
            (Some(value), _) => Ok(value),
            (None, Some(err)) => Err(anyhow!("Command failed: {}", err)),
            (None, None) => Err(anyhow!("Command not found: {}", command)),
        }
    }

    /// Execute built-in commands
    async fn execute_builtin(&self, command: &str, args: Vec<Value>) -> Result<Value> {
        match command {
            // Built-in VS Code commands
            "workbench.action.files.newUntitledFile" => {
                // Create new untitled file
                Ok(Value::Null)
            }
            "workbench.action.files.save" => {
                // Save current file
                Ok(Value::Null)
            }
            "workbench.action.files.saveAs" => {
                // Save as
                Ok(Value::Null)
            }
            "workbench.action.closeActiveEditor" => {
                // Close active editor
                Ok(Value::Null)
            }
            "workbench.action.splitEditor" => {
                // Split editor
                Ok(Value::Null)
            }
            "editor.action.formatDocument" => {
                // Format document
                Ok(Value::Null)
            }
            "editor.action.sourceAction" => {
                // Show source actions
                Ok(Value::Null)
            }
            "workbench.action.quickOpen" => {
                // Show quick open
                Ok(Value::Null)
            }
            "workbench.action.showCommands" => {
                // Show command palette
                Ok(Value::Null)
            }
            "git.commit" => {
                // Git commit
                Ok(Value::Null)
            }
            "git.push" => {
                // Git push
                Ok(Value::Null)
            }
            _ => Err(anyhow!("Unknown built-in command: {}", command)),
        }
    }

    /// Get all registered commands
    pub async fn get_commands(&self) -> Vec<String> {
        self.commands.read().await.keys().cloned().collect()
    }

    /// Check if a command exists
    pub async fn has_command(&self, command: &str) -> bool {
        self.commands.read().await.contains_key(command)
    }

    /// Create a command alias
    pub async fn create_alias(&self, alias: &str, target: &str) -> Result<()> {
        self.aliases.write().await.insert(alias.to_string(), target.to_string());
        Ok(())
    }

    /// Remove a command alias
    pub async fn remove_alias(&self, alias: &str) -> Result<()> {
        self.aliases.write().await.remove(alias);
        Ok(())
    }

    /// Get command execution history
    pub async fn get_history(&self, limit: Option<usize>) -> Vec<CommandExecution> {
        let history = self.history.read().await;
        let limit = limit.unwrap_or(history.len());
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Execute multiple commands in sequence
    pub async fn execute_sequence(&self, commands: Vec<(String, Vec<Value>)>) -> Result<Vec<Value>> {
        let mut results = Vec::new();
        for (cmd, args) in commands {
            results.push(self.execute_command(&cmd, args).await?);
        }
        Ok(results)
    }

    /// Execute command with timeout
    pub async fn execute_with_timeout(&self, command: &str, args: Vec<Value>, timeout_ms: u64) -> Result<Value> {
        let execute_future = self.execute_command(command, args);
        match tokio::time::timeout(tokio::time::Duration::from_millis(timeout_ms), execute_future).await {
            Ok(result) => result,
            Err(_) => Err(anyhow!("Command execution timed out after {}ms", timeout_ms)),
        }
    }
}

impl Default for CommandsAPI {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_execute() {
        let api = CommandsAPI::new();

        // Register a test command
        let disposable = api.register_command("test.echo", |args| {
            Ok(args.first().cloned().unwrap_or(Value::Null))
        }).await;

        // Execute the command
        let result = api.execute_command("test.echo", vec![Value::String("hello".to_string())]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("hello".to_string()));

        // Check history
        let history = api.get_history(None).await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].command, "test.echo");
    }

    #[tokio::test]
    async fn test_unknown_command() {
        let api = CommandsAPI::new();
        let result = api.execute_command("unknown.command", vec![]).await;
        assert!(result.is_err());
    }
}