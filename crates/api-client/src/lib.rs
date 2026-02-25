//! Universal API Development Tools for Parsec IDE
//!
//! This crate provides comprehensive API testing and development tools supporting
//! REST, GraphQL, WebSocket, gRPC, and more.

#![allow(dead_code, unused_imports, unused_variables)]

pub mod rest;
pub mod graphql;
pub mod websocket;
pub mod grpc;
pub mod collections;
pub mod environments;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::sync::{RwLock, Mutex};
use tracing::{info, warn, debug};

pub use rest::*;
pub use graphql::*;
pub use websocket::*;
pub use grpc::*;
pub use collections::*;
pub use environments::*;

/// Main API client manager
pub struct APIClientManager {
    rest_client: Arc<rest::RESTClient>,
    graphql_client: Arc<graphql::GraphQLClient>,
    websocket_client: Arc<websocket::WebSocketClient>,
    grpc_client: Arc<grpc::GRPCClient>,
    collections: Arc<collections::CollectionManager>,
    environments: Arc<environments::EnvironmentManager>,
    history: Arc<RwLock<Vec<RequestHistory>>>,
    config: APIClientConfig,
}

/// API client configuration
#[derive(Debug, Clone)]
pub struct APIClientConfig {
    pub max_history: usize,
    pub timeout_seconds: u64,
    pub follow_redirects: bool,
    pub max_redirects: u32,
    pub validate_ssl: bool,
    pub proxy_url: Option<String>,
    pub user_agent: String,
}

impl Default for APIClientConfig {
    fn default() -> Self {
        Self {
            max_history: 100,
            timeout_seconds: 30,
            follow_redirects: true,
            max_redirects: 5,
            validate_ssl: true,
            proxy_url: None,
            user_agent: "Parsec-API-Client/1.0".to_string(),
        }
    }
}

/// Request history entry
#[derive(Debug, Clone)]
pub struct RequestHistory {
    pub id: String,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub duration: std::time::Duration,
    pub request_size: usize,
    pub response_size: usize,
    pub collection: Option<String>,
    pub environment: Option<String>,
}

impl APIClientManager {
    /// Create a new API client manager
    pub fn new(config: APIClientConfig) -> Result<Self> {
        Ok(Self {
            rest_client: Arc::new(rest::RESTClient::new(config.clone())?),
            graphql_client: Arc::new(graphql::GraphQLClient::new(config.clone())?),
            websocket_client: Arc::new(websocket::WebSocketClient::new(config.clone())?),
            grpc_client: Arc::new(grpc::GRPCClient::new(config.clone())?),
            collections: Arc::new(collections::CollectionManager::new()?),
            environments: Arc::new(environments::EnvironmentManager::new()?),
            history: Arc::new(RwLock::new(Vec::with_capacity(config.max_history))),
            config,
        })
    }

    /// Send REST request
    pub async fn send_rest_request(&self, request: rest::RESTRequest) -> Result<rest::RESTResponse> {
        let start = std::time::Instant::now();
        
        // Apply environment variables
        let request = self.environments.apply_to_request(request).await?;
        
        // Send request
        let response = self.rest_client.send(request).await?;
        
        Ok(response)
    }

    /// Send GraphQL query
    pub async fn send_graphql_query(&self, query: graphql::GraphQLQuery) -> Result<graphql::GraphQLResponse> {
        let start = std::time::Instant::now();
        
        let query = self.environments.apply_to_graphql(query).await?;
        let response = self.graphql_client.query(query).await?;
        
        Ok(response)
    }

    /// Connect WebSocket
    pub async fn connect_websocket(&self, url: &str) -> Result<websocket::WebSocketConnection> {
        let url = self.environments.resolve_variables(url).await?;
        self.websocket_client.connect(&url).await
    }

    /// Send gRPC request
    pub async fn send_grpc_request(&self, request: grpc::GRPCRequest) -> Result<grpc::GRPCResponse> {
        let start = std::time::Instant::now();
        
        let request = self.environments.apply_to_grpc(request).await?;
        let response = self.grpc_client.send(request).await?;
        
        Ok(response)
    }

    /// Record request in history
    async fn record_history(&self, entry: impl Into<RequestHistory>) {
        let mut history = self.history.write().await;
        history.insert(0, entry.into());
        
        if history.len() > self.config.max_history {
            history.pop();
        }
    }

    /// Get request history
    pub async fn get_history(&self, limit: Option<usize>) -> Vec<RequestHistory> {
        let history = self.history.read().await;
        let limit = limit.unwrap_or(history.len());
        history.iter().take(limit).cloned().collect()
    }

    /// Clear history
    pub async fn clear_history(&self) {
        self.history.write().await.clear();
    }

    /// Import collection
    pub async fn import_collection(&self, path: &Path, format: CollectionFormat) -> Result<()> {
        self.collections.import(path, format).await
    }

    /// Export collection
    pub async fn export_collection(&self, name: &str, format: CollectionFormat) -> Result<String> {
        self.collections.export(name, format).await
    }

    /// Set active environment
    pub async fn set_environment(&self, name: Option<&str>) -> Result<()> {
        self.environments.set_active(name).await
    }

    /// Get all environments
    pub async fn list_environments(&self) -> Vec<String> {
        self.environments.list().await
    }
}

/// Collection format
#[derive(Debug, Clone, Copy)]
pub enum CollectionFormat {
    PostmanV2,
    OpenAPI3,
    HAR,
    Curl,
    Insomnia,
    Bruno,
}