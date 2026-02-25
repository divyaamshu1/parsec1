//! Live sharing for real-time collaboration

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use tokio::sync::{RwLock, mpsc};
use tracing::{info, warn, debug};
use serde::{Serialize, Deserialize};

#[cfg(feature = "live-share")]
use webrtc::{
    api::{APIBuilder, API},
    peer_connection::*,
    data_channel::*,
    ice_transport::*,
};

use crate::CollaborationConfig;

/// Live share manager
pub struct LiveShareManager {
    config: CollaborationConfig,
    sessions: Arc<RwLock<HashMap<String, LiveShareSession>>>,
    peers: Arc<RwLock<HashMap<String, PeerConnection>>>,
}

/// Live share session
#[derive(Debug, Clone)]
pub struct LiveShareSession {
    pub id: String,
    pub host_id: String,
    pub participants: Vec<String>,
    pub shared_files: Vec<SharedFile>,
    pub shared_terminals: Vec<SharedTerminal>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Shared file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedFile {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub content: Option<String>,
    pub owner_id: String,
    pub read_only: bool,
}

/// Shared terminal
#[derive(Debug, Clone)]
pub struct SharedTerminal {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub input: mpsc::Sender<String>,
    pub output: mpsc::Receiver<String>,
}

/// Peer connection
#[derive(Debug)]
pub struct PeerConnection {
    pub peer_id: String,
    pub session_id: String,
    #[cfg(feature =