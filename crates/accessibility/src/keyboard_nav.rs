#![cfg(feature = "keyboard-nav")]
// file is compiled only when the keyboard-nav feature is enabled

//! Keyboard navigation system

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use keyboard_types::{Key, Modifiers, Code};
use tracing::info;
use chrono;

use crate::{Result, AccessibilityError, AccessibilityConfig};

/// Navigation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NavigationMode {
    /// Standard navigation
    Standard,
    /// Enhanced navigation with shortcuts
    Enhanced,
    /// Full keyboard navigation (no mouse)
    FullKeyboard,
    /// Reduced motion navigation
    ReducedMotion,
}

/// Focus indicator style
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusIndicator {
    pub enabled: bool,
    pub style: FocusStyle,
    pub color: String,
    pub width: u32,
    pub animation: bool,
    pub sound: bool,
}

/// Focus style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FocusStyle {
    Outline,
    Background,
    Underline,
    Highlight,
    Invert,
    Custom(String),
}

/// Key binding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    pub id: String,
    pub key: Key,
    pub modifiers: Modifiers,
    pub command: String,
    pub context: String,
    pub description: String,
    pub enabled: bool,
}

/// Navigation history entry
#[derive(Debug, Clone)]
pub struct NavigationEntry {
    pub element_id: String,
    pub element_type: String,
    pub path: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Shortcut manager
pub struct ShortcutManager {
    /// Key bindings
    bindings: Arc<RwLock<HashMap<String, KeyBinding>>>,
    /// Context bindings
    context_bindings: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Current context
    current_context: Arc<RwLock<String>>,
    /// Configuration
    config: AccessibilityConfig,
}

impl ShortcutManager {
    /// Create new shortcut manager
    pub fn new() -> Self {
        Self {
            bindings: Arc::new(RwLock::new(HashMap::new())),
            context_bindings: Arc::new(RwLock::new(HashMap::new())),
            current_context: Arc::new(RwLock::new("global".to_string())),
            config: AccessibilityConfig::default(),
        }
    }

    /// Add key binding
    pub async fn add_binding(&self, binding: KeyBinding) {
        let id = binding.id.clone();
        
        // Store binding
        self.bindings.write().await.insert(id.clone(), binding.clone());
        
        // Index by context
        self.context_bindings.write().await
            .entry(binding.context)
            .or_insert_with(Vec::new)
            .push(id);
    }

    /// Remove key binding
    pub async fn remove_binding(&self, id: &str) {
        if let Some(binding) = self.bindings.write().await.remove(id) {
            // Remove from context index
            if let Some(mut bindings) = self.context_bindings.write().await.get_mut(&binding.context) {
                bindings.retain(|b| b != id);
            }
        }
    }

    /// Set current context
    pub async fn set_context(&self, context: &str) {
        *self.current_context.write().await = context.to_string();
    }

    /// Get binding for key
    pub async fn get_binding(&self, key: Key, modifiers: Modifiers) -> Option<KeyBinding> {
        let context = self.current_context.read().await.clone();
        let bindings = self.bindings.read().await;

        // Try exact context first
        for binding in bindings.values() {
            if binding.context == context && 
               binding.key == key && 
               binding.modifiers == modifiers &&
               binding.enabled {
                return Some(binding.clone());
            }
        }

        // Try global context
        for binding in bindings.values() {
            if binding.context == "global" && 
               binding.key == key && 
               binding.modifiers == modifiers &&
               binding.enabled {
                return Some(binding.clone());
            }
        }

        None
    }

    /// List bindings for current context
    pub async fn current_bindings(&self) -> Vec<KeyBinding> {
        let context = self.current_context.read().await.clone();
        let bindings = self.bindings.read().await;
        let context_bindings = self.context_bindings.read().await;

        let mut result = Vec::new();
        if let Some(ids) = context_bindings.get(&context) {
            for id in ids {
                if let Some(binding) = bindings.get(id) {
                    result.push(binding.clone());
                }
            }
        }

        result
    }
}

impl Default for ShortcutManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Navigation history
pub struct NavigationHistory {
    /// History entries
    entries: Arc<RwLock<VecDeque<NavigationEntry>>>,
    /// Current index
    current_index: Arc<RwLock<usize>>,
    /// Max history size
    max_size: usize,
}

impl NavigationHistory {
    /// Create new navigation history
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(max_size))),
            current_index: Arc::new(RwLock::new(0)),
            max_size,
        }
    }

    /// Add navigation entry
    pub async fn add_entry(&self, entry: NavigationEntry) {
        let mut entries = self.entries.write().await;
        
        // Remove entries after current index
        let current = *self.current_index.read().await;
        while entries.len() > current + 1 {
            entries.pop_back();
        }

        entries.push_back(entry);
        
        if entries.len() > self.max_size {
            entries.pop_front();
        }

        *self.current_index.write().await = entries.len() - 1;
    }

    /// Go back
    pub async fn back(&self) -> Option<NavigationEntry> {
        let mut current = self.current_index.write().await;
        if *current > 0 {
            *current -= 1;
            let entries = self.entries.read().await;
            entries.get(*current).cloned()
        } else {
            None
        }
    }

    /// Go forward
    pub async fn forward(&self) -> Option<NavigationEntry> {
        let mut current = self.current_index.write().await;
        let entries = self.entries.read().await;
        if *current + 1 < entries.len() {
            *current += 1;
            entries.get(*current).cloned()
        } else {
            None
        }
    }

    /// Get current entry
    pub async fn current(&self) -> Option<NavigationEntry> {
        let entries = self.entries.read().await;
        let current = *self.current_index.read().await;
        entries.get(current).cloned()
    }

    /// Clear history
    pub async fn clear(&self) {
        self.entries.write().await.clear();
        *self.current_index.write().await = 0;
    }
}

/// Keyboard navigation
pub struct KeyboardNavigation {
    /// Is enabled
    enabled: Arc<RwLock<bool>>,
    /// Navigation mode
    mode: Arc<RwLock<NavigationMode>>,
    /// Focus indicator
    focus_indicator: Arc<RwLock<FocusIndicator>>,
    /// Shortcut manager
    shortcuts: Arc<ShortcutManager>,
    /// Navigation history
    history: Arc<NavigationHistory>,
    /// Currently focused element
    focused_element: Arc<RwLock<Option<String>>>,
    /// Focusable elements
    focusable_elements: Arc<RwLock<HashMap<String, FocusableElement>>>,
    /// Configuration
    config: AccessibilityConfig,
}

/// Focusable element
#[derive(Debug, Clone)]
pub struct FocusableElement {
    pub id: String,
    pub element_type: String,
    pub label: String,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub enabled: bool,
    pub visible: bool,
    pub bounds: Option<(i32, i32, i32, i32)>,
}

impl KeyboardNavigation {
    /// Create new keyboard navigation
    pub async fn new(config: AccessibilityConfig) -> Result<Self> {
        let shortcuts = ShortcutManager::new();
        let history = NavigationHistory::new(100);

        let nav = Self {
            enabled: Arc::new(RwLock::new(true)),
            mode: Arc::new(RwLock::new(config.default_navigation_mode)),
            focus_indicator: Arc::new(RwLock::new(FocusIndicator {
                enabled: true,
                style: FocusStyle::Outline,
                color: "#007acc".to_string(),
                width: 2,
                animation: true,
                sound: true,
            })),
            shortcuts: Arc::new(shortcuts),
            history: Arc::new(history),
            focused_element: Arc::new(RwLock::new(None)),
            focusable_elements: Arc::new(RwLock::new(HashMap::new())),
            config,
        };

        // Register default shortcuts
        nav.register_default_shortcuts().await;

        Ok(nav)
    }

    /// Register default shortcuts
    async fn register_default_shortcuts(&self) {
        let shortcuts = [
            ("tab", "Navigate forward", Key::Tab, Modifiers::empty(), "next"),
            ("shift-tab", "Navigate backward", Key::Tab, Modifiers::SHIFT, "prev"),
            ("enter", "Activate", Key::Enter, Modifiers::empty(), "activate"),
            ("space", "Toggle", Key::Space, Modifiers::empty(), "toggle"),
            ("escape", "Cancel", Key::Escape, Modifiers::empty(), "cancel"),
            ("arrow-up", "Move up", Key::ArrowUp, Modifiers::empty(), "up"),
            ("arrow-down", "Move down", Key::ArrowDown, Modifiers::empty(), "down"),
            ("arrow-left", "Move left", Key::ArrowLeft, Modifiers::empty(), "left"),
            ("arrow-right", "Move right", Key::ArrowRight, Modifiers::empty(), "right"),
            ("home", "Go to start", Key::Home, Modifiers::empty(), "home"),
            ("end", "Go to end", Key::End, Modifiers::empty(), "end"),
            ("page-up", "Page up", Key::PageUp, Modifiers::empty(), "page-up"),
            ("page-down", "Page down", Key::PageDown, Modifiers::empty(), "page-down"),
            ("ctrl-home", "Go to first", Key::Home, Modifiers::CONTROL, "first"),
            ("ctrl-end", "Go to last", Key::End, Modifiers::CONTROL, "last"),
        ];

        for (id, desc, key, modifiers, cmd) in shortcuts {
            self.shortcuts.add_binding(KeyBinding {
                id: id.to_string(),
                key,
                modifiers,
                command: cmd.to_string(),
                context: "global".to_string(),
                description: desc.to_string(),
                enabled: true,
            }).await;
        }
    }

    /// Enable navigation
    pub async fn enable(&self) {
        *self.enabled.write().await = true;
    }

    /// Disable navigation
    pub async fn disable(&self) {
        *self.enabled.write().await = false;
    }

    /// Check if enabled
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// Set navigation mode
    pub async fn set_mode(&self, mode: NavigationMode) {
        *self.mode.write().await = mode;
    }

    /// Get navigation mode
    pub async fn mode(&self) -> NavigationMode {
        *self.mode.read().await
    }

    /// Set focus indicator
    pub async fn set_focus_indicator(&self, indicator: FocusIndicator) {
        *self.focus_indicator.write().await = indicator;
    }

    /// Get focus indicator
    pub async fn focus_indicator(&self) -> FocusIndicator {
        self.focus_indicator.read().await.clone()
    }

    /// Register focusable element
    pub async fn register_element(&self, element: FocusableElement) {
        self.focusable_elements.write().await.insert(element.id.clone(), element);
    }

    /// Unregister element
    pub async fn unregister_element(&self, id: &str) {
        self.focusable_elements.write().await.remove(id);
    }

    /// Focus element
    pub async fn focus(&self, id: &str) -> Result<()> {
        let elements = self.focusable_elements.read().await;
        if let Some(element) = elements.get(id) {
            if element.enabled && element.visible {
                *self.focused_element.write().await = Some(id.to_string());

                // Add to history
                self.history.add_entry(NavigationEntry {
                    element_id: id.to_string(),
                    element_type: element.element_type.clone(),
                    path: id.to_string(),
                    timestamp: chrono::Utc::now(),
                }).await;

                // Play focus sound if enabled
                if self.focus_indicator.read().await.sound {
                    self.play_focus_sound().await?;
                }

                return Ok(());
            }
        }
        Err(AccessibilityError::KeyboardError(format!("Element not focusable: {}", id)))
    }

    /// Blur current element
    pub async fn blur(&self) {
        *self.focused_element.write().await = None;
    }

    /// Get focused element
    pub async fn focused(&self) -> Option<String> {
        self.focused_element.read().await.clone()
    }

    /// Navigate to next element
    pub async fn next(&self) -> Result<()> {
        let elements = self.focusable_elements.read().await;
        let current = self.focused_element.read().await.clone();

        // Get sorted list of focusable elements
        let mut focusable: Vec<_> = elements.values()
            .filter(|e| e.enabled && e.visible)
            .collect();

        // Sort by some order (tab index, position, etc.)
        focusable.sort_by(|a, b| a.id.cmp(&b.id));

        if focusable.is_empty() {
            return Ok(());
        }

        if let Some(current_id) = current {
            if let Some(pos) = focusable.iter().position(|e| e.id == current_id) {
                let next = (pos + 1) % focusable.len();
                self.focus(&focusable[next].id).await?;
            } else {
                self.focus(&focusable[0].id).await?;
            }
        } else {
            self.focus(&focusable[0].id).await?;
        }

        Ok(())
    }

    /// Navigate to previous element
    pub async fn prev(&self) -> Result<()> {
        let elements = self.focusable_elements.read().await;
        let current = self.focused_element.read().await.clone();

        let mut focusable: Vec<_> = elements.values()
            .filter(|e| e.enabled && e.visible)
            .collect();

        focusable.sort_by(|a, b| a.id.cmp(&b.id));

        if focusable.is_empty() {
            return Ok(());
        }

        if let Some(current_id) = current {
            if let Some(pos) = focusable.iter().position(|e| e.id == current_id) {
                let prev = if pos == 0 { focusable.len() - 1 } else { pos - 1 };
                self.focus(&focusable[prev].id).await?;
            } else {
                self.focus(&focusable[focusable.len() - 1].id).await?;
            }
        } else {
            self.focus(&focusable[focusable.len() - 1].id).await?;
        }

        Ok(())
    }

    /// Activate current element
    pub async fn activate(&self) -> Result<()> {
        if let Some(id) = self.focused().await {
            // Would trigger element's activation
            info!("Activating element: {}", id);
            Ok(())
        } else {
            Err(AccessibilityError::KeyboardError("No element focused".to_string()))
        }
    }

    /// Process key event
    pub async fn process_key(&self, key: Key, modifiers: Modifiers) -> Result<bool> {
        if !self.is_enabled().await {
            return Ok(false);
        }

        if let Some(binding) = self.shortcuts.get_binding(key, modifiers).await {
            match binding.command.as_str() {
                "next" => self.next().await?,
                "prev" => self.prev().await?,
                "activate" => self.activate().await?,
                _ => info!("Unknown command: {}", binding.command),
            }
            return Ok(true);
        }

        Ok(false)
    }

    /// Play focus sound
    async fn play_focus_sound(&self) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use rodio::{source::SineWave, Source, OutputStream};
            let (_stream, stream_handle) = OutputStream::try_default()
                .map_err(|_| AccessibilityError::KeyboardError("No audio output".to_string()))?;
            
            let source = SineWave::new(880.0)
                .take_duration(Duration::from_millis(50))
                .amplify(0.5);
            
            stream_handle.play_raw(source.convert_samples())?;
        }
        Ok(())
    }

    /// Get shortcut manager
    pub fn shortcuts(&self) -> Arc<ShortcutManager> {
        self.shortcuts.clone()
    }

    /// Get navigation history
    pub fn history(&self) -> Arc<NavigationHistory> {
        self.history.clone()
    }

    /// Get focusable elements
    pub async fn focusable_elements(&self) -> Vec<FocusableElement> {
        self.focusable_elements.read().await
            .values()
            .filter(|e| e.enabled && e.visible)
            .cloned()
            .collect()
    }
}