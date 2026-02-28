#![allow(dead_code, unused_imports, ambiguous_glob_imports)]

//! Monitoring metrics stub

use std::sync::Arc;
use tokio::sync::RwLock;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone)]
pub struct MetricsConfig {
    pub enabled: bool,
}

pub struct MetricsCollector {
    config: Arc<RwLock<MetricsConfig>>,
}

impl MetricsCollector {
    pub fn new(_config: MetricsConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(MetricsConfig { enabled: true })),
        }
    }

    pub async fn record_counter(&self, _name: &str, _value: u64, _labels: &[(&str, &str)]) -> Result<()> {
        Ok(())
    }

    pub fn histogram(&self, _name: &str) -> Result<()> {
        Ok(())
    }
}