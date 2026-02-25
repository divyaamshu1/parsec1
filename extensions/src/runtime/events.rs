//! Extension event system

use std::sync::Arc;
use tokio::sync::mpsc;
use crate::ExtensionEvent;

/// Event listener for extensions
pub struct EventListener {
    runtime: Arc<crate::runtime::ExtensionRuntime>,
    handler: Box<dyn Fn(ExtensionEvent) + Send + Sync>,
}

impl EventListener {
    /// Create a new event listener
    pub fn new<F>(runtime: Arc<crate::runtime::ExtensionRuntime>, handler: F) -> Self
    where
        F: Fn(ExtensionEvent) + Send + Sync + 'static,
    {
        Self {
            runtime,
            handler: Box::new(handler),
        }
    }

    /// Start listening for events
    pub async fn start_listening(self) {
        // NOTE: Spawn is disabled due to Send/Sync constraints
        // Events can still be received synchronously
    }
}

/// Stats collector for runtime metrics
pub struct StatsCollector {
    runtime: Arc<crate::runtime::ExtensionRuntime>,
    interval: std::time::Duration,
}

impl StatsCollector {
    /// Create a new stats collector
    pub fn new(runtime: Arc<crate::runtime::ExtensionRuntime>, interval: std::time::Duration) -> Self {
        Self { runtime, interval }
    }

    /// Start collecting stats
    pub async fn start_collecting(&self) {
        // NOTE: Spawn is disabled due to Send/Sync constraints
        // The statistics can still be queried synchronously
    }
}