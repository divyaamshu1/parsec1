//! SQLite database driver

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use rusqlite::{Connection, OpenFlags, types::Value as SqliteValue};
use tokio::task;
use tracing::{info, warn, debug};

use crate::{DatabaseConnection, DatabaseType, QueryResult, Value, TableInfo, TableSchema, ColumnInfo, DatabaseStats};

/// SQLite connection
pub struct SQLiteConnection {
    name: String,
    path: PathBuf,
    conn: Option<Arc<Mutex<Connection>>>,
    connected: bool,
}

impl SQLiteConnection {
    /// Create a new SQLite connection
    pub fn new(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            conn: None,
            connected: false,
        }
    }

    /// Create an in-memory database
    pub fn in_memory(name: String) -> Self {
        Self {
            name,
            path: PathBuf::from(":memory:"),
            conn: None,
            connected: false,
        }
    }

    /// Convert SQLite value to our Value enum
    fn sqlite_to_value(&self, value: SqliteValue) -> Value {
        match value {
            SqliteValue::Null => Value::Null,
            SqliteValue::Integer(i) => Value::Integer(i),
            SqliteValue::Real(f) => Value::Float(f),
            SqliteValue::Text(s) => Value::String(s),
            SqliteValue::Blob(b) => Value::Binary(b),
        }
    }
}

#[async_trait]
impl DatabaseConnection for SQLiteConnection {
    fn db_type(&self) -> DatabaseType {
        DatabaseType::SQLite
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn connection_string(&self) -> String {
        format!("sqlite:{}", self.path.display())
    }

    async fn connect(&mut self) -> Result<()> {
        let path = self.path.clone();
        
        // Run blocking SQLite operations in a separate thread
        let conn = task::spawn_blocking(move || {
            if path.to_string_lossy() == ":memory:" {
                Connection::open_in_memory()
            } else {
                Connection::open_with_flags(
                    &path,
                    OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
                )
            }
        }).await??;

        self.conn = Some(Arc::new(Mutex::new(conn)));
        self.connected = true;

        info!("Connected to SQLite database: {}", self.name);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.conn = None;
        self.connected = false;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }

    async fn execute_query(&self, query: &str) -> Result<QueryResult> {
        let conn = self.conn.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?
            .clone();

        let query = query.to_string();
        let start = std::time::Instant::now();

        // Run blocking SQLite operations in a separate thread
        let (columns, rows, rows_affected) = task::spawn_blocking(move || {
            let conn_guard = conn.blocking_lock();
            let mut stmt = conn_guard.prepare(&query)?;
            let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
            
            let rows_iter = stmt.query_map([], |row: &rusqlite::Row| {
                let mut values = Vec::new();
                for i in 0..column_names.len() {
                    let value = row.get::<_, SqliteValue>(i)?;
                    values.push(value);
                }
                Ok(values)
            })?;

            let mut rows_data = Vec::new();
            for row_result in rows_iter {
                if let Ok(values) = row_result {
                    rows_data.push(values);
                }
            }

            let rows_affected = if query.to_uppercase().starts_with("INSERT") {
                conn_guard.last_insert_rowid() as u64
            } else if query.to_uppercase().starts_with("UPDATE") || query.to_uppercase().starts_with("DELETE") {
                conn_guard.changes() as u64
            } else {
                0
            };

            Ok::<_, anyhow::Error>((column_names, rows_data, rows_affected))
        }).await??;

        // Convert SQLite values to our Value enum
        let mut converted_rows = Vec::new();
        for row in rows {
            let mut converted_row = Vec::new();
            for value in row {
                converted_row.push(self.sqlite_to_value(value));
            }
            converted_rows.push(converted_row);
        }

        Ok(QueryResult {
            columns,
            rows: converted_rows,
            rows_affected,
            execution_time: start.elapsed(),
        })
    }

    async fn execute_update(&self, query: &str) -> Result<u64> {
        let conn = self.conn.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?
            .clone();

        let query = query.to_string();

        let changes = task::spawn_blocking(move || {
            let conn_guard = conn.blocking_lock();
            conn_guard.execute(&query, [])?;
            Ok::<_, anyhow::Error>(conn_guard.changes() as u64)
        }).await??;

        Ok(changes)
    }

    async fn list_tables(&self) -> Result<Vec<TableInfo>> {
        let conn = self.conn.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?
            .clone();

        let tables = task::spawn_blocking(move || {
            let conn_guard = conn.blocking_lock();
            let mut stmt = conn_guard.prepare(
                "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
            )?;

            let rows = stmt.query_map([], |row: &rusqlite::Row| row.get::<_, String>(0))?;

            let mut tables = Vec::new();
            for row in rows {
                tables.push(TableInfo {
                    name: row?,
                    schema: None,
                    row_count: None,
                    size_bytes: None,
                    created_at: None,
                });
            }

            Ok::<_, anyhow::Error>(tables)
        }).await??;

        Ok(tables)
    }

    async fn get_table_schema(&self, table: &str) -> Result<TableSchema> {
        let conn = self.conn.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?
            .clone();

        let table_name = table.to_string();
        let table_clone = table_name.clone();

        let columns = task::spawn_blocking(move || {
            let conn_guard = conn.blocking_lock();
            let mut stmt = conn_guard.prepare(&format!("PRAGMA table_info({})", table_clone))?;

            let rows = stmt.query_map([], |row: &rusqlite::Row| {
                Ok(ColumnInfo {
                    name: row.get(1)?,
                    data_type: row.get(2)?,
                    nullable: row.get::<_, i32>(3)? == 0,
                    default: row.get(4)?,
                    is_primary_key: row.get::<_, i32>(5)? == 1,
                    is_unique: false,
                })
            })?;

            let mut columns = Vec::new();
            for row in rows {
                columns.push(row?);
            }

            Ok::<_, anyhow::Error>(columns)
        }).await??;

        Ok(TableSchema {
            name: table_name,
            columns,
            primary_key: Vec::new(),
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
        })
    }

    async fn begin_transaction(&self) -> Result<Box<dyn crate::Transaction>> {
        self.execute_update("BEGIN TRANSACTION").await?;
        Ok(Box::new(SQLiteTransaction {
            name: self.name.clone(),
        }))
    }

    async fn get_stats(&self) -> Result<DatabaseStats> {
        Ok(DatabaseStats {
            connection_count: 1,
            active_queries: 0,
            total_queries: 0,
            average_query_time: Duration::from_secs(0),
            cache_hit_ratio: None,
            disk_usage: None,
            memory_usage: None,
        })
    }
}

/// SQLite transaction
pub struct SQLiteTransaction {
    name: String,
}

#[async_trait]
impl crate::Transaction for SQLiteTransaction {
    async fn execute(&self, query: &str) -> Result<QueryResult> {
        // Would need connection reference
        Err(anyhow!("Transaction execute not implemented"))
    }

    async fn commit(&self) -> Result<()> {
        // Would need to send COMMIT
        Ok(())
    }

    async fn rollback(&self) -> Result<()> {
        // Would need to send ROLLBACK
        Ok(())
    }
}