//! Terminal emulator module
//!
//! Provides a full-featured terminal emulator for the Parsec IDE,
//! with support for multiple terminals, split views, and shell integration.

mod pty;
mod buffer;
mod renderer;

pub use pty::*;
pub use buffer::*;
pub use renderer::*;

use std::sync::Arc;
use anyhow::Result;
use tokio::sync::{mpsc, RwLock};
use std::collections::HashMap;

/// Main terminal emulator instance
#[derive(Debug)]
pub struct Terminal {
    /// Terminal ID
    id: String,
    /// Terminal name
    name: String,
    /// PTY process (shell)
    pty: Option<Arc<RwLock<PtyProcess>>>,
    /// Terminal buffer (scrollback + screen)
    buffer: Arc<RwLock<TerminalBuffer>>,
    /// Terminal renderer
    renderer: Arc<RwLock<TerminalRenderer>>,
    /// Terminal size
    size: TerminalSize,
    /// Terminal state
    state: TerminalState,
    /// Command history
    history: CommandHistory,
    /// Current working directory
    cwd: Option<String>,
    /// Environment variables
    env: HashMap<String, String>,
    /// Configuration
    config: TerminalConfig,
    /// Event sender for terminal events
    event_tx: mpsc::UnboundedSender<TerminalEvent>,
}

/// Terminal size in rows and columns
#[derive(Debug, Clone, Copy)]
pub struct TerminalSize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: Option<u16>,
    pub pixel_height: Option<u16>,
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self {
            rows: 24,
            cols: 80,
            pixel_width: None,
            pixel_height: None,
        }
    }
}

/// Terminal state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalState {
    Idle,
    Running,
    Waiting,
    Busy,
    Closed,
    Error,
}

/// Terminal configuration
#[derive(Debug, Clone)]
pub struct TerminalConfig {
    pub shell_path: Option<String>,
    pub shell_args: Vec<String>,
    pub working_dir: Option<String>,
    pub env: HashMap<String, String>,
    pub scrollback_lines: usize,
    pub cursor_style: CursorStyle,
    pub cursor_blink: bool,
    pub bell_style: BellStyle,
    pub mouse_support: bool,
    pub bracketed_paste: bool,
    pub alt_screen: bool,
    pub true_color: bool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            shell_path: None,
            shell_args: Vec::new(),
            working_dir: None,
            env: HashMap::new(),
            scrollback_lines: 10000,
            cursor_style: CursorStyle::Block,
            cursor_blink: true,
            bell_style: BellStyle::Visual,
            mouse_support: true,
            bracketed_paste: true,
            alt_screen: true,
            true_color: true,
        }
    }
}

/// Cursor style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

/// Bell style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BellStyle {
    None,
    Visual,
    Audible,
}

/// Terminal event
#[derive(Debug, Clone)]
pub enum TerminalEvent {
    Output(Vec<u8>),
    Input(Vec<u8>),
    Resized(TerminalSize),
    TitleChanged(String),
    Bell,
    Closed,
    Error(String),
}

/// Command history
#[derive(Debug, Clone)]
pub struct CommandHistory {
    entries: Vec<String>,
    current_index: Option<usize>,
    temp_buffer: Option<String>,
    max_entries: usize,
}

impl CommandHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::with_capacity(max_entries),
            current_index: None,
            temp_buffer: None,
            max_entries,
        }
    }

    pub fn add(&mut self, command: String) {
        if command.trim().is_empty() {
            return;
        }
        if self.entries.last() == Some(&command) {
            return; // Don't duplicate last command
        }
        self.entries.push(command);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
        self.current_index = None;
        self.temp_buffer = None;
    }

    pub fn previous(&mut self, current: &str) -> Option<String> {
        match self.current_index {
            None => {
                if !self.entries.is_empty() {
                    self.current_index = Some(self.entries.len() - 1);
                    self.temp_buffer = Some(current.to_string());
                    self.entries.last().cloned()
                } else {
                    None
                }
            }
            Some(idx) if idx > 0 => {
                self.current_index = Some(idx - 1);
                Some(self.entries[idx - 1].clone())
            }
            _ => None,
        }
    }

    pub fn next(&mut self) -> Option<String> {
        match self.current_index {
            Some(idx) if idx < self.entries.len() - 1 => {
                self.current_index = Some(idx + 1);
                Some(self.entries[idx + 1].clone())
            }
            Some(_) => {
                self.current_index = None;
                self.temp_buffer.take()
            }
            None => None,
        }
    }

    pub fn reset(&mut self) {
        self.current_index = None;
        self.temp_buffer = None;
    }
}

impl Terminal {
    /// Create a new terminal instance
    pub fn new(id: String, name: String, config: TerminalConfig) -> Self {
        let (event_tx, _) = mpsc::unbounded_channel();
        
        Self {
            id,
            name,
            pty: None,
            buffer: Arc::new(RwLock::new(TerminalBuffer::new(config.scrollback_lines))),
            renderer: Arc::new(RwLock::new(TerminalRenderer::new())),
            size: TerminalSize::default(),
            state: TerminalState::Idle,
            history: CommandHistory::new(1000),
            cwd: config.working_dir.clone(),
            env: config.env.clone(),
            config,
            event_tx,
        }
    }

    /// Initialize terminal with a shell process
    pub async fn init(&mut self) -> Result<()> {
        let shell_path = self.config.shell_path.clone()
            .unwrap_or_else(|| Self::default_shell());
        
        let pty = PtyProcess::new(
            shell_path,
            self.config.shell_args.clone(),
            self.config.working_dir.clone(),
            self.config.env.clone(),
        )?;
        
        // Set initial size
        pty.resize(self.size.rows, self.size.cols)?;
        
        self.pty = Some(Arc::new(RwLock::new(pty)));
        self.state = TerminalState::Idle;
        
        Ok(())
    }

    /// Get default shell for platform
    fn default_shell() -> String {
        if cfg!(windows) {
            "powershell.exe".to_string()
        } else {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
        }
    }

    /// Write data to terminal
    pub async fn write(&self, data: &[u8]) -> Result<()> {
        if let Some(pty) = &self.pty {
            let pty = pty.write().await;
            pty.write(data)?;
        }
        Ok(())
    }

    /// Write string to terminal
    pub async fn write_str(&self, s: &str) -> Result<()> {
        self.write(s.as_bytes()).await
    }

    /// Read output from terminal
    pub async fn read(&self) -> Option<Vec<u8>> {
        if let Some(pty) = &self.pty {
            let mut pty = pty.write().await;
            pty.read().await
        } else {
            None
        }
    }

    /// Resize terminal
    pub async fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.size = TerminalSize {
            rows,
            cols,
            ..self.size
        };
        
        if let Some(pty) = &self.pty {
            let pty = pty.write().await;
            pty.resize(rows, cols)?;
        }
        
        let mut buffer = self.buffer.write().await;
        buffer.resize(rows, cols);
        
        Ok(())
    }

    /// Get terminal size
    pub fn size(&self) -> TerminalSize {
        self.size
    }

    /// Get terminal state
    pub fn state(&self) -> TerminalState {
        self.state
    }

    /// Check if terminal is alive
    pub fn is_alive(&self) -> bool {
        self.state != TerminalState::Closed
    }

    /// Close terminal
    pub async fn close(&mut self) -> Result<()> {
        if let Some(pty) = self.pty.take() {
            let mut pty = pty.write().await;
            pty.kill().await?;
        }
        
        self.state = TerminalState::Closed;
        Ok(())
    }

    /// Send Ctrl+C
    pub async fn send_sigint(&self) -> Result<()> {
        self.write(&[0x03]).await // ETX
    }

    /// Send Ctrl+Z
    pub async fn send_sigstop(&self) -> Result<()> {
        self.write(&[0x1A]).await // SUB
    }

    /// Clear terminal screen
    pub async fn clear(&self) {
        let mut buffer = self.buffer.write().await;
        buffer.clear();
    }

    /// Clear scrollback
    pub async fn clear_scrollback(&self) {
        let mut buffer = self.buffer.write().await;
        buffer.clear_scrollback();
    }

    /// Get current line (for command extraction)
    pub fn current_line(&self) -> Option<String> {
        // Implementation would get current line from buffer
        None
    }

    /// Add command to history
    pub fn add_to_history(&mut self, command: String) {
        self.history.add(command);
    }

    /// Get previous command from history
    pub fn previous_command(&mut self, current: &str) -> Option<String> {
        self.history.previous(current)
    }

    /// Get next command from history
    pub fn next_command(&mut self) -> Option<String> {
        self.history.next()
    }

    /// Reset history navigation
    pub fn reset_history(&mut self) {
        self.history.reset();
    }

    /// Get terminal buffer for rendering
    pub async fn buffer(&self) -> Arc<RwLock<TerminalBuffer>> {
        self.buffer.clone()
    }

    /// Get terminal renderer
    pub async fn renderer(&self) -> Arc<RwLock<TerminalRenderer>> {
        self.renderer.clone()
    }

    /// Get terminal ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get terminal name
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Terminal manager for multiple terminals
pub struct TerminalManager {
    terminals: HashMap<String, Arc<RwLock<Terminal>>>,
    active: Option<String>,
    next_id: usize,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            terminals: HashMap::new(),
            active: None,
            next_id: 1,
        }
    }

    /// Create a new terminal
    pub async fn create(&mut self, name: Option<String>, config: Option<TerminalConfig>) -> Result<String> {
        let id = format!("term-{}", self.next_id);
        self.next_id += 1;
        
        let name = name.unwrap_or_else(|| format!("Terminal {}", self.next_id - 1));
        let config = config.unwrap_or_default();
        
        let mut terminal = Terminal::new(id.clone(), name, config);
        terminal.init().await?;
        
        self.terminals.insert(id.clone(), Arc::new(RwLock::new(terminal)));
        self.active = Some(id.clone());
        
        Ok(id)
    }

    /// Get terminal by ID
    pub fn get(&self, id: &str) -> Option<Arc<RwLock<Terminal>>> {
        self.terminals.get(id).cloned()
    }

    /// Get active terminal
    pub fn active(&self) -> Option<Arc<RwLock<Terminal>>> {
        self.active.as_ref().and_then(|id| self.get(id))
    }

    /// Set active terminal
    pub fn set_active(&mut self, id: &str) -> bool {
        if self.terminals.contains_key(id) {
            self.active = Some(id.to_string());
            true
        } else {
            false
        }
    }

    /// Close terminal
    pub async fn close(&mut self, id: &str) -> Result<()> {
        if let Some(terminal) = self.terminals.remove(id) {
            let mut terminal = terminal.write().await;
            terminal.close().await?;
            
            if self.active.as_deref() == Some(id) {
                self.active = self.terminals.keys().next().cloned();
            }
        }
        Ok(())
    }

    /// List all terminals
    pub fn list(&self) -> Vec<(String, String, TerminalState)> {
        self.terminals
            .iter()
            .map(|(id, term)| {
                let term = term.blocking_read();
                (id.clone(), term.name().to_string(), term.state())
            })
            .collect()
    }

    /// Resize all terminals
    pub async fn resize_all(&self, rows: u16, cols: u16) {
        for terminal in self.terminals.values() {
            let mut terminal = terminal.write().await;
            let _ = terminal.resize(rows, cols).await;
        }
    }
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_terminal_creation() {
        let mut manager = TerminalManager::new();
        let id = manager.create(None, None).await.unwrap();
        assert!(manager.get(&id).is_some());
    }

    #[test]
fn test_command_history() {
        let mut history = CommandHistory::new(5);
        history.add("ls".to_string());
        history.add("cd ..".to_string());
        
        assert_eq!(history.previous(""), Some("cd ..".to_string()));
        assert_eq!(history.previous(""), Some("ls".to_string()));
        assert_eq!(history.next(), Some("cd ..".to_string()));
        assert_eq!(history.next(), None);
    }
}