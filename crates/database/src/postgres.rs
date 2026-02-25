//! PostgreSQL database driver

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio_postgres::{Client, NoTls, Row};
use tracing::{info, warn, debug};

use crate::{DatabaseConnection, DatabaseType, QueryResult, Value, TableInfo, TableSchema, ColumnInfo, ForeignKey, IndexInfo, DatabaseStats};

/// PostgreSQL connection
pub struct PostgresConnection {
    name: String,
    connection_string: String,
    client: Option<Client>,
    connected: bool,
    stats: ConnectionStats,
}

#[derive(Default)]
struct ConnectionStats {
    queries_executed: u64,
    total_query_time: Duration,
    last_connect: Option<chrono::DateTime<chrono::Utc>>,
}

impl PostgresConnection {
    /// Create a new PostgreSQL connection
    pub fn new(name: String, connection_string: String) -> Self {
        Self {
            name,
            connection_string,
            client: None,
            connected: false,
            stats: ConnectionStats::default(),
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
            format!("postgresql://{}:{}@{}:{}/{}", user, pass, host, port, database)
        } else {
            format!("postgresql://{}@{}:{}/{}", user, host, port, database)
        };

        Self::new(name, connection_string)
    }

    /// Convert PostgreSQL row to Value
    fn row_to_values(&self, row: &Row, columns: &[String]) -> Vec<Value> {
        let mut values = Vec::new();

        for (i, _col) in columns.iter().enumerate() {
            values.push(self.pg_value_to_value(row, i));
        }

        values
    }

    /// Convert PostgreSQL value to our Value enum
    fn pg_value_to_value(&self, row: &Row, idx: usize) -> Value {
        // Try common types
        if let Ok(v) = row.try_get::<_, bool>(idx) {
            return Value::Boolean(v);
        }
        if let Ok(v) = row.try_get::<_, i32>(idx) {
            return Value::Integer(v as i64);
        }
        if let Ok(v) = row.try_get::<_, i64>(idx) {
            return Value::Integer(v);
        }
        if let Ok(v) = row.try_get::<_, f64>(idx) {
            return Value::Float(v);
        }
        if let Ok(v) = row.try_get::<_, String>(idx) {
            return Value::String(v);
        }
        if let Ok(v) = row.try_get::<_, Vec<u8>>(idx) {
            return Value::Binary(v);
        }

        Value::Null
    }
}

#[async_trait]
impl DatabaseConnection for PostgresConnection {
    fn db_type(&self) -> DatabaseType {
        DatabaseType::PostgreSQL
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn connection_string(&self) -> String {
        self.connection_string.clone()
    }

    async fn connect(&mut self) -> Result<()> {
        let (client, connection) = tokio_postgres::connect(&self.connection_string, NoTls).await?;

        // Spawn connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                warn!("PostgreSQL connection error: {}", e);
            }
        });

        self.client = Some(client);
        self.connected = true;
        self.stats.last_connect = Some(chrono::Utc::now());

        info!("Connected to PostgreSQL database: {}", self.name);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.client = None;
        self.connected = false;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }

    async fn execute_query(&self, query: &str) -> Result<QueryResult> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let start = std::time::Instant::now();

        // Determine if it's a SELECT query
        let is_select = query.trim().to_uppercase().starts_with("SELECT");

        if is_select {
            let rows = client.query(query, &[]).await?;
            let columns = if !rows.is_empty() {
                rows[0].columns().iter().map(|c| c.name().to_string()).collect()
            } else {
                Vec::new()
            };

            let values = rows.iter()
                .map(|row| self.row_to_values(row, &columns))
                .collect();

            Ok(QueryResult {
                columns,
                rows: values,
                rows_affected: 0,
                execution_time: start.elapsed(),
            })
        } else {
            let rows_affected = client.execute(query, &[]).await?;

            Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                rows_affected,
                execution_time: start.elapsed(),
            })
        }
    }

    async fn execute_update(&self, query: &str) -> Result<u64> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let rows_affected = client.execute(query, &[]).await?;
        Ok(rows_affected)
    }

    async fn list_tables(&self) -> Result<Vec<TableInfo>> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let rows = client.query(
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'",
            &[]
        ).await?;

        let mut tables = Vec::new();
        for row in rows {
            let name: String = row.get(0);
            
            // Get row count
            let count_row = client.query_one(
                &format!("SELECT COUNT(*) FROM {}", name),
                &[]
            ).await.ok();
            let row_count = count_row.map(|r| r.get::<_, i64>(0) as u64);

            tables.push(TableInfo {
                name,
                schema: Some("public".to_string()),
                row_count,
                size_bytes: None,
                created_at: None,
            });
        }

        Ok(tables)
    }

    async fn get_table_schema(&self, table: &str) -> Result<TableSchema> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let rows = client.query(
            "SELECT 
                column_name, 
                data_type, 
                is_nullable,
                column_default
            FROM information_schema.columns 
            WHERE table_name = $1
            ORDER BY ordinal_position",
            &[&table]
        ).await?;

        let mut columns = Vec::new();
        for row in rows {
            columns.push(ColumnInfo {
                name: row.get(0),
                data_type: row.get(1),
                nullable: row.get::<_, String>(2) == "YES",
                default: row.get(3),
                is_primary_key: false,
                is_unique: false,
            });
        }

        // Get primary key
        let pk_rows = client.query(
            "SELECT
                kcu.column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu
                ON tc.constraint_name = kcu.constraint_name
            WHERE tc.table_name = $1
                AND tc.constraint_type = 'PRIMARY KEY'",
            &[&table]
        ).await?;

        let primary_key = pk_rows.iter().map(|r| r.get(0)).collect();

        // Get foreign keys
        let fk_rows = client.query(
            "SELECT
                kcu.column_name,
                ccu.table_name AS foreign_table_name,
                ccu.column_name AS foreign_column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu
                ON tc.constraint_name = kcu.constraint_name
            JOIN information_schema.constraint_column_usage ccu
                ON tc.constraint_name = ccu.constraint_name
            WHERE tc.table_name = $1
                AND tc.constraint_type = 'FOREIGN KEY'",
            &[&table]
        ).await?;

        let foreign_keys = fk_rows.iter().map(|row| ForeignKey {
            column: row.get(0),
            foreign_table: row.get(1),
            foreign_column: row.get(2),
            on_delete: None,
            on_update: None,
        }).collect();

        Ok(TableSchema {
            name: table.to_string(),
            columns,
            primary_key,
            foreign_keys,
            indexes: Vec::new(),
        })
    }

    async fn begin_transaction(&self) -> Result<Box<dyn crate::Transaction>> {
        Err(anyhow!("Transactions not yet implemented for PostgreSQL"))
    }

    async fn get_stats(&self) -> Result<DatabaseStats> {
        Ok(DatabaseStats {
            connection_count: if self.connected { 1 } else { 0 },
            active_queries: 0,
            total_queries: self.stats.queries_executed,
            average_query_time: if self.stats.queries_executed > 0 {
                self.stats.total_query_time / self.stats.queries_executed as u32
            } else {
                Duration::from_secs(0)
            },
            cache_hit_ratio: None,
            disk_usage: None,
            memory_usage: None,
        })
    }
}

/// PostgreSQL transaction - Note: Not currently implemented due to lifetime constraints
#[allow(dead_code)]
pub struct PostgresTransaction {
    _placeholder: String,
}

#[allow(dead_code)]
#[async_trait]
impl crate::Transaction for PostgresTransaction {
    async fn execute(&self, _query: &str) -> Result<QueryResult> {
        Err(anyhow!("Transactions not yet implemented"))
    }

    async fn commit(&self) -> Result<()> {
        Err(anyhow!("Transactions not yet implemented"))
    }

    async fn rollback(&self) -> Result<()> {
        Err(anyhow!("Transactions not yet implemented"))
    }
}