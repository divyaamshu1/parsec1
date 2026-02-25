//! Asset importers for custom engines

use std::path::Path;

use async_trait::async_trait;

use crate::assets::{AssetImporter, AssetType, AssetPreview};

/// Asset importer factory for custom engines
pub trait AssetImporterFactory: Send + Sync {
    fn create_importers(&self) -> Vec<Box<dyn AssetImporter>>;
    fn supported_asset_types(&self) -> Vec<AssetType>;
}