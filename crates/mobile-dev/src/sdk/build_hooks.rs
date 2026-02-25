//! Build hooks for custom frameworks

use async_trait::async_trait;
use anyhow::Result;

use crate::build::{BuildConfig, BuildResult};

/// Build hook trait
#[async_trait]
pub trait BuildHook: Send + Sync {
    fn name(&self) -> &str;
    async fn before_build(&self, config: &BuildConfig) -> Result<()>;
    async fn after_build(&self, result: &BuildResult) -> Result<()>;
}