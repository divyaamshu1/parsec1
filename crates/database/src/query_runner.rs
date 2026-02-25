//! Query runner with history and auto-completion

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use sqlparser::{parser::Parser, dialect::GenericDialect};
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

/// Query runner
pub struct QueryRunner {
    history: Arc<RwLock<HashMap<String, VecDeque<QueryHistoryEntry>>>>,
    max_history: usize,
}

/// Query history entry
#[derive(Debug, Clone)]
pub struct QueryHistoryEntry {
    pub query: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub execution_time: Duration,
    pub success: bool,
    pub rows_affected: u64,
}

impl QueryRunner {
    /// Create a new query runner
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Arc::new(RwLock::new(HashMap::new())),
            max_history,
        }
    }

    /// Record a query execution
    pub async fn record_query(&self, connection: &str, query: &str, execution_time: Duration) {
        let mut history = self.history.write().await;
        let conn_history = history.entry(connection.to_string())
            .or_insert_with(|| VecDeque::with_capacity(self.max_history));

        let entry = QueryHistoryEntry {
            query: query.to_string(),
            timestamp: chrono::Utc::now(),
            execution_time,
            success: true,
            rows_affected: 0,
        };

        conn_history.push_front(entry);
        if conn_history.len() > self.max_history {
            conn_history.pop_back();
        }
    }

    /// Get query history for a connection
    pub async fn get_history(&self, connection: &str) -> Vec<QueryHistoryEntry> {
        let history = self.history.read().await;
        history.get(connection)
            .map(|h| h.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Clear query history
    pub async fn clear(&self) {
        self.history.write().await.clear();
    }

    /// Format query for display
    pub fn format_query(&self, query: &str, max_width: usize) -> String {
        if query.len() <= max_width {
            query.to_string()
        } else {
            format!("{}...", &query[..max_width])
        }
    }

    /// Get query suggestions based on history
    pub async fn get_suggestions(&self, connection: &str, partial: &str) -> Vec<String> {
        let history = self.history.read().await;
        let mut suggestions = Vec::new();

        if let Some(conn_history) = history.get(connection) {
            for entry in conn_history {
                if entry.query.contains(partial) && !suggestions.contains(&entry.query) {
                    suggestions.push(entry.query.clone());
                }
            }
        }

        suggestions
    }

    /// Parse SQL query
    pub fn parse_sql(&self, query: &str) -> Result<Vec<String>> {
        let dialect = GenericDialect {};
        match Parser::parse_sql(&dialect, query) {
            Ok(statements) => {
                let stmt_strings = statements.iter()
                    .map(|s| format!("{}", s))
                    .collect();
                Ok(stmt_strings)
            }
            Err(e) => Err(anyhow!("SQL parsing error: {}", e)),
        }
    }

    /// Check if query is read-only (SELECT)
    pub fn is_read_only(&self, query: &str) -> bool {
        let trimmed = query.trim().to_uppercase();
        trimmed.starts_with("SELECT") || trimmed.starts_with("SHOW") || trimmed.starts_with("DESCRIBE")
    }

    /// Check if query is a transaction control
    pub fn is_transaction_control(&self, query: &str) -> bool {
        let trimmed = query.trim().to_uppercase();
        trimmed.starts_with("BEGIN") || trimmed.starts_with("COMMIT") || trimmed.starts_with("ROLLBACK")
    }
}