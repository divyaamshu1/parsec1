#![allow(dead_code, unused_imports)]

//! Monitoring dashboard

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::{Result, MonitoringConfig};

#[derive(Debug, Clone)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeRange {
    pub fn last_minutes(minutes: i64) -> Self {
        let end = Utc::now();
        let start = end - chrono::Duration::minutes(minutes);
        Self { start, end }
    }

    pub fn last_hours(hours: i64) -> Self {
        let end = Utc::now();
        let start = end - chrono::Duration::hours(hours);
        Self { start, end }
    }

    pub fn last_days(days: i64) -> Self {
        let end = Utc::now();
        let start = end - chrono::Duration::days(days);
        Self { start, end }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Aggregation {
    Avg, Sum, Min, Max, Count, Rate, P50, P90, P95, P99,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WidgetType {
    LineChart, BarChart, Gauge, Table, Heatmap, MapChart,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub refresh_interval: u64,
    pub layout: Vec<Vec<WidgetType>>,
}

pub struct Dashboard {
    config: Arc<RwLock<DashboardConfig>>,
}

impl Dashboard {
    pub async fn new(_config: MonitoringConfig) -> Result<Self> {
        Ok(Self {
            config: Arc::new(RwLock::new(DashboardConfig {
                refresh_interval: 30,
                layout: vec![],
            })),
        })
    }

    pub async fn render(&self, _format: &str) -> Result<String> {
        Ok(String::new())
    }
}