#![allow(dead_code, unused_imports)]

//! Monitoring traces stub

use serde::{Serialize, Deserialize};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceIdStub(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanIdStub(pub u64);

#[derive(Debug, Clone)]
pub struct Trace {
    pub trace_id: TraceIdStub,
}

pub struct TracingProvider;

impl TracingProvider {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}