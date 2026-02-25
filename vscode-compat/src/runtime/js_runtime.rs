//! Pure JavaScript runtime for VS Code extensions
//!
//! Uses QuickJS for lightweight JavaScript execution without Node.js dependencies.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use tokio::sync::Mutex;
use tracing::{info, warn, error};

use super::{Runtime, JSValue, ExtensionContext, CancellationToken};
use crate::api::VSCodeAPI;
use crate::LoadedExtension;
use crate::runtime::ExtensionExports;

#[cfg(feature = "js-runtime")]
use quick_js::{Context, JsValue};

/// JavaScript runtime using QuickJS
pub struct JSRuntime {
    /// Main JavaScript context
    context: Arc<Mutex<Context>>,
    /// Loaded modules cache
    modules: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    /// Extension contexts
    extension_contexts: Arc<Mutex<HashMap<String, ExtensionContext>>>,
    /// API reference
    api: Option<Arc<VSCodeAPI>>,
}

impl JSRuntime {
    /// Create a new JavaScript runtime
    pub fn new() -> Result<Self> {
        #[cfg(not(feature = "js-runtime"))]
        {
            return Err(anyhow!("QuickJS runtime not enabled (feature 'js-runtime' required)"));
        }

        #[cfg(feature = "js-runtime")]
        {
            let context = Context::new().map_err(|e| anyhow!("Failed to create QuickJS context: {}", e))?;

            // Initialize global object
            let _ = context.eval("var global = this;")?;
            let _ = context.eval(r#"
                var console = {
                    log: function(...args) { print(args.map(String).join(' ')); },
                    error: function(...args) { printErr(args.map(String).join(' ')); },
                    warn: function(...args) { printErr('WARN: ' + args.map(String).join(' ')); },
                    info: function(...args) { print('INFO: ' + args.map(String).join(' ')); },
                    debug: function(...args) { print('DEBUG: ' + args.map(String).join(' ')); }
                };
            "#)?;

            // Initialize setTimeout/setInterval
            let _ = context.eval(r#"
                var timers = {};
                var timerId = 0;

                function setTimeout(callback, ms) {
                    var id = timerId++;
                    timers[id] = callback;
                    // In a real implementation, this would schedule with tokio
                    return id;
                }

                function clearTimeout(id) {
                    delete timers[id];
                }

                function setInterval(callback, ms) {
                    // Not implemented
                    return 0;
                }

                function clearInterval(id) {}
            "#)?;

            Ok(Self {
                context: Arc::new(Mutex::new(context)),
                modules: Arc::new(Mutex::new(HashMap::new())),
                extension_contexts: Arc::new(Mutex::new(HashMap::new())),
                api: None,
            })
        }
    }

    /// Set the VS Code API instance
    pub fn set_api(&mut self, api: Arc<VSCodeAPI>) {
        self.api = Some(api);
    }

    /// Load an extension module
    async fn load_module(&self, path: &Path) -> Result<()> {
        let content = tokio::fs::read(path).await?;
        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module")
            .to_string();

        self.modules.lock().await.insert(name, content);
        Ok(())
    }

    /// Create extension context
    async fn create_context(&self, extension: &LoadedExtension) -> Result<ExtensionContext> {
        let api = self.api.clone().ok_or_else(|| anyhow!("API not initialized"))?;
        Ok(ExtensionContext::new(extension.id.clone(), api))
    }

    /// Inject VS Code API into context
    async fn inject_vscode_api(&self, extension_id: &str) -> Result<()> {
        let mut context = self.context.lock().await;

        // Create vscode namespace
        let vscode_js = r#"
            var vscode = (function() {
                const commands = {};
                
                return {
                    commands: {
                        registerCommand: function(id, callback) {
                            commands[id] = callback;
                        },
                        executeCommand: function(id, ...args) {
                            // This would call into Rust
                            return Promise.resolve(null);
                        }
                    },
                    window: {
                        showInformationMessage: function(msg) { console.log(msg); return Promise.resolve(); },
                        showErrorMessage: function(msg) { console.error(msg); return Promise.resolve(); },
                        showWarningMessage: function(msg) { console.warn(msg); return Promise.resolve(); },
                        createOutputChannel: function(name) { return { append: console.log, appendLine: console.log }; }
                    },
                    workspace: {
                        getConfiguration: function() { return {}; }
                    }
                };
            })();
        "#;

        let _ = context.eval(vscode_js)?;

        Ok(())
    }

    /// Parse extension exports
    fn parse_exports(&self, code: &str) -> Result<ExtensionExports> {
        let mut exports = ExtensionExports {
            activate: None,
            deactivate: None,
        };

        // Look for module.exports or exports
        if code.contains("module.exports") || code.contains("exports.") {
            // This is CommonJS, we need to wrap it
        }

        // Look for export statements (ES modules)
        if code.contains("export function activate") {
            exports.activate = Some("activate".to_string());
        }
        if code.contains("export function deactivate") {
            exports.deactivate = Some("deactivate".to_string());
        }

        Ok(exports)
    }

    /// Wrap CommonJS module
    fn wrap_commonjs(&self, code: &str) -> String {
        format!(r#"
            (function(module, exports, require) {{
                {code}
                return module.exports;
            }})(module, exports, require);
        "#, code = code)
    }
}

#[cfg(feature = "js-runtime")]
impl Runtime for JSRuntime {
    fn execute_extension(&self, extension: &LoadedExtension) -> Result<()> {
        let extension_id = extension.id.clone();

        // In a real implementation, this would be async
        let runtime = self.clone();
        tokio::task::spawn_local(async move {
            if let Err(e) = runtime.execute_extension_async(&extension_id).await {
                error!("Failed to execute extension {}: {}", extension_id, e);
            }
        });

        Ok(())
    }

    fn call_function(&self, name: &str, args: Vec<JSValue>) -> Result<JSValue> {
        let context = self.context.blocking_lock();
        
        // Convert args to JS values
        let js_args: Vec<JsValue> = args.into_iter()
            .map(|arg| match arg {
                JSValue::Null => JsValue::Null,
                JSValue::Undefined => JsValue::Undefined,
                JSValue::Bool(b) => JsValue::Bool(b),
                JSValue::Number(n) => JsValue::Float(n),
                JSValue::String(s) => JsValue::String(s),
                JSValue::Array(arr) => {
                    // Convert array - simplified
                    JsValue::String(format!("{:?}", arr))
                }
                JSValue::Object(obj) => {
                    // Convert object - simplified
                    JsValue::String(format!("{:?}", obj))
                }
                JSValue::Function(_) => JsValue::String("[function]".to_string()),
            })
            .collect();

        // Call function
        let result = context.call_function(name, js_args)
            .map_err(|e| anyhow!("Failed to call function {}: {}", name, e))?;

        // Convert result back
        match result {
            JsValue::Null => Ok(JSValue::Null),
            JsValue::Undefined => Ok(JSValue::Undefined),
            JsValue::Bool(b) => Ok(JSValue::Bool(b)),
            JsValue::Int(i) => Ok(JSValue::Number(i as f64)),
            JsValue::Float(f) => Ok(JSValue::Number(f)),
            JsValue::String(s) => Ok(JSValue::String(s)),
            _ => Ok(JSValue::Null),
        }
    }

    fn get_value(&self, name: &str) -> Result<JSValue> {
        let context = self.context.blocking_lock();
        let value = context.eval_as::<JsValue>(&format!("typeof {} !== 'undefined' ? {} : null", name, name))
            .map_err(|e| anyhow!("Failed to get value {}: {}", name, e))?;

        match value {
            JsValue::Null => Ok(JSValue::Null),
            JsValue::Undefined => Ok(JSValue::Undefined),
            JsValue::Bool(b) => Ok(JSValue::Bool(b)),
            JsValue::Int(i) => Ok(JSValue::Number(i as f64)),
            JsValue::Float(f) => Ok(JSValue::Number(f)),
            JsValue::String(s) => Ok(JSValue::String(s)),
            _ => Ok(JSValue::Null),
        }
    }

    fn set_value(&self, name: &str, value: JSValue) -> Result<()> {
        let context = self.context.blocking_lock();
        
        let js_value = match &value {
            JSValue::Null => JsValue::Null,
            JSValue::Undefined => JsValue::Undefined,
            JSValue::Bool(b) => JsValue::Bool(*b),
            JSValue::Number(n) => JsValue::Float(*n),
            JSValue::String(s) => JsValue::String(s.clone()),
            _ => JsValue::Null,
        };

        // Convert our `JSValue` to a JSON-like literal and assign
        let json_val = match &value {
            JSValue::Null | JSValue::Undefined => serde_json::Value::Null,
            JSValue::Bool(b) => serde_json::Value::Bool(*b),
            JSValue::Number(n) => serde_json::Value::Number(serde_json::Number::from_f64(*n).unwrap_or(serde_json::Number::from(0))),
            JSValue::String(s) => serde_json::Value::String(s.clone()),
            JSValue::Array(arr) => serde_json::Value::Array(arr.iter().map(|v| match v {
                JSValue::String(s) => serde_json::Value::String(s.clone()),
                JSValue::Number(n) => serde_json::Value::Number(serde_json::Number::from_f64(*n).unwrap_or(serde_json::Number::from(0))),
                JSValue::Bool(b) => serde_json::Value::Bool(*b),
                _ => serde_json::Value::Null,
            }).collect()),
            JSValue::Object(obj) => {
                let mut map = serde_json::Map::new();
                for (k, v) in obj.iter() {
                    let vj = match v {
                        JSValue::String(s) => serde_json::Value::String(s.clone()),
                        JSValue::Number(n) => serde_json::Value::Number(serde_json::Number::from_f64(*n).unwrap_or(serde_json::Number::from(0))),
                        JSValue::Bool(b) => serde_json::Value::Bool(*b),
                        _ => serde_json::Value::Null,
                    };
                    map.insert(k.clone(), vj);
                }
                serde_json::Value::Object(map)
            }
            JSValue::Function(_) => serde_json::Value::Null,
        };

        let js_str = format!("{} = {}", name, serde_json::to_string(&json_val)?);
        context.eval(&js_str)
            .map_err(|e| anyhow!("Failed to set value {}: {}", name, e))?;

        Ok(())
    }
}

impl JSRuntime {
    /// Async version of execute_extension
    async fn execute_extension_async(&self, extension_id: &str) -> Result<()> {
        info!("Executing extension in JS runtime: {}", extension_id);

        // Get extension
        // In a real implementation, we'd look up the extension from registry
        // For now, just log
        info!("Extension {} would be executed here", extension_id);

        // Create context
        let mut ctx = self.context.lock().await;
        
        // Inject vscode API
        let vscode_api = r#"
            const vscode = {
                commands: {
                    registerCommand: (id, callback) => {
                        console.log('Command registered:', id);
                    }
                },
                window: {
                    showInformationMessage: (msg) => {
                        console.log('[INFO]', msg);
                        return Promise.resolve();
                    }
                }
            };
        "#;
        ctx.eval(vscode_api)?;

        // Load extension code
        // This would read the extension's main file and eval it

        Ok(())
    }
}

impl Clone for JSRuntime {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
            modules: self.modules.clone(),
            extension_contexts: self.extension_contexts.clone(),
            api: self.api.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "js-runtime")]
    fn test_js_runtime_creation() {
        let runtime = JSRuntime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    #[cfg(feature = "js-runtime")]
    fn test_eval() {
        let runtime = JSRuntime::new().unwrap();
        let context = runtime.context.blocking_lock();
        let result = context.eval("1 + 2").unwrap();
        match result {
            JsValue::Int(i) => assert_eq!(i, 3),
            JsValue::Float(f) => assert_eq!(f as i32, 3),
            _ => panic!("unexpected result"),
        }
    }
}