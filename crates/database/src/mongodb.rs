//! MongoDB database driver

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use mongodb::{Client, Database, Collection, options::ClientOptions};
use mongodb::bson::{Document, Bson};
use tracing::{info, warn, debug};

use crate::DatabaseStats;

use crate::{DatabaseConnection, DatabaseType, QueryResult, Value, TableInfo, TableSchema, ColumnInfo};

/// MongoDB connection
pub struct MongoDBConnection {
    name: String,
    connection_string: String,
    database_name: String,
    client: Option<Client>,
    database: Option<Database>,
    connected: bool,
}

impl MongoDBConnection {
    /// Create a new MongoDB connection
    pub fn new(name: String, connection_string: String, database_name: String) -> Self {
        Self {
            name,
            connection_string,
            database_name,
            client: None,
            database: None,
            connected: false,
        }
    }

    /// Convert BSON to our Value enum
    fn bson_to_value(&self, bson: &Bson) -> Value {
        match bson {
            Bson::Null => Value::Null,
            Bson::Boolean(b) => Value::Boolean(*b),
            Bson::Int32(i) => Value::Integer(*i as i64),
            Bson::Int64(i) => Value::Integer(*i),
            Bson::Double(f) => Value::Float(*f),
            Bson::String(s) => Value::String(s.clone()),
            Bson::Binary(b) => Value::Binary(b.bytes.clone()),
            Bson::Array(arr) => {
                let values = arr.iter().map(|v| self.bson_to_value(v)).collect();
                Value::Array(values)
            }
            Bson::Document(doc) => {
                let mut map = HashMap::new();
                for (k, v) in doc {
                    map.insert(k.clone(), self.bson_to_value(v));
                }
                Value::Object(map)
            }
            _ => Value::Null,
        }
    }
}

#[async_trait]
impl DatabaseConnection for MongoDBConnection {
    fn db_type(&self) -> DatabaseType {
        DatabaseType::MongoDB
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn connection_string(&self) -> String {
        self.connection_string.clone()
    }

    async fn connect(&mut self) -> Result<()> {
        let client_options = ClientOptions::parse(&self.connection_string).await?;
        let client = Client::with_options(client_options)?;

        // Test connection
        client.list_database_names().await?;

        let database = client.database(&self.database_name);

        self.client = Some(client);
        self.database = Some(database);
        self.connected = true;

        info!("Connected to MongoDB: {}/{}", self.name, self.database_name);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.client = None;
        self.database = None;
        self.connected = false;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }

    async fn execute_query(&self, query: &str) -> Result<QueryResult> {
        // MongoDB doesn't use SQL - this is a simplified implementation
        // In reality, you'd use MongoDB's query language
        Err(anyhow!("MongoDB doesn't support SQL queries"))
    }

    async fn execute_update(&self, _query: &str) -> Result<u64> {
        Err(anyhow!("MongoDB doesn't support SQL updates"))
    }

    async fn list_tables(&self) -> Result<Vec<TableInfo>> {
        let db = self.database.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let collection_names = db.list_collection_names().await?;

        let tables = collection_names.into_iter()
            .map(|name| TableInfo {
                name,
                schema: None,
                row_count: None,
                size_bytes: None,
                created_at: None,
            })
            .collect();

        Ok(tables)
    }

    async fn get_table_schema(&self, table: &str) -> Result<TableSchema> {
        // MongoDB is schemaless, so we sample a document
        let db = self.database.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let collection = db.collection::<Document>(table);
        
        if let Some(doc) = collection.find_one(Document::new()).await? {
            let mut columns = Vec::new();
            for (key, value) in doc {
                let data_type = match value {
                    Bson::Null => "null",
                    Bson::Boolean(_) => "boolean",
                    Bson::Int32(_) | Bson::Int64(_) => "integer",
                    Bson::Double(_) => "double",
                    Bson::String(_) => "string",
                    Bson::Binary(_) => "binary",
                    Bson::Array(_) => "array",
                    Bson::Document(_) => "object",
                    _ => "unknown",
                }.to_string();

                columns.push(ColumnInfo {
                    name: key.clone(),
                    data_type,
                    nullable: true,
                    default: None,
                    is_primary_key: key == "_id",
                    is_unique: key == "_id",
                });
            }

            Ok(TableSchema {
                name: table.to_string(),
                columns,
                primary_key: vec!["_id".to_string()],
                foreign_keys: Vec::new(),
                indexes: Vec::new(),
            })
        } else {
            Ok(TableSchema {
                name: table.to_string(),
                columns: Vec::new(),
                primary_key: Vec::new(),
                foreign_keys: Vec::new(),
                indexes: Vec::new(),
            })
        }
    }

    async fn begin_transaction(&self) -> Result<Box<dyn crate::Transaction>> {
        // MongoDB transactions require replica set
        Err(anyhow!("MongoDB transactions not supported in this implementation"))
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