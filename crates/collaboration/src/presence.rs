//! Presence awareness

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{RwLock, broadcast};
use tokio::time;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::{UserId, DocumentId, Result};

/// User status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Online,
    Away,
    Busy,
    Offline,
    Invisible,
}

/// Activity type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Activity {
    Editing { document: DocumentId, position: Option<(u32, u32)> },
    Debugging,
    Running,
    Idle,
    InMeeting,
    Presenting,
    Custom(String),
}

/// User presence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresence {
    pub user_id: UserId,
    pub status: Status,
    pub activity: Option<Activity>,
    pub last_seen: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub device: DeviceInfo,
    pub current_document: Option<DocumentId>,
    pub session_id: Option<String>,
}

/// Device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub platform: String,
    pub version: String,
    pub capabilities: Vec<String>,
}

/// Presence manager
pub struct PresenceManager {
    presences: Arc<RwLock<HashMap<UserId, UserPresence>>>,
    current_user: UserId,
    event_tx: broadcast::Sender<PresenceEvent>,
    event_rx: broadcast::Receiver<PresenceEvent>,
}

/// Presence event
#[derive(Debug, Clone)]
pub enum PresenceEvent {
    StatusChanged(UserId, Status),
    ActivityChanged(UserId, Option<Activity>),
    UserJoined(UserId),
    UserLeft(UserId),
    UserUpdated(UserId, UserPresence),
}

impl PresenceManager {
    /// Create new presence manager
    pub fn new(current_user: UserId) -> Self {
        let (event_tx, event_rx) = broadcast::channel(100);
        
        let manager = Self {
            presences: Arc::new(RwLock::new(HashMap::new())),
            current_user,
            event_tx,
            event_rx,
        };

        // Start heartbeat
        let manager_clone = manager.clone();
        tokio::spawn(async move {
            manager_clone.heartbeat_loop().await;
        });

        manager
    }

    /// Update current user presence
    pub async fn update_presence(&self, status: Option<Status>, activity: Option<Activity>) -> Result<()> {
        let mut presences = self.presences.write().await;
        
        let presence = presences.entry(self.current_user.clone()).or_insert_with(|| {
            UserPresence {
                user_id: self.current_user.clone(),
                status: Status::Online,
                activity: None,
                last_seen: Utc::now(),
                last_activity: Utc::now(),
                device: DeviceInfo {
                    name: "Unknown".to_string(),
                    platform: std::env::consts::OS.to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    capabilities: vec!["text".to_string()],
                },
                current_document: None,
                session_id: None,
            }
        });

        if let Some(s) = status {
            if presence.status != s {
                let _ = self.event_tx.send(PresenceEvent::StatusChanged(
                    self.current_user.clone(),
                    s.clone(),
                ));
            }
            presence.status = s; 
        }

        if let Some(a) = activity {
            let _ = self.event_tx.send(PresenceEvent::ActivityChanged(
                self.current_user.clone(),
                Some(a.clone())
            ));
            presence.activity = Some(a);
            presence.last_activity = Utc::now();
        }

        presence.last_seen = Utc::now();

        Ok(())
    }

    /// Update user presence (for other users)
    pub async fn update_remote_presence(&self, presence: UserPresence) {
        let user_id = presence.user_id.clone();
        let event = if self.presences.read().await.contains_key(&user_id) {
            PresenceEvent::UserUpdated(user_id.clone(), presence.clone())
        } else {
            PresenceEvent::UserJoined(user_id.clone())
        };
        
        self.presences.write().await.insert(user_id.clone(), presence);
        let _ = self.event_tx.send(event);
    }

    /// Remove user presence
    pub async fn remove_user(&self, user_id: &UserId) {
        self.presences.write().await.remove(user_id);
        let _ = self.event_tx.send(PresenceEvent::UserLeft(user_id.clone()));
    }

    /// Get presence for user
    pub async fn get_presence(&self, user_id: &UserId) -> Option<UserPresence> {
        self.presences.read().await.get(user_id).cloned()
    }

    /// Get all presences
    pub async fn get_all_presences(&self) -> Vec<UserPresence> {
        self.presences.read().await.values().cloned().collect()
    }

    /// Get online users
    pub async fn get_online_users(&self) -> Vec<UserId> {
        self.presences.read().await
            .values()
            .filter(|p| p.status != Status::Offline)
            .map(|p| p.user_id.clone())
            .collect()
    }

    /// Check if user is online
    pub async fn is_online(&self, user_id: &UserId) -> bool {
        self.presences.read().await
            .get(user_id)
            .map(|p| p.status != Status::Offline)
            .unwrap_or(false)
    }

    /// Heartbeat loop
    async fn heartbeat_loop(&self) {
        let mut interval = time::interval(Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            // Update current user presence
            let mut presences = self.presences.write().await;
            if let Some(presence) = presences.get_mut(&self.current_user) {
                presence.last_seen = Utc::now();
            }
            
            // Check for stale presences
            let now = Utc::now();
            let stale_users: Vec<UserId> = presences
                .iter()
                .filter(|(_, p)| {
                    p.user_id != self.current_user && 
                    (now - p.last_seen).num_seconds() > 120
                })
                .map(|(id, _)| id.clone())
                .collect();

            for user_id in stale_users {
                if let Some(presence) = presences.get_mut(&user_id) {
                    presence.status = Status::Offline;
                    let _ = self.event_tx.send(PresenceEvent::StatusChanged(
                        user_id.clone(),
                        Status::Offline
                    ));
                }
            }
        }
    }

    /// Subscribe to presence events
    pub fn subscribe(&self) -> broadcast::Receiver<PresenceEvent> {
        self.event_tx.subscribe()
    }
}

impl Clone for PresenceManager {
    fn clone(&self) -> Self {
        Self {
            presences: self.presences.clone(),
            current_user: self.current_user.clone(),
            event_tx: self.event_tx.clone(),
            event_rx: self.event_tx.subscribe(),
        }
    }
}