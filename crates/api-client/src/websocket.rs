//! WebSocket client with full support for real-time APIs

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use futures::{SinkExt, StreamExt};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn, debug};

use crate::APIClientConfig;

/// WebSocket connection
pub struct WebSocketConnection {
    pub url: String,
    pub id: String,
    write_tx: mpsc::UnboundedSender<WebSocketMessage>,
    read_rx: Arc<RwLock<mpsc::UnboundedReceiver<WebSocketMessage>>>,
    connected: Arc<RwLock<bool>>,
}

/// WebSocket message
#[derive(Debug, Clone)]
pub enum WebSocketMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close(Option<u16>, Option<String>),
}

/// WebSocket event
#[derive(Debug, Clone)]
pub enum WebSocketEvent {
    Connected,
    Message(WebSocketMessage),
    Disconnected,
    Error(String),
}

/// WebSocket client
pub struct WebSocketClient {
    connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
    config: APIClientConfig,
}

impl WebSocketClient {
    /// Create new WebSocket client
    pub fn new(config: APIClientConfig) -> Result<Self> {
        Ok(Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    /// Connect to WebSocket server
    pub async fn connect(&self, url: &str) -> Result<WebSocketConnection> {
        let (ws_stream, response) = connect_async(url).await?;
        
        info!("Connected to WebSocket: {} (status: {})", url, response.status());

        let (mut write, mut read) = ws_stream.split();
        let (write_tx, mut write_rx) = mpsc::unbounded_channel::<WebSocketMessage>();
        let (read_tx, read_rx) = mpsc::unbounded_channel::<WebSocketMessage>();

        let id = uuid::Uuid::new_v4().to_string();

        // Spawn write task
        let write_handle = tokio::spawn(async move {
            while let Some(msg) = write_rx.recv().await {
                let ws_message = match msg {
                    WebSocketMessage::Text(text) => Message::Text(text),
                    WebSocketMessage::Binary(data) => Message::Binary(data),
                    WebSocketMessage::Ping(data) => Message::Ping(data),
                    WebSocketMessage::Pong(data) => Message::Pong(data),
                    WebSocketMessage::Close(code, reason) => {
                        if let (Some(c), Some(r)) = (code, reason) {
                            Message::Close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                                code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::from(c),
                                reason: std::borrow::Cow::Owned(r),
                            }))
                        } else {
                            Message::Close(None)
                        }
                    }
                };

                if let Err(e) = write.send(ws_message).await {
                    warn!("WebSocket write error: {}", e);
                    break;
                }
            }
        });

        // Spawn read task
        let read_handle = tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(msg) => {
                        let ws_msg = match msg {
                            Message::Text(text) => WebSocketMessage::Text(text),
                            Message::Binary(data) => WebSocketMessage::Binary(data),
                            Message::Ping(data) => WebSocketMessage::Ping(data),
                            Message::Pong(data) => WebSocketMessage::Pong(data),
                            Message::Close(frame) => {
                                if let Some(f) = frame {
                                    WebSocketMessage::Close(Some(f.code.into()), Some(f.reason.to_string()))
                                } else {
                                    WebSocketMessage::Close(None, None)
                                }
                            }
                            Message::Frame(_) => continue,
                        };
                        let _ = read_tx.send(ws_msg);
                    }
                    Err(e) => {
                        warn!("WebSocket read error: {}", e);
                        break;
                    }
                }
            }
        });

        let connection = WebSocketConnection {
            url: url.to_string(),
            id: id.clone(),
            write_tx,
            read_rx: Arc::new(RwLock::new(read_rx)),
            connected: Arc::new(RwLock::new(true)),
        };

        self.connections.write().await.insert(id, connection.clone());

        Ok(connection)
    }

    /// Disconnect WebSocket
    pub async fn disconnect(&self, id: &str) -> Result<()> {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.remove(id) {
            *conn.connected.write().await = false;
        }
        Ok(())
    }

    /// List all connections
    pub async fn list_connections(&self) -> Vec<String> {
        self.connections.read().await.keys().cloned().collect()
    }
}

impl WebSocketConnection {
    /// Send message
    pub async fn send(&self, message: WebSocketMessage) -> Result<()> {
        if !*self.connected.read().await {
            return Err(anyhow!("WebSocket not connected"));
        }
        self.write_tx.send(message)?;
        Ok(())
    }

    /// Send text message
    pub async fn send_text(&self, text: String) -> Result<()> {
        self.send(WebSocketMessage::Text(text)).await
    }

    /// Send binary message
    pub async fn send_binary(&self, data: Vec<u8>) -> Result<()> {
        self.send(WebSocketMessage::Binary(data)).await
    }

    /// Receive next message
    pub async fn receive(&self) -> Option<WebSocketMessage> {
        let mut rx = self.read_rx.write().await;
        rx.recv().await
    }

    /// Receive messages as stream
    /// Note: async_stream crate not available, use receive() in a loop instead
    pub async fn receive_all(&self) -> Result<Vec<WebSocketMessage>> {
        let mut messages = Vec::new();
        let mut rx = self.read_rx.write().await;
        
        // Collect available messages without blocking indefinitely
        while let Ok(msg) = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            rx.recv()
        ).await {
            if let Some(m) = msg {
                messages.push(m);
            } else {
                break;
            }
        }
        
        Ok(messages)
    }

    /// Ping
    pub async fn ping(&self, data: Vec<u8>) -> Result<()> {
        self.send(WebSocketMessage::Ping(data)).await
    }

    /// Pong
    pub async fn pong(&self, data: Vec<u8>) -> Result<()> {
        self.send(WebSocketMessage::Pong(data)).await
    }

    /// Close connection
    pub async fn close(&self, code: Option<u16>, reason: Option<String>) -> Result<()> {
        self.send(WebSocketMessage::Close(code, reason)).await
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }
}

impl Clone for WebSocketConnection {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            id: self.id.clone(),
            write_tx: self.write_tx.clone(),
            read_rx: self.read_rx.clone(),
            connected: self.connected.clone(),
        }
    }
}