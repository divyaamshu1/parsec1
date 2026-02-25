pub mod types;
pub mod sources;
pub mod manager;

pub use manager::RegistryManager;
pub use types::{
    ExtensionInfo, InstalledExtension, ExtensionManifest, RegistryError,
    PublisherInfo, ExtensionDependency, CategoryInfo,
    SearchQuery, SearchResult, SortBy, SortOrder,
    ExtensionVersion, DownloadResult, MarketplaceError,
};