//! Language extension types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Language extension manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageManifest {
    /// Extension ID (e.g., "rust-lang.rust")
    pub id: String,
    /// Display name
    pub name: String,
    /// Version
    pub version: String,
    /// Publisher
    pub publisher: String,
    /// Description
    pub description: Option<String>,
    /// Language definition
    pub language: LanguageDef,
}

/// Language definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageDef {
    /// Language name (used internally)
    pub name: String,
    /// File extensions
    pub extensions: Vec<String>,
    /// Grammar file (WASM)
    pub grammar: String,
    /// Query files
    pub queries: Vec<String>,
    /// Optional indentation rules
    pub indentation: Option<IndentationRules>,
    /// Optional comment syntax
    pub comments: Option<CommentSyntax>,
}

/// Indentation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndentationRules {
    pub increase: Vec<String>,
    pub decrease: Vec<String>,
    pub ignore: Vec<String>,
}

/// Comment syntax
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentSyntax {
    pub line: Option<String>,
    pub block_start: Option<String>,
    pub block_end: Option<String>,
}

/// Loaded language extension
#[derive(Debug, Clone)]
pub struct LoadedLanguage {
    /// Extension ID
    pub id: String,
    /// Language name
    pub name: String,
    /// File extensions
    pub extensions: Vec<String>,
    /// Tree-sitter language (WASM)
    pub grammar: tree_sitter::Language,
    /// Query files content
    pub queries: HashMap<String, String>,
    /// Indentation rules
    pub indentation: Option<IndentationRules>,
    /// Comment syntax
    pub comments: Option<CommentSyntax>,
    /// Installation path
    pub path: PathBuf,
}

/// Language extension errors
#[derive(Debug, thiserror::Error)]
pub enum LanguageExtensionError {
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),
    
    #[error("Grammar not found: {0}")]
    GrammarNotFound(String),
    
    #[error("Query not found: {0}")]
    QueryNotFound(String),
    
    #[error("Failed to load WASM grammar: {0}")]
    WasmLoadError(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
}