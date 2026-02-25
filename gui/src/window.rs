//! Window management

use tauri::{Window, Manager, WebviewWindow};

/// Create the main application window - in this compile-pass we simply try to
/// retrieve an existing main webview window handle if present.
pub fn create_main_window(app: &tauri::AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}

/// Window handle for managing multiple windows
#[derive(Debug, Clone)]
pub struct WindowHandle {
    pub id: String,
    pub label: String,
    pub window: Window,
}

impl WindowHandle {
    pub fn new(label: &str, window: Window) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            label: label.to_string(),
            window,
        }
    }

    pub fn close(&self) -> Result<(), tauri::Error> {
        self.window.close()
    }

    pub fn minimize(&self) -> Result<(), tauri::Error> {
        self.window.minimize()
    }

    pub fn maximize(&self) -> Result<(), tauri::Error> {
        self.window.maximize()
    }

    pub fn set_title(&self, title: &str) -> Result<(), tauri::Error> {
        self.window.set_title(title)
    }
}