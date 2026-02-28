//! Breakpoint management

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};

/// Breakpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub id: usize,
    pub file: PathBuf,
    pub line: usize,
    pub column: Option<usize>,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
    pub log_message: Option<String>,
    pub enabled: bool,
    pub verified: bool,
}

/// Function breakpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionBreakpoint {
    pub id: usize,
    pub name: String,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
    pub enabled: bool,
}

/// Exception breakpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionBreakpoint {
    pub id: usize,
    pub exception: String,
    pub condition: Option<String>,
    pub enabled: bool,
}

/// Breakpoint manager
pub struct BreakpointManager {
    breakpoints: Arc<tokio::sync::RwLock<HashMap<usize, Breakpoint>>>,
    function_breakpoints: Arc<tokio::sync::RwLock<HashMap<usize, FunctionBreakpoint>>>,
    exception_breakpoints: Arc<tokio::sync::RwLock<HashMap<usize, ExceptionBreakpoint>>>,
    next_id: Arc<tokio::sync::Mutex<usize>>,
}

impl BreakpointManager {
    /// Create new breakpoint manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            breakpoints: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            function_breakpoints: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            exception_breakpoints: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            next_id: Arc::new(tokio::sync::Mutex::new(1)),
        })
    }

    /// Add breakpoint
    pub async fn add_breakpoint(&self, file: PathBuf, line: usize) -> Result<usize> {
        let mut next_id = self.next_id.lock().await;
        let id = *next_id;
        *next_id += 1;

        let bp = Breakpoint {
            id,
            file,
            line,
            column: None,
            condition: None,
            hit_condition: None,
            log_message: None,
            enabled: true,
            verified: false,
        };

        self.breakpoints.write().await.insert(id, bp);
        Ok(id)
    }

    /// Remove breakpoint
    pub async fn remove_breakpoint(&self, id: usize) -> Result<()> {
        self.breakpoints.write().await.remove(&id);
        Ok(())
    }

    /// Get breakpoint
    pub async fn get_breakpoint(&self, id: usize) -> Option<Breakpoint> {
        self.breakpoints.read().await.get(&id).cloned()
    }

    /// Get all breakpoints
    pub async fn get_breakpoints(&self) -> Vec<Breakpoint> {
        self.breakpoints.read().await.values().cloned().collect()
    }

    /// Get breakpoints for file
    pub async fn get_breakpoints_for_file(&self, file: &Path) -> Vec<Breakpoint> {
        self.breakpoints.read().await.values()
            .filter(|bp| bp.file == file)
            .cloned()
            .collect()
    }

    /// Enable breakpoint
    pub async fn enable_breakpoint(&self, id: usize) -> Result<()> {
        if let Some(bp) = self.breakpoints.write().await.get_mut(&id) {
            bp.enabled = true;
        }
        Ok(())
    }

    /// Disable breakpoint
    pub async fn disable_breakpoint(&self, id: usize) -> Result<()> {
        if let Some(bp) = self.breakpoints.write().await.get_mut(&id) {
            bp.enabled = false;
        }
        Ok(())
    }

    /// Set breakpoint condition
    pub async fn set_condition(&self, id: usize, condition: Option<String>) -> Result<()> {
        if let Some(bp) = self.breakpoints.write().await.get_mut(&id) {
            bp.condition = condition;
        }
        Ok(())
    }

    /// Set hit condition
    pub async fn set_hit_condition(&self, id: usize, condition: Option<String>) -> Result<()> {
        if let Some(bp) = self.breakpoints.write().await.get_mut(&id) {
            bp.hit_condition = condition;
        }
        Ok(())
    }

    /// Set log message
    pub async fn set_log_message(&self, id: usize, message: Option<String>) -> Result<()> {
        if let Some(bp) = self.breakpoints.write().await.get_mut(&id) {
            bp.log_message = message;
        }
        Ok(())
    }

    /// Verify breakpoint
    pub async fn verify_breakpoint(&self, id: usize, verified: bool) -> Result<()> {
        if let Some(bp) = self.breakpoints.write().await.get_mut(&id) {
            bp.verified = verified;
        }
        Ok(())
    }

    /// Add function breakpoint
    pub async fn add_function_breakpoint(&self, name: String) -> Result<usize> {
        let mut next_id = self.next_id.lock().await;
        let id = *next_id;
        *next_id += 1;

        let bp = FunctionBreakpoint {
            id,
            name,
            condition: None,
            hit_condition: None,
            enabled: true,
        };

        self.function_breakpoints.write().await.insert(id, bp);
        Ok(id)
    }

    /// Add exception breakpoint
    pub async fn add_exception_breakpoint(&self, exception: String) -> Result<usize> {
        let mut next_id = self.next_id.lock().await;
        let id = *next_id;
        *next_id += 1;

        let bp = ExceptionBreakpoint {
            id,
            exception,
            condition: None,
            enabled: true,
        };

        self.exception_breakpoints.write().await.insert(id, bp);
        Ok(id)
    }

    /// Clear all breakpoints
    pub async fn clear(&self) {
        self.breakpoints.write().await.clear();
        self.function_breakpoints.write().await.clear();
        self.exception_breakpoints.write().await.clear();
    }
}