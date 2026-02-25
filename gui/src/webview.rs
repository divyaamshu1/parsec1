//! WebView integration for extension UI and previews

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Serialize, Deserialize};
use tauri::{Window, Manager};
use tokio::sync::RwLock;

/// WebView instance
#[allow(dead_code)]
pub struct WebViewInstance {
    pub id: String,
    pub title: String,
    pub url: Option<String>,
    pub html: Option<String>,
    pub width: u32,
    pub height: u32,
    pub visible: bool,
    pub window: Window,
}

/// WebView manager
#[allow(dead_code)]
pub struct WebViewManager {
    webviews: Arc<RwLock<HashMap<String, WebViewInstance>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebViewInfo {
    pub id: String,
    pub title: String,
    pub url: Option<String>,
    pub width: u32,
    pub height: u32,
    pub visible: bool,
}

impl WebViewManager {
    pub fn new() -> Self {
        Self {
            webviews: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new webview
    pub async fn create(
        &self,
        window: Window,
        title: &str,
        url: Option<&str>,
        html: Option<&str>,
        width: u32,
        height: u32,
    ) -> Result<String, String> {
        let id = uuid::Uuid::new_v4().to_string();

        let webview = WebViewInstance {
            id: id.clone(),
            title: title.to_string(),
            url: url.map(|s| s.to_string()),
            html: html.map(|s| s.to_string()),
            width,
            height,
            visible: true,
            window,
        };

        self.webviews.write().await.insert(id.clone(), webview);

        // Creating a real Tauri WindowBuilder is gated behind tauri features
        // not enabled in this workspace-check pass. Skip actual window
        // creation here and keep the webview registry updated only.

        Ok(id)
    }

    /// Load URL in webview
    pub async fn load_url(&self, id: &str, url: &str) -> Result<(), String> {
        let mut webviews = self.webviews.write().await;
        if let Some(webview) = webviews.get_mut(id) {
            webview.url = Some(url.to_string());
            webview.html = None;
            
            // Would send to webview window
        }
        Ok(())
    }

    /// Load HTML in webview
    pub async fn load_html(&self, id: &str, html: &str) -> Result<(), String> {
        let mut webviews = self.webviews.write().await;
        if let Some(webview) = webviews.get_mut(id) {
            webview.html = Some(html.to_string());
            webview.url = None;
            
            // Would send to webview window
        }
        Ok(())
    }

    /// Set webview size
    pub async fn set_size(&self, id: &str, width: u32, height: u32) -> Result<(), String> {
        let mut webviews = self.webviews.write().await;
        if let Some(webview) = webviews.get_mut(id) {
            webview.width = width;
            webview.height = height;
            
            let _ = webview.window.set_size(tauri::Size::Physical(
                tauri::PhysicalSize { width, height }
            ));
        }
        Ok(())
    }

    /// Show/hide webview
    pub async fn set_visible(&self, id: &str, visible: bool) -> Result<(), String> {
        let mut webviews = self.webviews.write().await;
        if let Some(webview) = webviews.get_mut(id) {
            webview.visible = visible;
            
            if visible {
                let _ = webview.window.show();
            } else {
                let _ = webview.window.hide();
            }
        }
        Ok(())
    }

    /// Close webview
    pub async fn close(&self, id: &str) -> Result<(), String> {
        let mut webviews = self.webviews.write().await;
        if let Some(webview) = webviews.remove(id) {
            let _ = webview.window.close();
        }
        Ok(())
    }

    /// List webviews
    pub async fn list(&self) -> Vec<WebViewInfo> {
        let webviews = self.webviews.read().await;
        webviews.values().map(|w| WebViewInfo {
            id: w.id.clone(),
            title: w.title.clone(),
            url: w.url.clone(),
            width: w.width,
            height: w.height,
            visible: w.visible,
        }).collect()
    }

    /// Evaluate JavaScript in webview
    pub async fn eval(&self, _id: &str, _js: &str) -> Result<serde_json::Value, String> {
        // Would execute JS and return result
        Ok(serde_json::Value::Null)
    }
}

impl Default for WebViewManager {
    fn default() -> Self {
        Self::new()
    }
}

/// WebView commands
#[tauri::command]
pub async fn webview_create(
    _title: String,
    _url: Option<String>,
    _html: Option<String>,
    _width: u32,
    _height: u32,
    _window: Window,
    _state: tauri::State<'_, crate::AppState>,
) -> Result<String, String> {
    // Would get webview manager from state
    Ok("webview-id".to_string())
}

#[tauri::command]
pub async fn webview_load_url(
    _id: String,
    _url: String,
    _state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn webview_load_html(
    _id: String,
    _html: String,
    _state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn webview_set_size(
    _id: String,
    _width: u32,
    _height: u32,
    _state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn webview_close(
    _id: String,
    _state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn webview_eval(
    _id: String,
    _js: String,
    _state: tauri::State<'_, crate::AppState>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::Value::Null)
}

/// Extension preview in webview
#[allow(dead_code)]
pub struct ExtensionPreview {
    webview_id: String,
    extension_id: String,
}

impl ExtensionPreview {
    pub async fn new(
        manager: &WebViewManager,
        window: Window,
        extension_id: &str,
        html: &str,
    ) -> Result<Self, String> {
        let id = manager.create(
            window,
            &format!("Extension: {}", extension_id),
            None,
            Some(html),
            800,
            600,
        ).await?;

        Ok(Self {
            webview_id: id,
            extension_id: extension_id.to_string(),
        })
    }

    pub async fn update(&self, manager: &WebViewManager, html: &str) -> Result<(), String> {
        manager.load_html(&self.webview_id, html).await
    }

    pub async fn close(&self, manager: &WebViewManager) -> Result<(), String> {
        manager.close(&self.webview_id).await
    }
}

/// Mobile simulator preview
#[allow(dead_code)]
pub struct MobileSimulatorPreview {
    webview_id: String,
    device: String,
    platform: String,
}

impl MobileSimulatorPreview {
    pub async fn new(
        manager: &WebViewManager,
        window: Window,
        device: &str,
        platform: &str,
    ) -> Result<Self, String> {
        let html = format!(
            r#"<div class="simulator {}-{}">
                <style>
                    .simulator {{
                        width: 100%;
                        height: 100%;
                        display: flex;
                        flex-direction: column;
                        background: #1e1e1e;
                        border-radius: 20px;
                        overflow: hidden;
                    }}
                    .simulator iframe {{
                        flex: 1;
                        border: none;
                        background: white;
                    }}
                    .controls {{
                        height: 50px;
                        background: #2d2d2d;
                        display: flex;
                        justify-content: center;
                        align-items: center;
                        gap: 20px;
                    }}
                    .controls button {{
                        background: #0e639c;
                        color: white;
                        border: none;
                        padding: 8px 16px;
                        border-radius: 4px;
                        cursor: pointer;
                    }}
                    .controls button:hover {{
                        background: #1177bb;
                    }}
                </style>
                <iframe id="screen" sandbox="allow-scripts allow-same-origin"></iframe>
                <div class="controls">
                    <button onclick="rotate()">🔄 Rotate</button>
                    <button onclick="home()">🏠 Home</button>
                    <button onclick="back()">◀ Back</button>
                </div>
                <script>
                    function rotate() {{
                        window.external.invoke(JSON.stringify({{ type: 'rotate' }}));
                    }}
                    function home() {{
                        window.external.invoke(JSON.stringify({{ type: 'home' }}));
                    }}
                    function back() {{
                        window.external.invoke(JSON.stringify({{ type: 'back' }}));
                    }}
                    window.addEventListener('click', function(e) {{
                        var rect = document.getElementById('screen').getBoundingClientRect();
                        var x = e.clientX - rect.left;
                        var y = e.clientY - rect.top;
                        window.external.invoke(JSON.stringify({{
                            type: 'touch',
                            x: Math.floor(x * (1080 / rect.width)),
                            y: Math.floor(y * (1920 / rect.height))
                        }}));
                    }});
                </script>
            </div>"#,
            platform.to_lowercase(),
            device.replace(' ', "-").to_lowercase()
        );

        let id = manager.create(
            window,
            &format!("{} Simulator - {}", platform, device),
            None,
            Some(&html),
            360,
            800,
        ).await?;

        Ok(Self {
            webview_id: id,
            device: device.to_string(),
            platform: platform.to_string(),
        })
    }

    pub async fn update_screen(&self, manager: &WebViewManager, screenshot: &str) -> Result<(), String> {
        let js = format!("document.getElementById('screen').src = 'data:image/png;base64,{}';", screenshot);
        manager.eval(&self.webview_id, &js).await?;
        Ok(())
    }

    pub async fn send_touch(&self, manager: &WebViewManager, x: u32, y: u32) -> Result<(), String> {
        let js = format!(
            "var ev = new CustomEvent('simulator-touch', {{ detail: {{ x: {}, y: {} }} }}); window.dispatchEvent(ev);",
            x, y
        );
        manager.eval(&self.webview_id, &js).await?;
        Ok(())
    }

    pub async fn send_key(&self, manager: &WebViewManager, key: &str) -> Result<(), String> {
        let js = format!(
            "var ev = new CustomEvent('simulator-key', {{ detail: {{ key: '{}' }} }}); window.dispatchEvent(ev);",
            key
        );
        manager.eval(&self.webview_id, &js).await?;
        Ok(())
    }
}

/// WebView message handler for extension communication
pub trait WebViewMessageHandler: Send + Sync {
    fn handle_message(&self, message: &str) -> Result<String, String>;
}

pub struct ExtensionMessageHandler {
    extension_id: String,
}

impl WebViewMessageHandler for ExtensionMessageHandler {
    fn handle_message(&self, message: &str) -> Result<String, String> {
        // Forward message to extension
        Ok(format!("Handled for {}: {}", self.extension_id, message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_webview_manager() {
        let manager = WebViewManager::new();
        assert_eq!(manager.list().await.len(), 0);
    }

    // More tests would require actual Tauri windows
}