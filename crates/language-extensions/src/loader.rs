//! Language extension loader

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tracing::{info, warn};

use crate::types::*;
use crate::wasm::WasmGrammarLoader;

/// Language extension loader
pub struct LanguageExtensionLoader {
    extensions_dir: PathBuf,
    grammar_loader: WasmGrammarLoader,
    loaded: HashMap<String, LoadedLanguage>,
}

impl LanguageExtensionLoader {
    /// Create a new loader
    pub fn new(extensions_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            extensions_dir,
            grammar_loader: WasmGrammarLoader::new()?,
            loaded: HashMap::new(),
        })
    }

    /// Load a language extension from a directory
    pub fn load_from_dir(&mut self, path: &Path) -> Result<String> {
        info!("Loading language extension from: {}", path.display());
        
        // Read manifest
        let manifest_path = path.join("manifest.toml");
        if !manifest_path.exists() {
            return Err(LanguageExtensionError::InvalidManifest(
                "manifest.toml not found".to_string()
            ).into());
        }
        
        let manifest_content = fs::read_to_string(manifest_path)?;
        let manifest: LanguageManifest = toml::from_str(&manifest_content)?;
        
        // Load grammar
        let grammar_path = path.join(&manifest.language.grammar);
        if !grammar_path.exists() {
            return Err(LanguageExtensionError::GrammarNotFound(
                manifest.language.grammar.clone()
            ).into());
        }
        
        let grammar_bytes = fs::read(grammar_path)?;
        let grammar = self.grammar_loader.load_grammar(&grammar_bytes)?;
        
        // Load queries
        let mut queries = HashMap::new();
        for query_file in &manifest.language.queries {
            let query_path = path.join(query_file);
            if !query_path.exists() {
                warn!("Query file not found: {}", query_file);
                continue;
            }
            let content = fs::read_to_string(query_path)?;
            queries.insert(query_file.clone(), content);
        }
        
        // Create loaded language
        let loaded = LoadedLanguage {
            id: manifest.id.clone(),
            name: manifest.language.name.clone(),
            extensions: manifest.language.extensions.clone(),
            grammar,
            queries,
            indentation: manifest.language.indentation,
            comments: manifest.language.comments,
            path: path.to_path_buf(),
        };
        
        // Store
        let id = manifest.id.clone();
        self.loaded.insert(id.clone(), loaded);
        
        info!("Loaded language extension: {} v{}", manifest.name, manifest.version);
        Ok(id)
    }

    /// Load from the extensions directory
    pub fn load_all(&mut self) -> Result<Vec<String>> {
        let mut loaded = Vec::new();
        
        if !self.extensions_dir.exists() {
            return Ok(loaded);
        }
        
        for entry in fs::read_dir(&self.extensions_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Ok(id) = self.load_from_dir(&path) {
                    loaded.push(id);
                }
            }
        }
        
        Ok(loaded)
    }

    /// Get a loaded language by name
    pub fn get_by_name(&self, name: &str) -> Option<&LoadedLanguage> {
        self.loaded.values().find(|l| l.name == name)
    }

    /// Get a loaded language by file extension
    pub fn get_by_extension(&self, ext: &str) -> Option<&LoadedLanguage> {
        self.loaded.values().find(|l| l.extensions.contains(&ext.to_string()))
    }

    /// Get a loaded language by ID
    pub fn get_by_id(&self, id: &str) -> Option<&LoadedLanguage> {
        self.loaded.get(id)
    }

    /// Get all loaded languages
    pub fn all(&self) -> Vec<&LoadedLanguage> {
        self.loaded.values().collect()
    }

    /// Unload a language
    pub fn unload(&mut self, id: &str) -> bool {
        self.loaded.remove(id).is_some()
    }
}