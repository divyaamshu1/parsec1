//! Terminal multiplexer for managing multiple terminal sessions

#![allow(dead_code, unused_variables)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, RwLock, broadcast};
use tokio::time;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use crate::{
    TerminalConfig, TerminalEvent, TerminalStats, TerminalHandle,
    PtyProcess, PtySize, PtyEvent, TerminalBuffer, TerminalRenderer,
    Result, TerminalError, SplitManager, SplitPane, PaneId,
    Theme, ThemeManager,
};

/// Terminal session ID
pub type SessionId = String;

/// Terminal session
pub struct TerminalSession {
    /// Session ID
    id: SessionId,
    /// Session name
    name: String,
    /// Terminal buffer
    buffer: Arc<RwLock<TerminalBuffer>>,
    /// PTY process
    pty: Arc<RwLock<Option<PtyProcess>>>,
    /// Terminal renderer
    renderer: Arc<RwLock<TerminalRenderer>>,
    /// Event sender
    event_tx: broadcast::Sender<TerminalEvent>,
    /// Event receiver
    event_rx: broadcast::Receiver<TerminalEvent>,
    /// Command sender
    cmd_tx: mpsc::UnboundedSender<TerminalCommand>,
    /// Command receiver
    cmd_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<TerminalCommand>>>>,
    /// Configuration
    config: TerminalConfig,
    /// Creation time
    created_at: std::time::Instant,
    /// Last activity time
    last_activity: Arc<RwLock<std::time::Instant>>,
    /// Bytes written
    bytes_written: Arc<RwLock<u64>>,
    /// Bytes read
    bytes_read: Arc<RwLock<u64>>,
    /// Split manager (if this session contains splits)
    split_manager: Arc<RwLock<Option<SplitManager>>>,
    /// Parent session (if this is a split)
    parent_id: Option<SessionId>,
}

/// Terminal command
#[derive(Debug)]
enum TerminalCommand {
    Write(Vec<u8>),
    Resize(u16, u16),
    Clear,
    ClearScrollback,
    SetScrollOffset(isize),
    Close,
}

impl TerminalSession {
    /// Create a new terminal session
    pub async fn new(
        id: SessionId,
        name: String,
        config: TerminalConfig,
    ) -> Result<Self> {
        let (event_tx, event_rx) = broadcast::channel(100);
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

        let buffer = Arc::new(RwLock::new(TerminalBuffer::new()));

        let renderer = Arc::new(RwLock::new(TerminalRenderer::new()));

        let pty = if config.shell.is_some() {
            let shell = config.shell.clone().unwrap_or_else(|| {
                if cfg!(windows) { "powershell.exe".to_string() } else { "/bin/bash".to_string() }
            });
            let args = if cfg!(windows) { vec![] } else { vec!["-l".to_string()] };
            
            match PtyProcess::new(
                shell,
                args,
                config.working_dir.clone(),
                config.env.clone(),
            ) {
                Ok(p) => {
                    let pty = Arc::new(RwLock::new(Some(p)));
                    let buffer_clone = buffer.clone();
                    let event_tx_clone = event_tx.clone();
                    let bytes_read_clone = Arc::clone(&Arc::new(RwLock::new(0u64)));
                    
                    // Spawn PTY reader (clone the Arc explicitly to avoid moving `pty` itself)
                    let pty_clone = pty.clone();
                    tokio::spawn(async move {
                        Self::pty_reader_loop(
                            pty_clone,
                            buffer_clone,
                            event_tx_clone,
                            bytes_read_clone,
                        ).await;
                    });
                    
                    pty
                }
                Err(e) => {
                    warn!("Failed to start PTY: {}", e);
                    Arc::new(RwLock::new(None))
                }
            }
        } else {
            Arc::new(RwLock::new(None))
        };

        let session = Self {
            id,
            name,
            buffer,
            pty,
            renderer,
            event_tx,
            event_rx,
            cmd_tx,
            cmd_rx: Arc::new(RwLock::new(Some(cmd_rx))),
            config,
            created_at: std::time::Instant::now(),
            last_activity: Arc::new(RwLock::new(std::time::Instant::now())),
            bytes_written: Arc::new(RwLock::new(0)),
            bytes_read: Arc::new(RwLock::new(0)),
            split_manager: Arc::new(RwLock::new(None)),
            parent_id: None,
        };

        // Spawn command processor
        let session_clone = session.clone();
        tokio::spawn(async move {
            session_clone.command_processor().await;
        });

        Ok(session)
    }

    /// PTY reader loop
    async fn pty_reader_loop(
        pty: Arc<RwLock<Option<PtyProcess>>>,
        buffer: Arc<RwLock<TerminalBuffer>>,
        event_tx: broadcast::Sender<TerminalEvent>,
        bytes_read: Arc<RwLock<u64>>,
    ) {
        let mut last_title = String::new();
        
        loop {
            let data = {
                let mut pty_guard = pty.write().await;
                if let Some(pty) = pty_guard.as_mut() {
                    match pty.read().await {
                        Some(data) => {
                            let mut bytes_guard = bytes_read.write().await;
                            *bytes_guard += data.len() as u64;
                            data
                        }
                        None => {
                            // PTY closed
                            let _ = event_tx.send(TerminalEvent::Data(b"exit\r\n".to_vec()));
                            break;
                        }
                    }
                } else {
                    time::sleep(Duration::from_millis(10)).await;
                    continue;
                }
            };

            // Write to buffer
            let mut buffer_guard = buffer.write().await;
            buffer_guard.write(&data);

            // Check for title changes
            let title = buffer_guard.title().to_string();
            if title != last_title {
                last_title = title.clone();
                let _ = event_tx.send(TerminalEvent::TitleChanged(title));
            }

            // Forward data
            let _ = event_tx.send(TerminalEvent::Data(data));

            // Check for bell
            if buffer_guard.bell_received() {
                let _ = event_tx.send(TerminalEvent::Bell);
                buffer_guard.clear_bell();
            }
        }
    }

    /// Command processor loop
    async fn command_processor(self) {
        let mut cmd_rx = {
            let mut guard = self.cmd_rx.write().await;
            guard.take().unwrap()
        };

        while let Some(cmd) = cmd_rx.recv().await {
            *self.last_activity.write().await = std::time::Instant::now();

            match cmd {
                TerminalCommand::Write(data) => {
                    if let Some(pty) = self.pty.write().await.as_mut() {
                        if pty.write(&data).is_ok() {
                            let mut bw = self.bytes_written.write().await;
                            *bw += data.len() as u64;
                        }
                    } else {
                        // No PTY, echo to buffer
                        let mut buf = self.buffer.write().await;
                        buf.write(&data);
                    }
                }
                TerminalCommand::Resize(rows, cols) => {
                    if let Some(pty) = self.pty.write().await.as_mut() {
                        let _ = pty.resize(rows, cols);
                    }
                    let mut buf = self.buffer.write().await;
                    buf.resize(rows, cols);
                }
                TerminalCommand::Clear => {
                    let mut buf = self.buffer.write().await;
                    buf.clear();
                }
                TerminalCommand::ClearScrollback => {
                    let mut buf = self.buffer.write().await;
                    buf.clear_scrollback();
                }
                TerminalCommand::SetScrollOffset(offset) => {
                    let mut buf = self.buffer.write().await;
                    buf.set_scroll_offset(offset);
                }
                TerminalCommand::Close => {
                    if let Some(mut pty) = self.pty.write().await.take() {
                        let _ = pty.kill().await;
                    }
                    break;
                }
            }
        }
    }

    /// Get session ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get session name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Write data to terminal
    pub async fn write(&self, data: &[u8]) -> Result<()> {
        self.cmd_tx.send(TerminalCommand::Write(data.to_vec()))?;
        Ok(())
    }

    /// Resize terminal
    pub async fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        if rows == 0 || cols == 0 {
            return Err(TerminalError::InvalidSize { rows, cols });
        }
        self.cmd_tx.send(TerminalCommand::Resize(rows, cols))?;
        Ok(())
    }

    /// Clear terminal
    pub async fn clear(&self) -> Result<()> {
        self.cmd_tx.send(TerminalCommand::Clear)?;
        Ok(())
    }

    /// Clear scrollback buffer
    pub async fn clear_scrollback(&self) -> Result<()> {
        self.cmd_tx.send(TerminalCommand::ClearScrollback)?;
        Ok(())
    }

    /// Set scroll offset
    pub async fn set_scroll_offset(&self, offset: isize) -> Result<()> {
        self.cmd_tx.send(TerminalCommand::SetScrollOffset(offset))?;
        Ok(())
    }

    /// Get terminal content
    pub async fn content(&self) -> Vec<Vec<crate::Cell>> {
        let buffer_guard = self.buffer.read().await;
        buffer_guard.content().await
    }

    /// Get visible content
    pub async fn visible_content(&self) -> Vec<Vec<crate::Cell>> {
        let buffer_guard = self.buffer.read().await;
        buffer_guard.visible_content()
    }

    /// Get cursor position
    pub async fn cursor_position(&self) -> (u16, u16) {
        let buffer_guard = self.buffer.read().await;
        buffer_guard.cursor()
    }

    /// Get terminal size
    pub async fn size(&self) -> (u16, u16) {
        let buffer_guard = self.buffer.read().await;
        buffer_guard.size()
    }

    /// Get terminal title
    pub async fn title(&self) -> String {
        let buffer_guard = self.buffer.read().await;
        buffer_guard.title().to_string()
    }

    /// Check if terminal is alive
    pub fn is_alive(&self) -> bool {
        self.pty.blocking_read().is_some()
    }

    /// Subscribe to terminal events
    pub fn subscribe(&self) -> broadcast::Receiver<TerminalEvent> {
        self.event_tx.subscribe()
    }

    /// Get terminal stats
    pub async fn stats(&self) -> TerminalStats {
        let bw = self.bytes_written.read().await;
        let br = self.bytes_read.read().await;
        let buffer_guard = self.buffer.read().await;
        TerminalStats {
            bytes_written: *bw,
            bytes_read: *br,
            lines_scrolled: buffer_guard.lines_scrolled() as u64,
            bell_count: buffer_guard.bell_count() as u64,
            peak_memory_kb: 0, // Would need memory tracking
            current_memory_kb: 0,
            uptime_secs: self.created_at.elapsed().as_secs(),
            resize_count: self.buffer.read().await.resize_count(),
        }
    }

    /// Close session
    pub async fn close(&self) -> Result<()> {
        self.cmd_tx.send(TerminalCommand::Close)?;
        Ok(())
    }
}

impl Clone for TerminalSession {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            buffer: self.buffer.clone(),
            pty: self.pty.clone(),
            renderer: self.renderer.clone(),
            event_tx: self.event_tx.clone(),
            event_rx: self.event_tx.subscribe(),
            cmd_tx: self.cmd_tx.clone(),
            cmd_rx: Arc::new(RwLock::new(None)),
            config: self.config.clone(),
            created_at: self.created_at,
            last_activity: self.last_activity.clone(),
            bytes_written: self.bytes_written.clone(),
            bytes_read: self.bytes_read.clone(),
            split_manager: self.split_manager.clone(),
            parent_id: self.parent_id.clone(),
        }
    }
}

/// Session event
#[derive(Debug, Clone)]
pub enum SessionEvent {
    Created(SessionId),
    Closed(SessionId),
    Resized(SessionId, u16, u16),
    Data(SessionId, Vec<u8>),
    TitleChanged(SessionId, String),
    Activity(SessionId),
}

/// Terminal multiplexer
pub struct Multiplexer {
    /// Active sessions
    sessions: HashMap<SessionId, TerminalSession>,
    /// Session handles
    handles: HashMap<SessionId, TerminalHandle>,
    /// Event sender
    event_tx: broadcast::Sender<SessionEvent>,
    /// Event receiver
    event_rx: broadcast::Receiver<SessionEvent>,
    /// Default configuration
    default_config: TerminalConfig,
    /// Theme manager
    theme_manager: Arc<ThemeManager>,
}

impl Multiplexer {
    /// Create a new multiplexer
    pub fn new(default_config: TerminalConfig) -> Self {
        let (event_tx, event_rx) = broadcast::channel(100);
        let theme_manager = Arc::new(ThemeManager::new());

        Self {
            sessions: HashMap::new(),
            handles: HashMap::new(),
            event_tx,
            event_rx,
            default_config,
            theme_manager,
        }
    }

    /// Create a new terminal session
    pub async fn create_session(
        &mut self,
        name: Option<String>,
        config: Option<TerminalConfig>,
    ) -> Result<SessionId> {
        let id = Uuid::new_v4().to_string();
        let name = name.unwrap_or_else(|| format!("Terminal {}", self.sessions.len() + 1));
        let config = config.unwrap_or_else(|| self.default_config.clone());

        let session = TerminalSession::new(id.clone(), name.clone(), config).await?;
        
        self.sessions.insert(id.clone(), session);
        
        let handle = TerminalHandle::new(id.clone(), Arc::new(RwLock::new(self.clone())));
        self.handles.insert(id.clone(), handle);

        let _ = self.event_tx.send(SessionEvent::Created(id.clone()));

        info!("Created terminal session: {} ({})", name, id);
        Ok(id)
    }

    /// Get a terminal session
    pub fn get_session(&self, id: &str) -> Option<&TerminalSession> {
        self.sessions.get(id)
    }

    /// Get a terminal handle
    pub fn get_handle(&self, id: &str) -> Option<TerminalHandle> {
        self.handles.get(id).cloned()
    }

    /// List all session IDs
    pub fn list_sessions(&self) -> Vec<SessionId> {
        self.sessions.keys().cloned().collect()
    }

    /// List all session info
    pub fn list_session_info(&self) -> Vec<SessionInfo> {
        self.sessions
            .iter()
            .map(|(id, session)| SessionInfo {
                id: id.clone(),
                name: session.name().to_string(),
                rows: 0, // Size info requires async call, use default
                cols: 0,
                is_alive: session.is_alive(),
                created_at: session.created_at,
                last_activity: std::time::Instant::now(),
            })
            .collect()
    }

    /// Close a session
    pub async fn close_session(&mut self, id: &str) -> Result<()> {
        if let Some(session) = self.sessions.remove(id) {
            session.close().await?;
            self.handles.remove(id);
            let _ = self.event_tx.send(SessionEvent::Closed(id.to_string()));
            info!("Closed terminal session: {}", id);
        }
        Ok(())
    }

    /// Close all sessions
    pub async fn close_all(&mut self) {
        for id in self.sessions.keys().cloned().collect::<Vec<_>>() {
            let _ = self.close_session(&id).await;
        }
    }

    /// Write to all sessions
    pub async fn broadcast(&self, data: &[u8]) -> usize {
        let mut count = 0;
        for session in self.sessions.values() {
            if session.write(data).await.is_ok() {
                count += 1;
            }
        }
        count
    }

    /// Subscribe to session events
    pub fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.event_tx.subscribe()
    }

    /// Get theme manager
    pub fn theme_manager(&self) -> &ThemeManager {
        &self.theme_manager
    }

    /// Apply theme to session
    pub async fn apply_theme(&self, session_id: &str, theme_name: &str) -> Result<()> {
        if let Some(_session) = self.sessions.get(session_id) {
            if let Some(_theme) = self.theme_manager.get_theme(theme_name) {
                // Would need to apply theme to renderer
                Ok(())
            } else {
                Err(TerminalError::ThemeError(format!("Theme not found: {}", theme_name)))
            }
        } else {
            Err(TerminalError::SessionNotFound(session_id.to_string()))
        }
    }

    /// Split a session
    pub async fn split_session(
        &mut self,
        session_id: &str,
        _direction: crate::Direction,
        _size: Option<u16>,
    ) -> Result<SessionId> {
        let (parent_config, parent_name) = {
            let parent = self.sessions.get(session_id)
                .ok_or_else(|| TerminalError::SessionNotFound(session_id.to_string()))?;
            (parent.config.clone(), parent.name().to_string())
        };

        // Create new session for split
        let child_id = self.create_session(
            Some(format!("{}-split", parent_name)),
            Some(parent_config),
        ).await?;

        // Initialize split manager if needed
        {
            if let Some(parent_ref) = self.sessions.get(session_id) {
                let mut split_manager: tokio::sync::RwLockWriteGuard<Option<SplitManager>> =
                    parent_ref.split_manager.write().await;
                if split_manager.is_none() {
                    *split_manager = Some(SplitManager::new());
                }
                // TODO: Add pane IDs when slotmap integration is finalized
                // if let Some(manager) = split_manager.as_mut() {
                //     manager.add_pane(parent_ref.id(), None)?;
                //     manager.add_pane(child_id, Some(direction))?;
                // }
            }
        }

        // Mark child as split
        if let Some(child) = self.sessions.get_mut(&child_id) {
            child.parent_id = Some(session_id.to_string());
        }

        Ok(child_id)
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Check if session exists
    pub fn has_session(&self, id: &str) -> bool {
        self.sessions.contains_key(id)
    }
}

impl Clone for Multiplexer {
    fn clone(&self) -> Self {
        Self {
            sessions: self.sessions.clone(),
            handles: self.handles.clone(),
            event_tx: self.event_tx.clone(),
            event_rx: self.event_tx.subscribe(),
            default_config: self.default_config.clone(),
            theme_manager: self.theme_manager.clone(),
        }
    }
}

/// Session information
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub rows: u16,
    pub cols: u16,
    pub is_alive: bool,
    pub created_at: std::time::Instant,
    pub last_activity: std::time::Instant,
}