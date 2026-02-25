//! Variable inspection

use std::collections::HashMap;

use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};

/// Variable scope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableScope {
    pub name: String,
    pub var_ref: usize,
    pub expensive: bool,
}

/// Variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub type_name: Option<String>,
    pub var_ref: usize,
    pub indexed_variables: Option<usize>,
    pub named_variables: Option<usize>,
}

/// Variables manager
pub struct VariablesManager {
    variables: Arc<tokio::sync::RwLock<HashMap<usize, Vec<Variable>>>>,
    scopes: Arc<tokio::sync::RwLock<HashMap<usize, Vec<VariableScope>>>>,
}

impl VariablesManager {
    /// Create new variables manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            variables: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            scopes: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        })
    }

    /// Set scopes for frame
    pub async fn set_scopes(&self, frame_id: usize, scopes: Vec<VariableScope>) {
        self.scopes.write().await.insert(frame_id, scopes);
    }

    /// Get scopes for frame
    pub async fn get_scopes(&self, frame_id: usize) -> Option<Vec<VariableScope>> {
        self.scopes.read().await.get(&frame_id).cloned()
    }

    /// Set variables for reference
    pub async fn set_variables(&self, var_ref: usize, variables: Vec<Variable>) {
        self.variables.write().await.insert(var_ref, variables);
    }

    /// Get variables for reference
    pub async fn get_variables(&self, var_ref: usize) -> Option<Vec<Variable>> {
        self.variables.read().await.get(&var_ref).cloned()
    }

    /// Get variable by reference
    pub async fn get_variable(&self, var_ref: usize, name: &str) -> Option<Variable> {
        if let Some(vars) = self.variables.read().await.get(&var_ref) {
            for var in vars {
                if var.name == name {
                    return Some(var.clone());
                }
            }
        }
        None
    }

    /// Clear for frame
    pub async fn clear(&self, frame_id: usize) {
        self.scopes.write().await.remove(&frame_id);
    }

    /// Clear all
    pub async fn clear_all(&self) {
        self.variables.write().await.clear();
        self.scopes.write().await.clear();
    }
}