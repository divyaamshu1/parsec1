//! Platform trait for custom platforms

use std::path::PathBuf;

use async_trait::async_trait;

use crate::platform::Platform;

/// Platform extension trait
#[async_trait]
pub trait PlatformExtension: Send + Sync {
    fn name(&self) -> &str;
    async fn detect(&self) -> Option<Box<dyn Platform>>;
}