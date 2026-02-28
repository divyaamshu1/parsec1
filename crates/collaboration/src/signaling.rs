//! Signaling stubs for P2P connections

use crate::Result;

#[derive(Debug, Clone)]
pub struct SignalingServer;

#[derive(Debug, Clone)]
pub struct SignalingClient;

#[derive(Debug, Clone)]
pub enum SignalMessage {
    Connect,
    Disconnect,
    Data(Vec<u8>),
}

impl SignalingServer {
    pub async fn new() -> Result<Self> {
        Ok(SignalingServer)
    }
}

impl SignalingClient {
    pub async fn connect(&self, _url: &str) -> Result<()> {
        Ok(())
    }
}
