//! Parsec Collaboration Engine
//!
//! Real-time collaboration features including:
//! - Live sharing with WebRTC P2P
//! - Comments and discussions
//! - Code review workflows
//! - Presence awareness
//! - Encrypted communication
//! - CRDT-based conflict-free editing

#![allow(dead_code, unused_imports)]

pub mod live_share;
pub mod comments;
pub mod review;
pub mod presence;
pub mod crdt;
pub mod signaling;
pub mod encryption;
pub mod metrics;

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, broadcast, mpsc};
use chrono::{DateTime, Utc};
use uuid::Uuid;

// Re-exports
pub use live_share::{LiveShare, LiveShareSession, SessionRole, Participant, SessionState};
pub use comments::{Comment, CommentThread, CommentManager, CommentReply};
pub use review::{Review, ReviewManager, ReviewStatus, ChangeRequest, Feedback};
pub use presence::{PresenceManager, UserPresence, Status, Activity};
pub use crdt::{Document, CrdtManager, Edit, Operation, SyncState};
pub use signaling::{SignalingServer, SignalingClient, SignalMessage};
pub use encryption::{EncryptionManager, KeyPair, Cipher, MessageEnvelope};
pub use metrics::CollaborationMetrics;

/// Result type for collaboration operations
pub type Result<T> = std::result::Result<T, CollaborationError>;

/// Collaboration error
#[derive(Debug, thiserror::Error)]
pub enum CollaborationError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Participant not found: {0}")]
    ParticipantNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Signaling failed: {0}")]
    SignalingFailed(String),

    #[error("WebRTC error: {0}")]
    WebRtcError(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("CRDT error: {0}")]
    CrdtError(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Channel error")]
    ChannelError,

    #[error("Timeout")]
    Timeout,
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for CollaborationError {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        CollaborationError::ChannelError
    }
}

impl From<tokio::sync::broadcast::error::SendError<Vec<u8>>> for CollaborationError {
    fn from(_: tokio::sync::broadcast::error::SendError<Vec<u8>>) -> Self {
        CollaborationError::ChannelError
    }
}

impl From<futures::channel::mpsc::SendError> for CollaborationError {
    fn from(_: futures::channel::mpsc::SendError) -> Self {
        CollaborationError::ChannelError
    }
}

/// User identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct UserId(pub String);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Session identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Document identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct DocumentId(pub String);

impl DocumentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Collaboration event
#[derive(Debug, Clone)]
pub enum CollaborationEvent {
    /// Session events
    SessionCreated(SessionId, UserId),
    SessionJoined(SessionId, UserId),
    SessionLeft(SessionId, UserId),
    SessionClosed(SessionId),

    /// Participant events
    ParticipantJoined(SessionId, Participant),
    ParticipantLeft(SessionId, UserId),
    ParticipantUpdated(SessionId, Participant),

    /// Document events
    DocumentOpened(DocumentId, UserId),
    DocumentClosed(DocumentId, UserId),
    DocumentChanged(DocumentId, Edit),

    /// Comment events
    CommentAdded(CommentId),
    CommentUpdated(CommentId),
    CommentResolved(CommentId),
    CommentReplied(CommentId, CommentId),

    /// Review events
    ReviewCreated(ReviewId),
    ReviewUpdated(ReviewId),
    ReviewApproved(ReviewId),
    ReviewChangesRequested(ReviewId),

    /// Presence events
    PresenceChanged(UserId, UserPresence),
    ActivityStarted(UserId, Activity),
    ActivityEnded(UserId, Activity),

    /// Error events
    Error(String),
}

/// Comment identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CommentId(pub String);

impl CommentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for CommentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Review identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReviewId(pub String);

impl ReviewId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for ReviewId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Collaboration configuration
#[derive(Debug, Clone)]
pub struct CollaborationConfig {
    /// Signaling server URL
    pub signaling_server: String,
    /// TURN server URL (optional)
    pub turn_server: Option<String>,
    /// TURN credentials
    pub turn_credentials: Option<(String, String)>,
    /// STUN server URLs
    pub stun_servers: Vec<String>,
    /// Enable encryption
    pub enable_encryption: bool,
    /// Enable metrics
    pub enable_metrics: bool,
    /// Heartbeat interval (seconds)
    pub heartbeat_interval: u64,
    /// Reconnect timeout (seconds)
    pub reconnect_timeout: u64,
    /// Max participants per session
    pub max_participants: usize,
    /// Compression enabled
    pub compression: bool,
}

impl Default for CollaborationConfig {
    fn default() -> Self {
        Self {
            signaling_server: "wss://signaling.parsec.dev".to_string(),
            turn_server: None,
            turn_credentials: None,
            stun_servers: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ],
            enable_encryption: true,
            enable_metrics: true,
            heartbeat_interval: 30,
            reconnect_timeout: 10,
            max_participants: 50,
            compression: true,
        }
    }
}

/// Main collaboration engine
pub struct CollaborationEngine {
    /// Configuration
    config: CollaborationConfig,
    /// Active sessions
    sessions: Arc<RwLock<HashMap<SessionId, LiveShareSession>>>,
    /// Presence manager
    presence: Arc<PresenceManager>,
    /// Comment manager
    comments: Arc<CommentManager>,
    /// Review manager
    reviews: Arc<ReviewManager>,
    /// CRDT manager
    crdt: Arc<CrdtManager>,
    /// Encryption manager
    encryption: Arc<EncryptionManager>,
    /// Metrics
    metrics: Arc<CollaborationMetrics>,
    /// Event broadcaster
    event_tx: broadcast::Sender<CollaborationEvent>,
    /// Event receiver
    event_rx: broadcast::Receiver<CollaborationEvent>,
    /// Current user
    current_user: UserId,
}

impl CollaborationEngine {
    /// Create new collaboration engine
    pub fn new(config: CollaborationConfig, current_user: UserId) -> Self {
        let (event_tx, event_rx) = broadcast::channel(1000);

        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            presence: Arc::new(PresenceManager::new(current_user.clone())),
            comments: Arc::new(CommentManager::new()),
            reviews: Arc::new(ReviewManager::new()),
            crdt: Arc::new(CrdtManager::new()),
            encryption: Arc::new(EncryptionManager::new()),
            metrics: Arc::new(CollaborationMetrics::new()),
            event_tx,
            event_rx,
            current_user,
        }
    }

    /// Create a new live share session
    pub async fn create_session(&self, name: String, password: Option<String>) -> Result<LiveShareSession> {
        let session = LiveShareSession::new(
            name,
            self.current_user.clone(),
            password,
            self.config.clone(),
        ).await?;

        self.sessions.write().await.insert(session.id.clone(), session.clone());
        
        let _ = self.event_tx.send(CollaborationEvent::SessionCreated(
            session.id.clone(),
            self.current_user.clone()
        ));

        Ok(session)
    }

    /// Join a live share session
    pub async fn join_session(
        &self,
        session_id: SessionId,
        password: Option<String>,
    ) -> Result<LiveShareSession> {
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(&session_id)
            .ok_or_else(|| CollaborationError::SessionNotFound(session_id.to_string()))?;

        session.add_participant(self.current_user.clone(), password).await?;

        let _ = self.event_tx.send(CollaborationEvent::SessionJoined(
            session_id.clone(),
            self.current_user.clone()
        ));

        Ok(session.clone())
    }

    /// Leave a session
    pub async fn leave_session(&self, session_id: SessionId) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.remove_participant(&self.current_user).await?;

            let _ = self.event_tx.send(CollaborationEvent::SessionLeft(
                session_id.clone(),
                self.current_user.clone()
            ));

            // check participants count directly
            if session.participants.read().await.is_empty() {
                sessions.remove(&session_id);
                let _ = self.event_tx.send(CollaborationEvent::SessionClosed(session_id));
            }
        }
        Ok(())
    }

    /// Get session
    pub async fn get_session(&self, session_id: &SessionId) -> Option<LiveShareSession> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// List active sessions
    pub async fn list_sessions(&self) -> Vec<LiveShareSession> {
        self.sessions.read().await.values().cloned().collect()
    }

    /// Get presence manager
    pub fn presence(&self) -> Arc<PresenceManager> {
        self.presence.clone()
    }

    /// Get comment manager
    pub fn comments(&self) -> Arc<CommentManager> {
        self.comments.clone()
    }

    /// Get review manager
    pub fn reviews(&self) -> Arc<ReviewManager> {
        self.reviews.clone()
    }

    /// Get CRDT manager
    pub fn crdt(&self) -> Arc<CrdtManager> {
        self.crdt.clone()
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<CollaborationEvent> {
        self.event_tx.subscribe()
    }

    /// Get metrics
    pub fn metrics(&self) -> Arc<CollaborationMetrics> {
        self.metrics.clone()
    }
}

/// Position in document
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// Range in document
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}