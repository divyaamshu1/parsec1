//! Redis database driver

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use redis::{Client, AsyncCommands, RedisResult, Value as RedisValue};
use tracing::{info, warn, debug};

use crate::{DatabaseConnection, DatabaseType, QueryResult, Value, TableInfo, TableSchema, ColumnInfo, DatabaseStats};

/// Redis connection
pub struct RedisConnection {
    name: String,
    connection_string: String,
    client: Option<Client>,
    connected: bool,
}

impl RedisConnection {
    /// Create a new Redis connection
    pub fn new(name: String, connection_string: String) -> Self {
        Self {
            name,
            connection_string,
            client: None,
            connected: false,
        }
    }

    /// Create from URL parts
    pub fn from_parts(name: String, host: &str, port: u16, password: Option<&str>) -> Self {
        let connection_string = if let Some(pass) = password {
            format!("redis://:{}@{}:{}/", pass, host, port)
        } else {
            format!("redis://{}:{}/", host, port)
        };

        Self::new(name, connection_string)
    }

    /// Convert Redis value to our Value enum
    fn redis_to_value(&self, value: RedisValue) -> Value {
        match value {
            RedisValue::Nil => Value::Null,
            RedisValue::Int(i) => Value::Integer(i),
            RedisValue::SimpleString(s) => Value::String(s),
            RedisValue::BulkString(b) => Value::Binary(b),
            RedisValue::Array(vals) => {
                let values = vals.into_iter().map(|v| self.redis_to_value(v)).collect();
                Value::Array(values)
            }
            _ => Value::Null,
        }
    }
}

#[async_trait]
#[allow(dependency_on_unit_never_type_fallback)]
impl DatabaseConnection for RedisConnection {
    fn db_type(&self) -> DatabaseType {
        DatabaseType::Redis
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn connection_string(&self) -> String {
        self.connection_string.clone()
    }

    async fn connect(&mut self) -> Result<()> {
        let client = Client::open(self.connection_string.as_str())?;
        
        // Test connection
        let mut conn = client.get_multiplexed_async_connection().await?;
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;

        self.client = Some(client);
        self.connected = true;

        info!("Connected to Redis: {}", self.name);
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
        // Redis doesn't use SQL - execute Redis commands directly
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let mut conn = client.get_multiplexed_async_connection().await?;
        let start = std::time::Instant::now();

        // Parse command (simplified - just split by whitespace)
        let parts: Vec<&str> = query.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow!("Empty command"));
        }

        let cmd_name = parts[0];
        let args = &parts[1..];

        // Execute Redis command
        let mut cmd = redis::cmd(cmd_name);
        for arg in args {
            cmd.arg(arg);
        }

        let result: RedisResult<RedisValue> = cmd.query_async(&mut conn).await;
        
        match result {
            Ok(value) => {
                let our_value = self.redis_to_value(value);
                
                // Convert to tabular format for display
                let columns = vec!["result".to_string()];
                let rows = vec![vec![our_value]];

                Ok(QueryResult {
                    columns,
                    rows,
                    rows_affected: 0,
                    execution_time: start.elapsed(),
                })
            }
            Err(e) => Err(anyhow!("Redis error: {}", e)),
        }
    }

    async fn execute_update(&self, query: &str) -> Result<u64> {
        // For Redis, updates are just commands
        let result = self.execute_query(query).await?;
        Ok(result.rows.len() as u64)
    }

    async fn list_tables(&self) -> Result<Vec<TableInfo>> {
        // Redis doesn't have tables - return keys
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let mut conn = client.get_multiplexed_async_connection().await?;
        let keys: Vec<String> = conn.keys("*").await?;

        let tables = keys.into_iter()
            .map(|name| TableInfo {
                name,
                schema: None,
                row_count: Some(1),
                size_bytes: None,
                created_at: None,
            })
            .collect();

        Ok(tables)
    }

    async fn get_table_schema(&self, _table: &str) -> Result<TableSchema> {
        // Redis is schemaless
        Ok(TableSchema {
            name: "key".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "key".to_string(),
                    data_type: "string".to_string(),
                    nullable: false,
                    default: None,
                    is_primary_key: true,
                    is_unique: true,
                },
                ColumnInfo {
                    name: "value".to_string(),
                    data_type: "string".to_string(),
                    nullable: true,
                    default: None,
                    is_primary_key: false,
                    is_unique: false,
                },
            ],
            primary_key: vec!["key".to_string()],
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
        })
    }

    async fn begin_transaction(&self) -> Result<Box<dyn crate::Transaction>> {
        // Redis supports transactions via MULTI/EXEC
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let mut conn = client.get_multiplexed_async_connection().await?;
        redis::cmd("MULTI").query_async(&mut conn).await?;

        Ok(Box::new(RedisTransaction {
            name: self.name.clone(),
        }))
    }

    async fn get_stats(&self) -> Result<DatabaseStats> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let mut conn = client.get_multiplexed_async_connection().await?;
        let info: String = redis::cmd("INFO").query_async(&mut conn).await?;

        let mut total_queries = 0;
        for line in info.lines() {
            if line.starts_with("total_commands_processed:") {
                if let Some(num) = line.split(':').nth(1) {
                    total_queries = num.parse().unwrap_or(0);
                }
                break;
            }
        }

        Ok(DatabaseStats {
            connection_count: 1,
            active_queries: 0,
            total_queries,
            average_query_time: Duration::from_secs(0),
            cache_hit_ratio: None,
            disk_usage: None,
            memory_usage: None,
        })
    }
}

/// Redis transaction
pub struct RedisTransaction {
    name: String,
}

#[async_trait]
impl crate::Transaction for RedisTransaction {
    async fn execute(&self, query: &str) -> Result<QueryResult> {
        // Would need connection reference
        Err(anyhow!("Transaction execute not implemented"))
    }

    async fn commit(&self) -> Result<()> {
        // Would need to send EXEC
        Ok(())
    }

    async fn rollback(&self) -> Result<()> {
        // Would need to send DISCARD
        Ok(())
    }
}