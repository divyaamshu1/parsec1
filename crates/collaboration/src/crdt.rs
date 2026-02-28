//! Stub CRDT module for collaboration

use crate::Result;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Document;

#[derive(Debug, Clone)]
pub struct Edit;

#[derive(Debug, Clone)]
pub struct Operation;

#[derive(Debug, Clone)]
pub struct SyncState;

#[derive(Debug, Clone)]
pub struct CrdtManager;

impl CrdtManager {
    pub fn new() -> Self {
        CrdtManager
    }

    pub async fn apply_operation(&self, _op: Operation) -> Result<()> {
        Ok(())
    }
}
