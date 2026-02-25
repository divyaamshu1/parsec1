//! Parsec Extension System
//!
//! Provides a WebAssembly-based extension system for running
//! sandboxed extensions in the Parsec IDE.

#![allow(dead_code, unused_imports, unused_variables, unused_mut, ambiguous_glob_reexports, mismatched_lifetime_syntaxes)]

pub mod runtime;
pub mod api;
pub mod registry;

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Serialize, Deserialize};

pub use runtime::ExtensionRuntime;
pub use api::ExtensionAPI;
pub use registry::ExtensionRegistry;

/// Extension manifest (package.json equivalent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub description: Option<String>,
    pub entry: String, // WASM entry point
    pub engines: HashMap<String, String>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub icon: Option<String>,
    pub activation_events: Vec<String>,
    pub contributes: ExtensionContributes,
    pub capabilities: Vec<String>,
    pub permissions: Vec<String>,
    pub dependencies: Vec<ExtensionDependency>,
    pub dev_dependencies: Vec<ExtensionDependency>,
}

/// Extension contribution points
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtensionContributes {
    pub commands: Vec<ContributedCommand>,
    pub menus: Vec<ContributedMenu>,
    pub keybindings: Vec<ContributedKeybinding>,
    pub themes: Vec<ContributedTheme>,
    pub languages: Vec<ContributedLanguage>,
    pub snippets: Vec<ContributedSnippet>,
    pub views: Vec<ContributedView>,
    pub view_containers: Vec<ContributedViewContainer>,
    pub problem_matchers: Vec<ContributedProblemMatcher>,
    pub task_definitions: Vec<ContributedTaskDefinition>,
    pub debuggers: Vec<ContributedDebugger>,
}

/// Contributed command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributedCommand {
    pub command: String,
    pub title: String,
    pub category: Option<String>,
    pub icon: Option<String>,
    pub enablement: Option<String>,
}

/// Contributed menu item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributedMenu {
    pub location: String,
    pub command: String,
    pub group: Option<String>,
    pub when: Option<String>,
}

/// Contributed keybinding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributedKeybinding {
    pub key: String,
    pub command: String,
    pub when: Option<String>,
    pub mac: Option<String>,
    pub linux: Option<String>,
    pub win: Option<String>,
}

/// Contributed theme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributedTheme {
    pub id: String,
    pub label: String,
    pub path: String,
    pub ui_theme: Option<String>,
}

/// Contributed language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributedLanguage {
    pub id: String,
    pub name: String,
    pub extensions: Vec<String>,
    pub filenames: Vec<String>,
    pub first_line: Option<String>,
    pub configuration: Option<String>,
    pub grammar: Option<String>,
}

/// Contributed snippet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributedSnippet {
    pub language: String,
    pub name: String,
    pub prefix: String,
    pub body: Vec<String>,
    pub description: Option<String>,
}

/// Contributed view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributedView {
    pub id: String,
    pub name: String,
    pub container: String,
    pub icon: Option<String>,
    pub location: ViewLocation,
}

/// View location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewLocation {
    Sidebar,
    Panel,
    Editor,
    ActivityBar,
    StatusBar,
}

/// Contributed view container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributedViewContainer {
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
}

/// Contributed problem matcher
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributedProblemMatcher {
    pub id: String,
    pub file_pattern: Option<String>,
    pub location_pattern: String,
    pub message_pattern: String,
    pub severity: Option<String>,
}

/// Contributed task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributedTaskDefinition {
    pub task_type: String,
    pub required: Vec<String>,
    pub properties: HashMap<String, TaskProperty>,
}

/// Task property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProperty {
    pub property_type: String,
    pub description: String,
    pub default: Option<serde_json::Value>,
}

/// Contributed debugger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributedDebugger {
    pub debugger_type: String,
    pub label: String,
    pub program: String,
    pub runtime: Option<String>,
    pub configuration_attributes: HashMap<String, DebuggerAttribute>,
}

/// Debugger attribute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebuggerAttribute {
    pub attribute_type: String,
    pub description: String,
    pub default: Option<serde_json::Value>,
}

/// Extension dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionDependency {
    pub id: String,
    pub version: String,
    pub optional: bool,
}

/// Extension instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extension {
    pub id: String,
    pub manifest: ExtensionManifest,
    pub path: PathBuf,
    pub enabled: bool,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub size: u64,
    pub permissions: Vec<String>,
}

/// Extension state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtensionState {
    Inactive,
    Activating,
    Active,
    Deactivating,
    Error,
    Terminated,
}

/// Extension event
#[derive(Debug, Clone)]
pub enum ExtensionEvent {
    Installed(String),
    Uninstalled(String),
    Enabled(String),
    Disabled(String),
    Activated(String),
    Deactivated(String),
    Error(String, String),
}

/// Extension ID type
pub type ExtensionId = String;

/// Extension version type
pub type Version = String;

/// Extension manager result
pub type ExtensionResult<T> = Result<T, ExtensionError>;

/// Extension error types
#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    #[error("Extension not found: {0}")]
    NotFound(String),

    #[error("Extension already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("WASM error: {0}")]
    WasmError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Dependency error: {0}")]
    DependencyError(String),

    #[error("Version conflict: {0}")]
    VersionConflict(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl From<std::io::Error> for ExtensionError {
    fn from(err: std::io::Error) -> Self {
        ExtensionError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for ExtensionError {
    fn from(err: serde_json::Error) -> Self {
        ExtensionError::SerializationError(err.to_string())
    }
}

impl From<wasmtime::Error> for ExtensionError {
    fn from(err: wasmtime::Error) -> Self {
        ExtensionError::WasmError(err.to_string())
    }
}