//! Runtime statistics and metrics

use std::time::{Duration, Instant};

/// Runtime statistics (internal)
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    pub total_extensions_loaded: usize,
    pub total_extensions_activated: usize,
    pub total_commands_executed: usize,
    pub total_errors: usize,
    pub peak_active_extensions: usize,
    pub start_time: Instant,
}

impl Default for RuntimeStats {
    fn default() -> Self {
        Self {
            total_extensions_loaded: 0,
            total_extensions_activated: 0,
            total_commands_executed: 0,
            total_errors: 0,
            peak_active_extensions: 0,
            start_time: Instant::now(),
        }
    }
}

impl RuntimeStats {
    /// Create new runtime stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Record extension loaded
    pub fn record_loaded(&mut self) {
        self.total_extensions_loaded += 1;
    }

    /// Record extension activated
    pub fn record_activated(&mut self) {
        self.total_extensions_activated += 1;
    }

    /// Record command executed
    pub fn record_command(&mut self) {
        self.total_commands_executed += 1;
    }

    /// Record error
    pub fn record_error(&mut self) {
        self.total_errors += 1;
    }

    /// Update peak active
    pub fn update_peak(&mut self, current: usize) {
        self.peak_active_extensions = self.peak_active_extensions.max(current);
    }

    /// Get average commands per second
    pub fn commands_per_second(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.total_commands_executed as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Get error rate
    pub fn error_rate(&self) -> f64 {
        if self.total_extensions_loaded > 0 {
            self.total_errors as f64 / self.total_extensions_loaded as f64
        } else {
            0.0
        }
    }
}

/// Runtime statistics (public)
#[derive(Debug, Clone)]
pub struct RuntimeStatistics {
    pub total_extensions: usize,
    pub active_extensions: usize,
    pub total_extensions_loaded: usize,
    pub total_extensions_activated: usize,
    pub total_commands_executed: usize,
    pub total_errors: usize,
    pub peak_active_extensions: usize,
    pub total_memory_used: usize,
    pub peak_memory_used: usize,
    pub uptime: Duration,
}

/// Runtime metrics
#[derive(Debug, Clone)]
pub struct RuntimeMetrics {
    pub active_extensions: usize,
    pub total_loaded: usize,
    pub total_activated: usize,
    pub total_commands: usize,
    pub total_errors: usize,
    pub peak_active: usize,
    pub memory_used: usize,
    pub peak_memory: usize,
    pub uptime_secs: f64,
    pub commands_per_sec: f64,
    pub error_rate: f64,
}