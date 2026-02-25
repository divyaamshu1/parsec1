//! Universal Debugger with DAP (Debug Adapter Protocol) support
//!
//! This crate provides a complete debugging solution for any language/runtime
//! that supports the Debug Adapter Protocol.

#![allow(dead_code, unused_imports, unused_variables)]

pub mod dap;
pub mod breakpoint;
pub mod callstack;
pub mod variables;
pub mod watch;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::sync::{RwLock, Mutex};
use tracing::{info, warn, debug};
use serde::{Serialize, Deserialize};

pub use dap::*;
pub use breakpoint::*;
pub use callstack::*;
pub use variables::*;
pub use watch::*;

/// Main debugger manager
pub struct DebuggerManager {
    sessions: Arc<RwLock<HashMap<String, DebugSession>>>,
    adapters: Arc<RwLock<HashMap<String, Box<dyn DebugAdapter>>>>,
    breakpoint_manager: Arc<breakpoint::BreakpointManager>,
    callstack_manager: Arc<callstack::CallstackManager>,
    variables_manager: Arc<variables::VariablesManager>,
    watch_manager: Arc<watch::WatchManager>,
    config: DebuggerConfig,
}

/// Debugger configuration
#[derive(Debug, Clone)]
pub struct DebuggerConfig {
    pub adapters_dir: PathBuf,
    pub log_dir: PathBuf,
    pub default_timeout: std::time::Duration,
    pub max_variables_depth: usize,
    pub max_children: usize,
    pub evaluate_timeout: std::time::Duration,
}

impl Default for DebuggerConfig {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("parsec/debug");

        Self {
            adapters_dir: data_dir.join("adapters"),
            log_dir: data_dir.join("logs"),
            default_timeout: std::time::Duration::from_secs(30),
            max_variables_depth: 5,
            max_children: 100,
            evaluate_timeout: std::time::Duration::from_secs(5),
        }
    }
}

/// Debug session
#[derive(Debug, Clone)]
pub struct DebugSession {
    pub id: String,
    pub name: String,
    pub process_id: Option<u32>,
    pub adapter: String,
    pub state: DebugState,
    pub thread_id: Option<usize>,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

/// Debug state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugState {
    Initializing,
    Running,
    Paused,
    Stepping,
    Stopped,
    Terminated,
    Error,
}

/// Debug adapter trait
#[async_trait]
pub trait DebugAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn languages(&self) -> Vec<String>;
    
    async fn start(&self, config: AdapterConfig) -> Result<String>;
    async fn stop(&self, session_id: &str) -> Result<()>;
    
    async fn attach(&self, process_id: u32) -> Result<String>;
    async fn launch(&self, program: &Path, args: Vec<String>) -> Result<String>;
    
    async fn pause(&self, session_id: &str) -> Result<()>;
    async fn resume(&self, session_id: &str) -> Result<()>;
    async fn step_over(&self, session_id: &str, thread_id: usize) -> Result<()>;
    async fn step_into(&self, session_id: &str, thread_id: usize) -> Result<()>;
    async fn step_out(&self, session_id: &str, thread_id: usize) -> Result<()>;
    
    async fn set_breakpoint(&self, session_id: &str, bp: Breakpoint) -> Result<()>;
    async fn remove_breakpoint(&self, session_id: &str, bp_id: usize) -> Result<()>;
    async fn get_breakpoints(&self, session_id: &str) -> Result<Vec<Breakpoint>>;
    
    async fn get_stack_trace(&self, session_id: &str, thread_id: usize) -> Result<Vec<StackFrame>>;
    async fn get_scopes(&self, session_id: &str, frame_id: usize) -> Result<Vec<VariableScope>>;
    async fn get_variables(&self, session_id: &str, var_ref: usize) -> Result<Vec<Variable>>;
    
    async fn evaluate(&self, session_id: &str, expression: &str, frame_id: Option<usize>) -> Result<Variable>;
    async fn terminate(&self, session_id: &str) -> Result<()>;
    async fn disconnect(&self, session_id: &str) -> Result<()>;
}

/// Adapter configuration
#[derive(Debug, Clone)]
pub struct AdapterConfig {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: HashMap<String, String>,
}

impl DebuggerManager {
    /// Create new debugger manager
    pub fn new(config: DebuggerConfig) -> Result<Self> {
        std::fs::create_dir_all(&config.adapters_dir)?;
        std::fs::create_dir_all(&config.log_dir)?;

        Ok(Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            adapters: Arc::new(RwLock::new(HashMap::new())),
            breakpoint_manager: Arc::new(breakpoint::BreakpointManager::new()?),
            callstack_manager: Arc::new(callstack::CallstackManager::new()?),
            variables_manager: Arc::new(variables::VariablesManager::new()?),
            watch_manager: Arc::new(watch::WatchManager::new()?),
            config,
        })
    }

    /// Register a debug adapter
    pub async fn register_adapter(&self, adapter: Box<dyn DebugAdapter>) {
        self.adapters.write().await.insert(adapter.name().to_string(), adapter);
    }

    /// Get adapter for language
    pub async fn get_adapter_for_language(&self, language: &str) -> Option<Box<dyn DebugAdapter>> {
        let adapters = self.adapters.read().await;
        for adapter in adapters.values() {
            if adapter.languages().contains(&language.to_string()) {
                return Some(adapter.box_clone());
            }
        }
        None
    }

    /// Launch debugging session
    pub async fn launch(
        &self,
        adapter_name: &str,
        program: &Path,
        args: Vec<String>,
    ) -> Result<DebugSession> {
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(adapter_name)
            .ok_or_else(|| anyhow!("Adapter not found: {}", adapter_name))?;

        let session_id = adapter.launch(program, args).await?;
        
        let session = DebugSession {
            id: session_id.clone(),
            name: format!("Debug {}", program.display()),
            process_id: None,
            adapter: adapter_name.to_string(),
            state: DebugState::Initializing,
            thread_id: None,
            start_time: chrono::Utc::now(),
        };

        self.sessions.write().await.insert(session_id, session.clone());
        Ok(session)
    }

    /// Attach to running process
    pub async fn attach(&self, adapter_name: &str, process_id: u32) -> Result<DebugSession> {
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(adapter_name)
            .ok_or_else(|| anyhow!("Adapter not found: {}", adapter_name))?;

        let session_id = adapter.attach(process_id).await?;
        
        let session = DebugSession {
            id: session_id.clone(),
            name: format!("Attach to PID {}", process_id),
            process_id: Some(process_id),
            adapter: adapter_name.to_string(),
            state: DebugState::Initializing,
            thread_id: None,
            start_time: chrono::Utc::now(),
        };

        self.sessions.write().await.insert(session_id, session.clone());
        Ok(session)
    }

    /// Get session
    pub async fn get_session(&self, session_id: &str) -> Option<DebugSession> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// List sessions
    pub async fn list_sessions(&self) -> Vec<DebugSession> {
        self.sessions.read().await.values().cloned().collect()
    }

    /// Stop session
    pub async fn stop_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(session_id) {
            let adapters = self.adapters.read().await;
            if let Some(adapter) = adapters.get(&session.adapter) {
                adapter.stop(session_id).await?;
            }
        }
        Ok(())
    }

    /// Pause execution
    pub async fn pause(&self, session_id: &str) -> Result<()> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow!("Session not found"))?;

        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&session.adapter)
            .ok_or_else(|| anyhow!("Adapter not found"))?;

        adapter.pause(session_id).await
    }

    /// Resume execution
    pub async fn resume(&self, session_id: &str) -> Result<()> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow!("Session not found"))?;

        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&session.adapter)
            .ok_or_else(|| anyhow!("Adapter not found"))?;

        adapter.resume(session_id).await
    }

    /// Step over
    pub async fn step_over(&self, session_id: &str, thread_id: usize) -> Result<()> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow!("Session not found"))?;

        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&session.adapter)
            .ok_or_else(|| anyhow!("Adapter not found"))?;

        adapter.step_over(session_id, thread_id).await
    }

    /// Step into
    pub async fn step_into(&self, session_id: &str, thread_id: usize) -> Result<()> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow!("Session not found"))?;

        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&session.adapter)
            .ok_or_else(|| anyhow!("Adapter not found"))?;

        adapter.step_into(session_id, thread_id).await
    }

    /// Step out
    pub async fn step_out(&self, session_id: &str, thread_id: usize) -> Result<()> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow!("Session not found"))?;

        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&session.adapter)
            .ok_or_else(|| anyhow!("Adapter not found"))?;

        adapter.step_out(session_id, thread_id).await
    }

    /// Set breakpoint
    pub async fn set_breakpoint(&self, session_id: &str, bp: Breakpoint) -> Result<()> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow!("Session not found"))?;

        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&session.adapter)
            .ok_or_else(|| anyhow!("Adapter not found"))?;

        adapter.set_breakpoint(session_id, bp).await
    }

    /// Remove breakpoint
    pub async fn remove_breakpoint(&self, session_id: &str, bp_id: usize) -> Result<()> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow!("Session not found"))?;

        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&session.adapter)
            .ok_or_else(|| anyhow!("Adapter not found"))?;

        adapter.remove_breakpoint(session_id, bp_id).await
    }

    /// Get stack trace
    pub async fn get_stack_trace(&self, session_id: &str, thread_id: usize) -> Result<Vec<StackFrame>> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow!("Session not found"))?;

        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&session.adapter)
            .ok_or_else(|| anyhow!("Adapter not found"))?;

        adapter.get_stack_trace(session_id, thread_id).await
    }

    /// Get scopes
    pub async fn get_scopes(&self, session_id: &str, frame_id: usize) -> Result<Vec<VariableScope>> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow!("Session not found"))?;

        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&session.adapter)
            .ok_or_else(|| anyhow!("Adapter not found"))?;

        adapter.get_scopes(session_id, frame_id).await
    }

    /// Get variables
    pub async fn get_variables(&self, session_id: &str, var_ref: usize) -> Result<Vec<Variable>> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow!("Session not found"))?;

        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&session.adapter)
            .ok_or_else(|| anyhow!("Adapter not found"))?;

        adapter.get_variables(session_id, var_ref).await
    }

    /// Evaluate expression
    pub async fn evaluate(&self, session_id: &str, expression: &str, frame_id: Option<usize>) -> Result<Variable> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow!("Session not found"))?;

        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&session.adapter)
            .ok_or_else(|| anyhow!("Adapter not found"))?;

        adapter.evaluate(session_id, expression, frame_id).await
    }

    /// Get breakpoint manager
    pub fn breakpoints(&self) -> Arc<breakpoint::BreakpointManager> {
        self.breakpoint_manager.clone()
    }

    /// Get callstack manager
    pub fn callstack(&self) -> Arc<callstack::CallstackManager> {
        self.callstack_manager.clone()
    }

    /// Get variables manager
    pub fn variables(&self) -> Arc<variables::VariablesManager> {
        self.variables_manager.clone()
    }

    /// Get watch manager
    pub fn watch(&self) -> Arc<watch::WatchManager> {
        self.watch_manager.clone()
    }
}

impl dyn DebugAdapter {
    fn box_clone(&self) -> Box<dyn DebugAdapter> {
        // Would be implemented by each adapter
        unimplemented!("Clone not implemented")
    }
}