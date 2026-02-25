//! Build hooks for custom engines

use async_trait::async_trait;

use crate::build::{BuildConfig, BuildResult, BuildTarget};

/// Build hook trait
#[async_trait]
pub trait BuildHook: Send + Sync {
    fn name(&self) -> &str;
    async fn before_build(&self, config: &BuildConfig) -> Result<()>;
    async fn after_build(&self, result: &BuildResult) -> Result<()>;
    async fn get_build_targets(&self) -> Vec<BuildTarget>;
}