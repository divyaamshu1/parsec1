//! Debug hooks for custom frameworks

use async_trait::async_trait;use anyhow::Result;
use crate::debug::DebugSession;

/// Debug hook trait
#[async_trait]
pub trait DebugHook: Send + Sync {
    fn name(&self) -> &str;
    async fn on_session_start(&self, session: &DebugSession) -> Result<()>;
    async fn on_session_stop(&self, session: &DebugSession) -> Result<()>;
    async fn on_breakpoint(&self, session: &DebugSession) -> Result<()>;
}