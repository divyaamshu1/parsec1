//! WASM exports that extensions can implement

use wasmtime::TypedFunc;

/// WASM exports that extensions can implement
pub struct ExtensionExports {
    /// Extension activation function
    pub activate: Option<TypedFunc<(i32, i32), i32>>,
    /// Extension deactivation function
    pub deactivate: Option<TypedFunc<(), ()>>,
    /// Handle message function
    pub handle_message: Option<TypedFunc<(i32, i32), i32>>,
    /// Handle command function
    pub handle_command: Option<TypedFunc<(i32, i32, i32), i32>>,
}

impl ExtensionExports {
    /// Create empty exports
    pub fn empty() -> Self {
        Self {
            activate: None,
            deactivate: None,
            handle_message: None,
            handle_command: None,
        }
    }

    /// Check if extension has activate function
    pub fn has_activate(&self) -> bool {
        self.activate.is_some()
    }

    /// Check if extension has deactivate function
    pub fn has_deactivate(&self) -> bool {
        self.deactivate.is_some()
    }

    /// Check if extension has message handler
    pub fn has_message_handler(&self) -> bool {
        self.handle_message.is_some()
    }

    /// Check if extension has command handler
    pub fn has_command_handler(&self) -> bool {
        self.handle_command.is_some()
    }
}