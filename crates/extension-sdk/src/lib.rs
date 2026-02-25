//! Parsec Extension SDK
//!
//! This crate provides the types and traits for building extensions for Parsec IDE.

use serde::{Deserialize, Serialize};

pub mod api;
pub mod types;
pub mod macros;

pub use api::*;
pub use types::*;

/// Extension manifest (package.json equivalent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub description: Option<String>,
    pub entry: String,  // WASM entry point
    pub permissions: Vec<Permission>,
    pub activation_events: Vec<String>,
    pub contributes: Contributions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Permission {
    Filesystem { read: Vec<String>, write: Vec<String> },
    Network { domains: Vec<String> },
    Clipboard,
    Terminal,
    Git,
    AI,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Contributions {
    pub commands: Vec<Command>,
    pub menus: Vec<Menu>,
    pub keybindings: Vec<Keybinding>,
    pub themes: Vec<Theme>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub id: String,
    pub title: String,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Menu {
    pub location: String,  // "editor", "explorer", etc.
    pub command: String,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybinding {
    pub key: String,
    pub command: String,
    pub when: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub id: String,
    pub label: String,
    pub path: String,
}