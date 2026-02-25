//! Callstack management

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};

/// Stack frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub id: usize,
    pub name: String,
    pub file: Option<String>,
    pub line: usize,
    pub column: usize,
    pub end_line: Option<usize>,
}

/// Callstack manager
pub struct CallstackManager {
    frames: Arc<tokio::sync::RwLock<HashMap<usize, Vec<StackFrame>>>>,
}

impl CallstackManager {
    /// Create new callstack manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            frames: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        })
    }

    /// Set stack frames for thread
    pub async fn set_frames(&self, thread_id: usize, frames: Vec<StackFrame>) {
        self.frames.write().await.insert(thread_id, frames);
    }

    /// Get stack frames for thread
    pub async fn get_frames(&self, thread_id: usize) -> Option<Vec<StackFrame>> {
        self.frames.read().await.get(&thread_id).cloned()
    }

    /// Get frame by ID
    pub async fn get_frame(&self, frame_id: usize) -> Option<StackFrame> {
        for frames in self.frames.read().await.values() {
            for frame in frames {
                if frame.id == frame_id {
                    return Some(frame.clone());
                }
            }
        }
        None
    }

    /// Clear frames for thread
    pub async fn clear(&self, thread_id: usize) {
        self.frames.write().await.remove(&thread_id);
    }

    /// Clear all frames
    pub async fn clear_all(&self) {
        self.frames.write().await.clear();
    }
}