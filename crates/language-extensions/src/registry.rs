//! Language extension registry

use std::path::PathBuf;
use anyhow::Result;
use std::sync::{Arc, Mutex};

use crate::loader::LanguageExtensionLoader;
use crate::types::LoadedLanguage;

/// Language extension registry (singleton)
pub struct LanguageRegistry {
    loader: Arc<Mutex<LanguageExtensionLoader>>,
}

impl LanguageRegistry {
    /// Create new registry
    pub fn new(extensions_dir: PathBuf) -> Result<Self> {
        let loader = LanguageExtensionLoader::new(extensions_dir)?;
        Ok(Self {
            loader: Arc::new(Mutex::new(loader)),
        })
    }

    /// Global instance
    pub fn global() -> Option<&'static LanguageRegistry> {
        use std::sync::OnceLock;
        static INSTANCE: OnceLock<LanguageRegistry> = OnceLock::new();
        INSTANCE.get()
    }

    /// Initialize global instance
    pub fn init_global(extensions_dir: PathBuf) -> Result<()> {
        use std::sync::OnceLock;
        static INSTANCE: OnceLock<LanguageRegistry> = OnceLock::new();
        INSTANCE.set(Self::new(extensions_dir)?)
            .map_err(|_| anyhow::anyhow!("Global registry already initialized"))
    }

    /// Load all language extensions
    pub fn load_all(&self) -> Result<Vec<String>> {
        self.loader.lock().unwrap().load_all()
    }

    /// Load a specific extension
    pub fn load(&self, path: &std::path::Path) -> Result<String> {
        self.loader.lock().unwrap().load_from_dir(path)
    }

    /// Get language by name
    pub fn get_by_name(&self, name: &str) -> Option<LoadedLanguage> {
        self.loader.lock().unwrap().get_by_name(name).cloned()
    }

    /// Get language by extension
    pub fn get_by_extension(&self, ext: &str) -> Option<LoadedLanguage> {
        self.loader.lock().unwrap().get_by_extension(ext).cloned()
    }

    /// Get all languages
    pub fn all(&self) -> Vec<LoadedLanguage> {
        self.loader.lock().unwrap().all().into_iter().cloned().collect()
    }
}