//! Terminal panel component

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Serialize, Deserialize};
use tauri::{Window, Manager, Emitter};
use tokio::sync::{RwLock};

use parsec_core::terminal::{Terminal, TerminalManager};

/// Terminal instance in GUI
pub struct TerminalInstance {
    pub id: String,
    pub name: String,
    pub terminal: Arc<RwLock<Terminal>>,
    pub rows: u16,
    pub cols: u16,
}

/// Terminal panel manager
pub struct TerminalPanel {
    window: Window,
    terminals: Arc<RwLock<HashMap<String, TerminalInstance>>>,
    active_id: Arc<RwLock<Option<String>>>,
    manager: TerminalManager,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalInfo {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub rows: u16,
    pub cols: u16,
}

impl TerminalPanel {
    pub fn new(window: Window) -> Self {
        Self {
            window,
            terminals: Arc::new(RwLock::new(HashMap::new())),
            active_id: Arc::new(RwLock::new(None)),
            manager: TerminalManager::new(),
        }
    }

    /// Create new terminal
    pub async fn create_terminal(&mut self, name: Option<String>) -> Result<String, String> {
        let id = self.manager.create(name, None).await
            .map_err(|e| e.to_string())?;

        let terminal = self.manager.get(&id)
            .ok_or_else(|| "Terminal not found".to_string())?;

        let size = terminal.read().await.size();

        let instance = TerminalInstance {
            id: id.clone(),
            name: terminal.read().await.name().to_string(),
            terminal: terminal.clone(),
            rows: size.rows,
            cols: size.cols,
        };

        self.terminals.write().await.insert(id.clone(), instance);
        
        // Set as active if first terminal
        if self.terminals.read().await.len() == 1 {
            *self.active_id.write().await = Some(id.clone());
        }

        self.refresh_list().await?;
        Ok(id)
    }

    /// Write to terminal
    pub async fn write(&self, id: &str, data: &[u8]) -> Result<(), String> {
        let terminals = self.terminals.read().await;
        if let Some(instance) = terminals.get(id) {
            let terminal = instance.terminal.read().await;
            terminal.write(data).await
                .map_err(|e| e.to_string())?;
            
            // Read output and send to frontend
            if let Some(output) = terminal.read().await {
                self.window.emit("terminal:output", &(id, output))
                    .map_err(|e: tauri::Error| e.to_string())?;
            }
        }
        Ok(())
    }

    /// Resize terminal
    pub async fn resize(&self, id: &str, rows: u16, cols: u16) -> Result<(), String> {
        let terminals = self.terminals.read().await;
        if let Some(instance) = terminals.get(id) {
            let mut terminal = instance.terminal.write().await;
            terminal.resize(rows, cols).await
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Set active terminal
    pub async fn set_active(&mut self, id: &str) -> Result<(), String> {
        if self.terminals.read().await.contains_key(id) {
            *self.active_id.write().await = Some(id.to_string());
            self.refresh_list().await?;
        }
        Ok(())
    }

    /// Close terminal
    pub async fn close_terminal(&mut self, id: &str) -> Result<(), String> {
        self.terminals.write().await.remove(id);
        self.manager.close(id).await
            .map_err(|e| e.to_string())?;

        // Update active if needed
        let mut active = self.active_id.write().await;
        if active.as_deref() == Some(id) {
            *active = self.terminals.read().await.keys().next().cloned();
        }

        self.refresh_list().await?;
        Ok(())
    }

    /// Get terminal list
    pub async fn list_terminals(&self) -> Vec<TerminalInfo> {
        let terminals = self.terminals.read().await;
        let active = self.active_id.read().await.clone();

        terminals.values().map(|t| TerminalInfo {
            id: t.id.clone(),
            name: t.name.clone(),
            active: Some(&t.id) == active.as_ref(),
            rows: t.rows,
            cols: t.cols,
        }).collect()
    }

    /// Refresh terminal list in UI
    async fn refresh_list(&self) -> Result<(), String> {
        let list = self.list_terminals().await;
        self.window.emit("terminal:list", &list)
            .map_err(|e: tauri::Error| e.to_string())
    }
}

/// Terminal commands
#[tauri::command]
pub async fn terminal_create(
    name: Option<String>,
    state: tauri::State<'_, crate::AppState>,
    window: Window,
) -> Result<String, String> {
    // Would need to get terminal panel for this window
    Ok("term-1".to_string())
}

#[tauri::command]
pub async fn terminal_write(
    id: String,
    data: Vec<u8>,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    // Would write to terminal
    Ok(())
}

#[tauri::command]
pub async fn terminal_resize(
    id: String,
    rows: u16,
    cols: u16,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    // Would resize terminal
    Ok(())
}

#[tauri::command]
pub async fn terminal_set_active(
    id: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    // Would set active terminal
    Ok(())
}

#[tauri::command]
pub async fn terminal_close(
    id: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    // Would close terminal
    Ok(())
}

#[tauri::command]
pub async fn terminal_clear(
    id: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    // Would clear terminal
    Ok(())
}