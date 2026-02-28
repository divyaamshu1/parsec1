#![allow(dead_code, unused_imports)]

//! System resource monitoring stub

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub cores: usize,
    pub total_usage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total: u64,
    pub used: u64,
    pub free: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub total: u64,
    pub used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>,
    pub timestamp: DateTime<Utc>,
}

pub struct SystemMonitor;

impl SystemMonitor {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_metrics(&self) -> Result<SystemMetrics> {
        Ok(SystemMetrics {
            cpu: CpuInfo { cores: 0, total_usage: 0.0 },
            memory: MemoryInfo { total: 0, used: 0, free: 0 },
            disks: Vec::new(),
            timestamp: Utc::now(),
        })
    }
}