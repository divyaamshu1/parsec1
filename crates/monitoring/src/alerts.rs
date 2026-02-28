#![allow(dead_code, unused_imports, unused_variables)]

//! Monitoring alerts stub

use serde::{Serialize, Deserialize};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub severity: AlertSeverity,
    pub message: String,
}

pub struct AlertManager;

impl AlertManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn trigger_alert(&self, _alert: Alert) -> Result<()> {
        Ok(())
    }
}