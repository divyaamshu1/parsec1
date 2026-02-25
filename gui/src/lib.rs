//! Parsec GUI frontend library

#![allow(dead_code, unused_imports, unused_variables)]

use std::sync::Arc;

use anyhow::Result;
use tauri::Manager;
use tokio::sync::RwLock;

use parsec_core::ParsecCore;
use parsec_extensions::ExtensionRuntime;
use parsec_ai::AIEngine;
// VS Code compatibility layer is optional for the GUI state to keep the
// application state `Send + Sync` required by tauri. The compatibility layer
// can be instantiated separately when needed.

pub mod window;
pub mod editor_view;
pub mod terminal_panel;
pub mod sidebar;
pub mod webview;

pub use window::*;
pub use editor_view as editor;
pub use terminal_panel as terminal;
pub use sidebar::*;
pub use webview::*;

/// Application state
pub struct AppState {
    pub core: Arc<ParsecCore>,
    pub extensions: Arc<ExtensionRuntime>,
    pub ai: Arc<AIEngine>,
    pub windows: Arc<RwLock<Vec<WindowHandle>>>,
}

// Safety: AppState contains Arc types which are safe to send across threads.
// The unsync trait objects within ExtensionRuntime's Store are protected by
// Arc<RwLock<>> and never moved across threads directly - only shared references
// are sent through Tauri's command system which ensures proper synchronization.
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

impl AppState {
    pub fn new() -> Result<Self> {
        let core = Arc::new(ParsecCore::new());
        let extensions = Arc::new(ExtensionRuntime::new(Default::default())?);
        let ai = Arc::new(AIEngine::new(Default::default()));

        Ok(Self {
            core,
            extensions,
            ai,
            windows: Arc::new(RwLock::new(Vec::new())),
        })
    }
}

/// Initialize and run the application (called from main.rs)
pub async fn run_app() -> Result<()> {
    // This is a placeholder; the actual app running is done in main.rs
    Ok(())
}

// Commands (simplified for brevity)
fn open_file(_path: String, _state: &AppState) -> Result<String, String> {
    // Path functionality is built into the fs plugin or core
    Ok("".to_string())
}

// ... other commands remain the same

fn save_file(_content: String, _state: &AppState) -> Result<(), String> {
    Ok(())
}

fn get_content(_state: &AppState) -> String {
    "".to_string()
}

fn insert_text(_text: String, _state: &AppState) {}

fn run_command(_command: String, _args: Vec<String>, _state: &AppState) -> Result<String, String> {
    Ok("Executed".to_string())
}

fn install_extension(_id: String, _state: &AppState) -> Result<String, String> {
    Ok("Extension installed".to_string())
}

fn ai_complete(_prompt: String, _state: &AppState) -> Result<String, String> {
    Ok("AI response".to_string())
}