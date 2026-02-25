//! MySQL database driver

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use mysql_async::{Pool, Conn, Row, prelude::*};
use tracing::{info, warn, debug};

use crate::{DatabaseConnection, DatabaseType, QueryResult, Value, TableInfo, TableSchema, ColumnInfo, DatabaseStats};

/// MySQL connection
pub struct MySQLConnection {
    name: String,
    connection_string: String,
    pool: Option<Pool>,
    connected: bool,
}

impl MySQLConnection {
    /// Create a new MySQL connection
    pub fn new(name: String, connection_string: String) -> Self {
        Self {
            name,
            connection_string,
            pool: None,
            connected: false,
        }
    }

    /// Create from URL parts
    pub fn from_parts(
        name: String,
        host: &str,
        port: u16,
        database: &str,
        user: &str,
        password: Option<&str>,
    ) -> Self {
        let connection_string = if let Some(pass) = password {
            format!("mysql://{}:{}@{}:{}/{}", user, pass, host, port, database)
        } else {
            format!("mysql://{}@{}:{}/{}", user, host, port, database)
        };

        Self::new(name, connection_string)
    }

    /// Convert MySQL row to Value
    fn row_to_values(&self, row: Row, columns: &[String]) -> Vec<Value> {
        let mut values = Vec::new();

        for (i, col) in columns.iter().enumerate() {
            if let Some(val) = row.get_opt::<String, _>(i) {
                match val {
                    Ok(v) => values.push(Value::String(v)),
                    Err(_) => values.push(Value::Null),
                }
            } else {
                values.push(Value::Null);
            }
        }

        values
    }
}

#[async_trait]
impl DatabaseConnection for MySQLConnection {
    fn db_type(&self) -> DatabaseType {
        DatabaseType::MySQL
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn connection_string(&self) -> String {
        self.connection_string.clone()
    }

    async fn connect(&mut self) -> Result<()> {
        let pool = Pool::new(self.connection_string.as_str());
        
        // Test connection
        let mut conn = pool.get_conn().await?;
        conn.ping().await?;

        self.pool = Some(pool);
        self.connected = true;

        info!("Connected to MySQL database: {}", self.name);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(pool) = self.pool.take() {
            pool.disconnect().await?;
        }
        self.pool = None;
        self.connected = false;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }

    async fn execute_query(&self, query: &str) -> Result<QueryResult> {
        let pool = self.pool.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let mut conn = pool.get_conn().await?;
        let start = std::time::Instant::now();

        // Determine if it's a SELECT query
        let is_select = query.trim().to_uppercase().starts_with("SELECT");

        if is_select {
            let rows: Vec<Row> = conn.query(query).await?;
            
            let columns = if !rows.is_empty() {
                // Get column names from first row
                rows[0].columns().iter().map(|c| c.name_str().to_string()).collect()
            } else {
                Vec::new()
            };

            let values = rows.into_iter()
                .map(|row| self.row_to_values(row, &columns))
                .collect();

            Ok(QueryResult {
                columns,
                rows: values,
                rows_affected: 0,
                execution_time: start.elapsed(),
            })
        } else {
            let result = conn.exec_iter(query, ()).await?;
            let rows_affected = result.affected_rows();

            Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                rows_affected,
                execution_time: start.elapsed(),
            })
        }
    }

    async fn execute_update(&self, query: &str) -> Result<u64> {
        let pool = self.pool.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let mut conn = pool.get_conn().await?;
        let result = conn.exec_iter(query, ()).await?;
        Ok(result.affected_rows())
    }

    async fn list_tables(&self) -> Result<Vec<TableInfo>> {
        let pool = self.pool.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let mut conn = pool.get_conn().await?;
        let rows: Vec<Row> = conn.query("SHOW TABLES").await?;

        let mut tables = Vec::new();
        for row in rows {
            if let Some(name) = row.get::<String, _>(0) {
                tables.push(TableInfo {
                    name,
                    schema: None,
                    row_count: None,
                    size_bytes: None,
                    created_at: None,
                });
            }
        }

        Ok(tables)
    }

    async fn get_table_schema(&self, table: &str) -> Result<TableSchema> {
        let pool = self.pool.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let mut conn = pool.get_conn().await?;
        let rows: Vec<Row> = conn.query(format!("DESCRIBE {}", table)).await?;

        let mut columns = Vec::new();
        for row in rows {
            let name: String = row.get(0).unwrap();
            let data_type: String = row.get(1).unwrap();
            let nullable: String = row.get(2).unwrap_or("YES".to_string());
            let key: String = row.get(3).unwrap_or("".to_string());
            let default: Option<String> = row.get(4);
            
            columns.push(ColumnInfo {
                name,
                data_type,
                nullable: nullable == "YES",
                default,
                is_primary_key: key == "PRI",
                is_unique: key == "UNI",
            });
        }

        Ok(TableSchema {
            name: table.to_string(),
            columns,
            primary_key: Vec::new(),
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
        })
    }

    async fn begin_transaction(&self) -> Result<Box<dyn crate::Transaction>> {
        Err(anyhow!("Transactions not yet implemented for MySQL"))
    }

    async fn get_stats(&self) -> Result<DatabaseStats> {
        Ok(DatabaseStats {
            connection_count: if self.connected { 1 } else { 0 },
            active_queries: 0,
            total_queries: 0,
            average_query_time: Duration::from_secs(0),
            cache_hit_ratio: None,
            disk_usage: None,
            memory_usage: None,
        })
    }
}

/// MySQL transaction - Not yet implemented
pub struct MySQLTransaction {
    _placeholder: String,
}

#[allow(dead_code)]
#[async_trait]
impl crate::Transaction for MySQLTransaction {
    async fn execute(&self, _query: &str) -> Result<QueryResult> {
        Err(anyhow!("Transactions not yet implemented for MySQL"))
    }

    async fn commit(&self) -> Result<()> {
        Err(anyhow!("Transactions not yet implemented for MySQL"))
    }

    async fn rollback(&self) -> Result<()> {
        Err(anyhow!("Transactions not yet implemented for MySQL"))
    }
}