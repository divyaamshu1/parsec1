//! Keybinding system with multi-profile support

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::fs;
use serde::{Serialize, Deserialize};
use keyboard_types::{Key, Modifiers, Code};

use crate::{Result, CustomizationError, CustomizationConfig};

/// Modifier key
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Meta,
    Super,
    Hyper,
}

impl Modifier {
    /// Convert to keyboard_types modifier
    pub fn to_keyboard_mod(&self) -> Modifiers {
        match self {
            Modifier::Ctrl => Modifiers::CONTROL,
            Modifier::Alt => Modifiers::ALT,
            Modifier::Shift => Modifiers::SHIFT,
            Modifier::Meta => Modifiers::META,
            Modifier::Super => Modifiers::SUPER,
            Modifier::Hyper => Modifiers::HYPER,
        }
    }
}

/// Key sequence (multiple keys in order)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeySequence {
    pub keys: Vec<Chord>,
    pub timeout_ms: Option<u64>,
}

impl KeySequence {
    pub fn new(keys: Vec<Chord>) -> Self {
        Self { keys, timeout_ms: Some(500) }
    }

    pub fn single(key: Chord) -> Self {
        Self { keys: vec![key], timeout_ms: None }
    }
}

/// Key chord (simultaneous keys)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Chord {
    pub key: String,
    pub modifiers: Vec<Modifier>,
}

impl Chord {
    pub fn new(key: impl Into<String>, modifiers: Vec<Modifier>) -> Self {
        Self {
            key: key.into(),
            modifiers,
        }
    }

    /// Check if chord matches key event
    pub fn matches(&self, key: &Key, mods: Modifiers) -> bool {
        let key_match = match (key, self.key.as_str()) {
            // space is a special case since keyboard_types represents it as " " but our
            // configuration uses the string "Space"
            (Key::Character(c), "Space") => c == " ",
            (Key::Character(c), k) => c == k,
            (Key::ArrowUp, "ArrowUp") => true,
            (Key::ArrowDown, "ArrowDown") => true,
            (Key::ArrowLeft, "ArrowLeft") => true,
            (Key::ArrowRight, "ArrowRight") => true,
            (Key::Enter, "Enter") => true,
            (Key::Tab, "Tab") => true,
            (Key::Escape, "Escape") => true,
            (Key::Backspace, "Backspace") => true,
            (Key::Delete, "Delete") => true,
            (Key::Home, "Home") => true,
            (Key::End, "End") => true,
            (Key::PageUp, "PageUp") => true,
            (Key::PageDown, "PageDown") => true,
            (Key::F1, "F1") => true,
            (Key::F2, "F2") => true,
            (Key::F3, "F3") => true,
            (Key::F4, "F4") => true,
            (Key::F5, "F5") => true,
            (Key::F6, "F6") => true,
            (Key::F7, "F7") => true,
            (Key::F8, "F8") => true,
            (Key::F9, "F9") => true,
            (Key::F10, "F10") => true,
            (Key::F11, "F11") => true,
            (Key::F12, "F12") => true,
            _ => false,
        };

        if !key_match {
            return false;
        }

        // Check modifiers
        for modifier in &self.modifiers {
            let required = match modifier {
                Modifier::Ctrl => Modifiers::CONTROL,
                Modifier::Alt => Modifiers::ALT,
                Modifier::Shift => Modifiers::SHIFT,
                Modifier::Meta => Modifiers::META,
                Modifier::Super => Modifiers::SUPER,
                Modifier::Hyper => Modifiers::HYPER,
            };
            if !mods.contains(required) {
                return false;
            }
        }

        true
    }

    /// Format chord as string
    pub fn format(&self) -> String {
        let mut parts = Vec::new();
        for modifier in &self.modifiers {
            parts.push(match modifier {
                Modifier::Ctrl => "Ctrl",
                Modifier::Alt => "Alt",
                Modifier::Shift => "Shift",
                Modifier::Meta => "Meta",
                Modifier::Super => "Super",
                Modifier::Hyper => "Hyper",
            });
        }
        parts.push(&self.key);
        parts.join("+")
    }
}

/// Key action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyAction {
    /// Execute command
    Command(String),
    /// Insert text
    InsertText(String),
    /// Run macro
    Macro(Vec<KeySequence>),
    /// Toggle feature
    Toggle(String),
    /// Open panel
    OpenPanel(String),
    /// Focus element
    Focus(String),
    /// Custom action
    Custom(String, serde_json::Value),
}

/// Key binding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    pub id: String,
    pub sequence: KeySequence,
    pub action: KeyAction,
    pub context: String,
    pub description: String,
    pub enabled: bool,
    pub group: String,
    pub when: Option<String>,
}

/// Keymap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keymap {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub author: Option<String>,
    pub bindings: Vec<KeyBinding>,
    pub parent: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Keymap profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeymapProfile {
    pub name: String,
    pub active_keymap: String,
    pub overrides: Vec<KeyBinding>,
    pub disabled_bindings: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Keybinding manager
pub struct KeybindingManager {
    /// Available keymaps
    keymaps: Arc<RwLock<HashMap<String, Keymap>>>,
    /// Active keymap name
    active_keymap: Arc<RwLock<String>>,
    /// Keymap profiles
    profiles: Arc<RwLock<HashMap<String, KeymapProfile>>>,
    /// Active profile
    active_profile: Arc<RwLock<String>>,
    /// Context-specific bindings
    context_bindings: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Configuration
    config: CustomizationConfig,
    /// Key sequence timeout
    sequence_timeout: Arc<RwLock<std::time::Duration>>,
}

impl KeybindingManager {
    /// Create new keybinding manager
    pub async fn new(config: CustomizationConfig) -> Result<Self> {
        let manager = Self {
            keymaps: Arc::new(RwLock::new(HashMap::new())),
            active_keymap: Arc::new(RwLock::new(config.default_keymap.clone())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            active_profile: Arc::new(RwLock::new("default".to_string())),
            context_bindings: Arc::new(RwLock::new(HashMap::new())),
            config: config.clone(),
            sequence_timeout: Arc::new(RwLock::new(std::time::Duration::from_millis(500))),
        };

        // Load default keymap
        manager.load_default_keymap().await?;

        // Scan keymaps directory
        manager.scan_keymaps().await?;

        Ok(manager)
    }

    /// Load default keymap
    async fn load_default_keymap(&self) -> Result<()> {
        let keymap = Keymap {
            name: "default".to_string(),
            description: Some("Default keymap for Parsec IDE".to_string()),
            version: env!("CARGO_PKG_VERSION").to_string(),
            author: Some("Parsec Team".to_string()),
            bindings: self.create_default_bindings(),
            parent: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.keymaps.write().await.insert("default".to_string(), keymap);
        Ok(())
    }

    /// Create default keybindings
    fn create_default_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                id: "file.save".to_string(),
                sequence: KeySequence::single(Chord::new("s", vec![Modifier::Ctrl])),
                action: KeyAction::Command("workbench.action.files.save".to_string()),
                context: "editor".to_string(),
                description: "Save file".to_string(),
                enabled: true,
                group: "File".to_string(),
                when: None,
            },
            KeyBinding {
                id: "file.open".to_string(),
                sequence: KeySequence::single(Chord::new("o", vec![Modifier::Ctrl])),
                action: KeyAction::Command("workbench.action.files.open".to_string()),
                context: "global".to_string(),
                description: "Open file".to_string(),
                enabled: true,
                group: "File".to_string(),
                when: None,
            },
            KeyBinding {
                id: "file.new".to_string(),
                sequence: KeySequence::single(Chord::new("n", vec![Modifier::Ctrl])),
                action: KeyAction::Command("workbench.action.files.new".to_string()),
                context: "global".to_string(),
                description: "New file".to_string(),
                enabled: true,
                group: "File".to_string(),
                when: None,
            },
            KeyBinding {
                id: "edit.copy".to_string(),
                sequence: KeySequence::single(Chord::new("c", vec![Modifier::Ctrl])),
                action: KeyAction::Command("editor.action.clipboardCopy".to_string()),
                context: "editor".to_string(),
                description: "Copy".to_string(),
                enabled: true,
                group: "Edit".to_string(),
                when: None,
            },
            KeyBinding {
                id: "edit.cut".to_string(),
                sequence: KeySequence::single(Chord::new("x", vec![Modifier::Ctrl])),
                action: KeyAction::Command("editor.action.clipboardCut".to_string()),
                context: "editor".to_string(),
                description: "Cut".to_string(),
                enabled: true,
                group: "Edit".to_string(),
                when: None,
            },
            KeyBinding {
                id: "edit.paste".to_string(),
                sequence: KeySequence::single(Chord::new("v", vec![Modifier::Ctrl])),
                action: KeyAction::Command("editor.action.clipboardPaste".to_string()),
                context: "editor".to_string(),
                description: "Paste".to_string(),
                enabled: true,
                group: "Edit".to_string(),
                when: None,
            },
            KeyBinding {
                id: "edit.undo".to_string(),
                sequence: KeySequence::single(Chord::new("z", vec![Modifier::Ctrl])),
                action: KeyAction::Command("undo".to_string()),
                context: "editor".to_string(),
                description: "Undo".to_string(),
                enabled: true,
                group: "Edit".to_string(),
                when: None,
            },
            KeyBinding {
                id: "edit.redo".to_string(),
                sequence: KeySequence::new(vec![
                    Chord::new("y", vec![Modifier::Ctrl]),
                ]),
                action: KeyAction::Command("redo".to_string()),
                context: "editor".to_string(),
                description: "Redo".to_string(),
                enabled: true,
                group: "Edit".to_string(),
                when: None,
            },
            KeyBinding {
                id: "view.toggleSidebar".to_string(),
                sequence: KeySequence::single(Chord::new("b", vec![Modifier::Ctrl])),
                action: KeyAction::Command("workbench.action.toggleSidebar".to_string()),
                context: "global".to_string(),
                description: "Toggle sidebar".to_string(),
                enabled: true,
                group: "View".to_string(),
                when: None,
            },
            KeyBinding {
                id: "view.toggleTerminal".to_string(),
                sequence: KeySequence::single(Chord::new("`", vec![Modifier::Ctrl])),
                action: KeyAction::Command("workbench.action.terminal.toggle".to_string()),
                context: "global".to_string(),
                description: "Toggle terminal".to_string(),
                enabled: true,
                group: "View".to_string(),
                when: None,
            },
            KeyBinding {
                id: "search.find".to_string(),
                sequence: KeySequence::single(Chord::new("f", vec![Modifier::Ctrl])),
                action: KeyAction::Command("actions.find".to_string()),
                context: "editor".to_string(),
                description: "Find".to_string(),
                enabled: true,
                group: "Search".to_string(),
                when: None,
            },
            KeyBinding {
                id: "search.replace".to_string(),
                sequence: KeySequence::single(Chord::new("h", vec![Modifier::Ctrl])),
                action: KeyAction::Command("editor.action.startFindReplace".to_string()),
                context: "editor".to_string(),
                description: "Replace".to_string(),
                enabled: true,
                group: "Search".to_string(),
                when: None,
            },
            KeyBinding {
                id: "debug.start".to_string(),
                sequence: KeySequence::single(Chord::new("F5", vec![])),
                action: KeyAction::Command("workbench.action.debug.start".to_string()),
                context: "debug".to_string(),
                description: "Start debugging".to_string(),
                enabled: true,
                group: "Debug".to_string(),
                when: None,
            },
            KeyBinding {
                id: "debug.stop".to_string(),
                sequence: KeySequence::single(Chord::new("F5", vec![Modifier::Shift])),
                action: KeyAction::Command("workbench.action.debug.stop".to_string()),
                context: "debug".to_string(),
                description: "Stop debugging".to_string(),
                enabled: true,
                group: "Debug".to_string(),
                when: None,
            },
            KeyBinding {
                id: "terminal.new".to_string(),
                sequence: KeySequence::single(Chord::new("n", vec![Modifier::Ctrl, Modifier::Shift])),
                action: KeyAction::Command("workbench.action.terminal.new".to_string()),
                context: "terminal".to_string(),
                description: "New terminal".to_string(),
                enabled: true,
                group: "Terminal".to_string(),
                when: None,
            },
        ]
    }

    /// Scan keymaps directory
    async fn scan_keymaps(&self) -> Result<()> {
        let mut read_dir = fs::read_dir(&self.config.keymaps_dir).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(keymap) = serde_json::from_str::<Keymap>(&content) {
                        self.keymaps.write().await.insert(keymap.name.clone(), keymap);
                    }
                }
            }
        }

        Ok(())
    }

    /// Add keymap
    pub async fn add_keymap(&self, keymap: Keymap) -> Result<()> {
        let name = keymap.name.clone();
        self.keymaps.write().await.insert(name.clone(), keymap);
        
        // Save to disk
        let path = self.config.keymaps_dir.join(format!("{}.json", name));
        let json = serde_json::to_string_pretty(&self.keymaps.read().await.get(&name).unwrap())?;
        fs::write(path, json).await?;

        Ok(())
    }

    /// Remove keymap
    pub async fn remove_keymap(&self, name: &str) -> Result<()> {
        self.keymaps.write().await.remove(name);
        
        let path = self.config.keymaps_dir.join(format!("{}.json", name));
        if path.exists() {
            fs::remove_file(path).await?;
        }

        Ok(())
    }

    /// Set active keymap
    pub async fn set_active_keymap(&self, name: &str) -> Result<()> {
        if !self.keymaps.read().await.contains_key(name) {
            return Err(CustomizationError::KeybindingError(format!("Keymap not found: {}", name)));
        }

        *self.active_keymap.write().await = name.to_string();
        
        // Rebuild context index
        self.build_context_index().await;

        Ok(())
    }

    /// Get active keymap
    pub async fn active_keymap(&self) -> Option<Keymap> {
        let name = self.active_keymap.read().await.clone();
        self.keymaps.read().await.get(&name).cloned()
    }

    /// List keymaps
    pub async fn list_keymaps(&self) -> Vec<Keymap> {
        self.keymaps.read().await.values().cloned().collect()
    }

    /// Get keymap by name
    pub async fn get_keymap(&self, name: &str) -> Option<Keymap> {
        self.keymaps.read().await.get(name).cloned()
    }

    /// Build context index for faster lookups
    async fn build_context_index(&self) {
        let mut index: HashMap<String, Vec<String>> = HashMap::new();
        let active_name = self.active_keymap.read().await.clone();
        
        if let Some(keymap) = self.keymaps.read().await.get(&active_name) {
            for binding in &keymap.bindings {
                index.entry(binding.context.clone())
                    .or_insert_with(Vec::new)
                    .push(binding.id.clone());
            }
        }

        *self.context_bindings.write().await = index;
    }

    /// Get binding for key sequence
    pub async fn get_binding(&self, key: &Key, modifiers: Modifiers) -> Option<KeyBinding> {
        let active_name = self.active_keymap.read().await.clone();
        // keep the lock guard alive while we clone the keymap so that the returned value
        // does not borrow from the temporary guard
        let keymap = {
            let guard = self.keymaps.read().await;
            guard.get(&active_name)?.clone()
        };

        // Create chord from key event
        let key_str = match key {
            Key::Character(c) => {
                if c == " " {
                    "Space".to_string()
                } else {
                    c.clone()
                }
            }
            Key::ArrowUp => "ArrowUp".to_string(),
            Key::ArrowDown => "ArrowDown".to_string(),
            Key::ArrowLeft => "ArrowLeft".to_string(),
            Key::ArrowRight => "ArrowRight".to_string(),
            Key::Enter => "Enter".to_string(),
            Key::Tab => "Tab".to_string(),
            Key::Escape => "Escape".to_string(),
            Key::Backspace => "Backspace".to_string(),
            Key::Delete => "Delete".to_string(),
            Key::Home => "Home".to_string(),
            Key::End => "End".to_string(),
            Key::PageUp => "PageUp".to_string(),
            Key::PageDown => "PageDown".to_string(),
            Key::F1 => "F1".to_string(),
            Key::F2 => "F2".to_string(),
            Key::F3 => "F3".to_string(),
            Key::F4 => "F4".to_string(),
            Key::F5 => "F5".to_string(),
            Key::F6 => "F6".to_string(),
            Key::F7 => "F7".to_string(),
            Key::F8 => "F8".to_string(),
            Key::F9 => "F9".to_string(),
            Key::F10 => "F10".to_string(),
            Key::F11 => "F11".to_string(),
            Key::F12 => "F12".to_string(),
            _ => return None,
        };

        let mut modifier_vec = Vec::new();
        if modifiers.contains(Modifiers::CONTROL) {
            modifier_vec.push(Modifier::Ctrl);
        }
        if modifiers.contains(Modifiers::ALT) {
            modifier_vec.push(Modifier::Alt);
        }
        if modifiers.contains(Modifiers::SHIFT) {
            modifier_vec.push(Modifier::Shift);
        }
        if modifiers.contains(Modifiers::META) {
            modifier_vec.push(Modifier::Meta);
        }
        if modifiers.contains(Modifiers::SUPER) {
            modifier_vec.push(Modifier::Super);
        }
        if modifiers.contains(Modifiers::HYPER) {
            modifier_vec.push(Modifier::Hyper);
        }

        let chord = Chord::new(key_str, modifier_vec);

        // Find matching binding
        for binding in &keymap.bindings {
            if !binding.enabled {
                continue;
            }

            if binding.sequence.keys.len() == 1 {
                if binding.sequence.keys[0] == chord {
                    return Some(binding.clone());
                }
            }
            // Multi-key sequences would need state tracking
        }

        None
    }

    /// Create profile
    pub async fn create_profile(&self, name: &str, keymap_name: &str) -> Result<KeymapProfile> {
        let profile = KeymapProfile {
            name: name.to_string(),
            active_keymap: keymap_name.to_string(),
            overrides: Vec::new(),
            disabled_bindings: Vec::new(),
            created_at: chrono::Utc::now(),
        };

        self.profiles.write().await.insert(name.to_string(), profile.clone());
        Ok(profile)
    }

    /// Set active profile
    pub async fn set_active_profile(&self, name: &str) -> Result<()> {
        let profiles = self.profiles.read().await;
        if let Some(profile) = profiles.get(name) {
            self.set_active_keymap(&profile.active_keymap).await?;
            *self.active_profile.write().await = name.to_string();
            Ok(())
        } else {
            Err(CustomizationError::KeybindingError(format!("Profile not found: {}", name)))
        }
    }

    /// Override binding in current profile
    pub async fn override_binding(&self, binding: KeyBinding) -> Result<()> {
        let profile_name = self.active_profile.read().await.clone();
        let mut profiles = self.profiles.write().await;
        
        if let Some(profile) = profiles.get_mut(&profile_name) {
            profile.overrides.push(binding);
        }

        Ok(())
    }

    /// Disable binding in current profile
    pub async fn disable_binding(&self, binding_id: &str) -> Result<()> {
        let profile_name = self.active_profile.read().await.clone();
        let mut profiles = self.profiles.write().await;
        
        if let Some(profile) = profiles.get_mut(&profile_name) {
            if !profile.disabled_bindings.contains(&binding_id.to_string()) {
                profile.disabled_bindings.push(binding_id.to_string());
            }
        }

        Ok(())
    }

    /// Enable binding in current profile
    pub async fn enable_binding(&self, binding_id: &str) -> Result<()> {
        let profile_name = self.active_profile.read().await.clone();
        let mut profiles = self.profiles.write().await;
        
        if let Some(profile) = profiles.get_mut(&profile_name) {
            profile.disabled_bindings.retain(|id| id != binding_id);
        }

        Ok(())
    }

    /// Export keymap
    pub async fn export_keymap(&self, name: &str, format: &str) -> Result<String> {
        let keymap = self.keymaps.read().await.get(name)
            .ok_or_else(|| CustomizationError::KeybindingError(format!("Keymap not found: {}", name)))?
            .clone();

        match format {
            "json" => Ok(serde_json::to_string_pretty(&keymap)?),
            "yaml" => Ok(serde_yaml::to_string(&keymap)?),
            _ => Err(CustomizationError::KeybindingError(format!("Unsupported format: {}", format))),
        }
    }

    /// Import keymap
    /// Import keymap directly from a `Keymap` object
    pub async fn import_keymap(&self, keymap: &Keymap) -> Result<()> {
        // simply add a clone of the provided keymap
        self.add_keymap(keymap.clone()).await?;
        Ok(())
    }

    /// Check for conflicts
    pub async fn check_conflicts(&self, keymap_name: &str) -> Result<Vec<Conflict>> {
        let keymap = self.keymaps.read().await.get(keymap_name)
            .ok_or_else(|| CustomizationError::KeybindingError(format!("Keymap not found: {}", keymap_name)))?
            .clone();

        let mut conflicts = Vec::new();

        for (i, binding1) in keymap.bindings.iter().enumerate() {
            for binding2 in keymap.bindings.iter().skip(i + 1) {
                if binding1.sequence == binding2.sequence {
                    conflicts.push(Conflict {
                        binding1: binding1.id.clone(),
                        binding2: binding2.id.clone(),
                        sequence: binding1.sequence.clone(),
                    });
                }
            }
        }

        Ok(conflicts)
    }
}

/// Keybinding conflict
#[derive(Debug, Clone)]
pub struct Conflict {
    pub binding1: String,
    pub binding2: String,
    pub sequence: KeySequence,
}