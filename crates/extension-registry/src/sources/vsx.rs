//! VS Code Marketplace HTTP client

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use reqwest::{Client, header};
use serde_json::{json, Value};

use crate::{
    ExtensionInfo, PublisherInfo,
    SearchQuery, SearchResult, SortBy,
    ExtensionVersion, DownloadResult, MarketplaceError,
};

/// VS Code Marketplace API client
pub struct VSCodeMarketplace {
    /// Base API URL
    base_url: String,
    /// HTTP client
    client: Client,
    /// API key (if any)
    api_key: Option<String>,
    /// User agent
    #[allow(dead_code)]
    user_agent: String,
}

impl VSCodeMarketplace {
    /// Create a new marketplace client
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json;api-version=3.0-preview.1")
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json")
        );

        let client = Client::builder()
            .default_headers(headers)
            .user_agent("Parsec-IDE/0.1")
            .build()
            .unwrap_or_default();

        Self {
            base_url,
            client,
            api_key,
            user_agent: "Parsec-IDE/0.1".to_string(),
        }
    }

    /// Search for extensions
    pub async fn search(&self, query: SearchQuery) -> Result<SearchResult> {
        let url = format!("{}/extensionquery", self.base_url);

        let mut filters = vec![
            json!({
                "criteria": [
                    {
                        "filterType": 8,
                        "value": "Microsoft.VisualStudio.Code"
                    }
                ],
                "pageNumber": query.page,
                "pageSize": query.page_size
            })
        ];

        // Add search text filter
        if !query.text.is_empty() {
            filters[0]["criteria"].as_array_mut().unwrap().push(json!({
                "filterType": 10,
                "value": query.text
            }));
        }

        // Add category filters
        if let Some(categories) = &query.categories {
            for category in categories {
                filters[0]["criteria"].as_array_mut().unwrap().push(json!({
                    "filterType": 4,
                    "value": category
                }));
            }
        }

        // Add tag filters
        if let Some(tags) = &query.tags {
            for tag in tags {
                filters[0]["criteria"].as_array_mut().unwrap().push(json!({
                    "filterType": 5,
                    "value": tag
                }));
            }
        }

        // Add publisher filter
        if let Some(publisher) = &query.publisher {
            filters[0]["criteria"].as_array_mut().unwrap().push(json!({
                "filterType": 1,
                "value": publisher
            }));
        }

        // Set sort order
        let flags = match query.sort_by {
            SortBy::Relevance => 0x2 | 0x4 | 0x8 | 0x80,
            SortBy::Downloads => 0x40 | 0x2 | 0x4 | 0x8 | 0x80,
            SortBy::Rating => 0x10 | 0x2 | 0x4 | 0x8 | 0x80,
            SortBy::Updated => 0x20 | 0x2 | 0x4 | 0x8 | 0x80,
            SortBy::Published => 0x80,
            SortBy::Name => 0x1 | 0x2 | 0x4 | 0x8 | 0x80,
        };

        let body = json!({
            "filters": filters,
            "assetTypes": [
                "Microsoft.VisualStudio.Services.Icons.Default",
                "Microsoft.VisualStudio.Services.Icons.Branding",
                "Microsoft.VisualStudio.Services.Icons.Small"
            ],
            "flags": flags
        });

        let mut request = self.client.post(&url).json(&body);

        if let Some(key) = &self.api_key {
            request = request.header("X-API-Key", key);
        }

        let response = request.send().await
            .map_err(|e| MarketplaceError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!(MarketplaceError::Api(format!("HTTP {}: {}", status, text))));
        }

        let data: Value = response.json().await
            .map_err(|e| MarketplaceError::InvalidResponse(e.to_string()))?;

        self.parse_search_response(data, &query)
    }

    /// Get extension details
    pub async fn get_extension(&self, publisher: &str, name: &str) -> Result<ExtensionInfo> {
        let url = format!("{}/extension/{}/{}", self.base_url, publisher, name);

        let mut request = self.client.get(&url);

        if let Some(key) = &self.api_key {
            request = request.header("X-API-Key", key);
        }

        let response = request.send().await
            .map_err(|e| MarketplaceError::Network(e.to_string()))?;

        if response.status() == 404 {
            return Err(anyhow!(MarketplaceError::NotFound(format!("{}.{}", publisher, name))));
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!(MarketplaceError::Api(format!("HTTP {}: {}", status, text))));
        }

        let data: Value = response.json().await
            .map_err(|e| MarketplaceError::InvalidResponse(e.to_string()))?;

        self.parse_extension_info(data)
    }

    /// Get extension versions
    pub async fn get_versions(&self, publisher: &str, name: &str) -> Result<Vec<ExtensionVersion>> {
        let url = format!("{}/extension/{}/{}/versions", self.base_url, publisher, name);

        let mut request = self.client.get(&url);

        if let Some(key) = &self.api_key {
            request = request.header("X-API-Key", key);
        }

        let response = request.send().await
            .map_err(|e| MarketplaceError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(anyhow!(MarketplaceError::Api(format!("HTTP {}", response.status()))));
        }

        let data: Value = response.json().await
            .map_err(|e| MarketplaceError::InvalidResponse(e.to_string()))?;

        self.parse_versions(data)
    }

    /// Download extension
    pub async fn download_extension(
        &self,
        publisher: &str,
        name: &str,
        version: Option<&str>,
        target: Option<&str>,
    ) -> Result<DownloadResult> {
        let version = version.unwrap_or("latest");
        let mut url = format!(
            "{}/publisher/{}/vsextensions/{}/{}/vspackage",
            self.base_url, publisher, name, version
        );

        if let Some(target) = target {
            url = format!("{}?targetPlatform={}", url, target);
        }

        let mut request = self.client.get(&url);

        if let Some(key) = &self.api_key {
            request = request.header("X-API-Key", key);
        }

        let response = request.send().await
            .map_err(|e| MarketplaceError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(anyhow!(MarketplaceError::Api(format!("HTTP {}", response.status()))));
        }

        // Get filename from Content-Disposition or generate one
        let filename = response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| {
                h.split(';')
                    .find(|p| p.trim().starts_with("filename="))
                    .map(|p| p.trim().trim_start_matches("filename=").trim_matches('"'))
            })
            .unwrap_or(&format!("{}.{}.vsix", name, version))
            .to_string();

        let temp_dir = std::env::temp_dir().join("parsec-extensions");
        std::fs::create_dir_all(&temp_dir)?;

        let local_path = temp_dir.join(filename);
        let bytes = response.bytes().await
            .map_err(|e| MarketplaceError::Network(e.to_string()))?;

        tokio::fs::write(&local_path, bytes).await
            .map_err(|e| MarketplaceError::Network(e.to_string()))?;

        // Calculate integrity hash
        let integrity_hash = self.calculate_hash(&local_path).await?;
        let file_size = std::fs::metadata(&local_path)?.len();

        Ok(DownloadResult {
            extension_id: format!("{}.{}", publisher, name),
            version: version.to_string(),
            local_path,
            file_size,
            integrity_hash: Some(integrity_hash),
        })
    }

    /// Get trending extensions
    pub async fn get_trending(&self, timeframe: &str, limit: usize) -> Result<Vec<ExtensionInfo>> {
        // This uses the gallery API's trending endpoint
        let url = format!("{}/extensionquery", self.base_url);

        let body = json!({
            "filters": [{
                "criteria": [
                    {
                        "filterType": 8,
                        "value": "Microsoft.VisualStudio.Code"
                    },
                    {
                        "filterType": 9,
                        "value": timeframe
                    }
                ],
                "pageNumber": 1,
                "pageSize": limit
            }],
            "flags": 0x2 | 0x4 | 0x8 | 0x80 | 0x100
        });

        let response = self.client.post(&url).json(&body).send().await
            .map_err(|e| MarketplaceError::Network(e.to_string()))?;

        let data: Value = response.json().await
            .map_err(|e| MarketplaceError::InvalidResponse(e.to_string()))?;

        self.parse_search_results(data)
    }

    /// Calculate SHA256 hash of a file
    async fn calculate_hash(&self, path: &PathBuf) -> Result<String> {
        use sha2::{Sha256, Digest};
        use tokio::io::AsyncReadExt;

        let mut file = tokio::fs::File::open(path).await?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 8192];

        loop {
            let n = file.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    /// Parse search response from marketplace
    fn parse_search_response(&self, data: Value, query: &SearchQuery) -> Result<SearchResult> {
        let results = data["results"].as_array().ok_or_else(|| {
            MarketplaceError::InvalidResponse("Missing results array".to_string())
        })?;

        let total = results.get(0)
            .and_then(|r| r["resultMetadata"].as_array())
            .and_then(|meta| meta.iter().find(|m| m["metadataType"].as_str() == Some("ResultCount")))
            .and_then(|m| m["metadataItems"].as_array())
            .and_then(|items| items.first())
            .and_then(|item| item["count"].as_u64())
            .unwrap_or(0) as usize;

        let extensions = self.parse_search_results(data)?;

        Ok(SearchResult {
            total,
            extensions,
            page: query.page,
            page_size: query.page_size,
            has_more: total > query.page * query.page_size,
        })
    }

    /// Parse search results array
    fn parse_search_results(&self, data: Value) -> Result<Vec<ExtensionInfo>> {
        let mut extensions = Vec::new();

        if let Some(results) = data["results"].as_array() {
            for result in results {
                if let Some(extensions_array) = result["extensions"].as_array() {
                    for ext in extensions_array {
                        if let Ok(info) = self.parse_extension_info(ext.clone()) {
                            extensions.push(info);
                        }
                    }
                }
            }
        }

        Ok(extensions)
    }

    /// Parse single extension info
    fn parse_extension_info(&self, data: Value) -> Result<ExtensionInfo> {
        let stats = data["statistics"].as_array().map(|v| v.clone()).unwrap_or_default();

        let downloads = stats.iter()
            .find(|s| s["statisticName"].as_str() == Some("install"))
            .and_then(|s| s["value"].as_f64())
            .unwrap_or(0.0) as u64;

        let rating = stats.iter()
            .find(|s| s["statisticName"].as_str() == Some("averagerating"))
            .and_then(|s| s["value"].as_f64())
            .unwrap_or(0.0) as f32;

        let rating_count = stats.iter()
            .find(|s| s["statisticName"].as_str() == Some("ratingcount"))
            .and_then(|s| s["value"].as_f64())
            .unwrap_or(0.0) as u32;

        let publisher = data["publisher"].as_object().ok_or_else(|| {
            MarketplaceError::InvalidResponse("Missing publisher info".to_string())
        })?;

        let publisher_info = PublisherInfo {
            publisher_id: publisher["publisherId"].as_str().unwrap_or("").to_string(),
            publisher_name: publisher["publisherName"].as_str().unwrap_or("").to_string(),
            display_name: publisher["displayName"].as_str().unwrap_or("").to_string(),
            domain: publisher["domain"].as_str().map(|s| s.to_string()),
            verified: publisher["isDomainVerified"].as_bool().unwrap_or(false),
        };

        let extension_name = data["extensionName"].as_str().unwrap_or("").to_string();
        let extension_id = format!("{}.{}", publisher_info.publisher_name, extension_name);

        let versions = data["versions"].as_array().map(|v| v.clone()).unwrap_or_default();
        let latest_version = versions.first().and_then(|v| v.as_object());

        let version = latest_version
            .and_then(|v| v["version"].as_str())
            .unwrap_or("0.0.0")
            .to_string();

        let last_updated = latest_version
            .and_then(|v| v["lastUpdated"].as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        let release_date = latest_version
            .and_then(|v| v["releaseDate"].as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        let engines = latest_version
            .and_then(|v| v["engines"].as_object())
            .map(|eng| {
                eng.iter()
                    .filter_map(|(k, v)| v.as_str().map(|vs| (k.clone(), vs.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        let categories = data["categories"].as_array()
            .map(|cats| {
                cats.iter()
                    .filter_map(|c| c.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let tags = data["tags"].as_array()
            .map(|t| {
                t.iter()
                    .filter_map(|tag| tag.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let extension_info = ExtensionInfo {
            id: extension_id.clone(),
            name: extension_name.clone(),
            extension_id,
            extension_name,
            display_name: data["displayName"].as_str().unwrap_or("").to_string(),
            publisher: publisher_info.publisher_name.clone(),
            version,
            description: data["shortDescription"].as_str().map(|s| s.to_string()),
            categories,
            tags,
            repository: None,
            homepage: None,
            license: None,
            icon_url: self.find_icon_url(&data),
            readme_url: None,
            changelog_url: None,
            downloads,
            rating,
            rating_count,
            release_date,
            last_updated,
            dependencies: Vec::new(),
            extension_pack: Vec::new(),
            engines,
            categories_labels: Vec::new(),
        };

        Ok(extension_info)
    }

    /// Parse versions response
    fn parse_versions(&self, data: Value) -> Result<Vec<ExtensionVersion>> {
        let mut versions = Vec::new();

        if let Some(versions_array) = data.as_array() {
            for version in versions_array {
                let version_str = version["version"].as_str().unwrap_or("").to_string();
                let target_platform = version["targetPlatform"].as_str().map(|s| s.to_string());
                let engine_version = version["engines"]["vscode"].as_str().unwrap_or("*").to_string();
                let asset_uri = version["assetUri"].as_str().unwrap_or("").to_string();
                let file_size = version["files"].as_array()
                    .and_then(|files| files.first())
                    .and_then(|f| f["size"].as_u64())
                    .unwrap_or(0);
                let release_date = version["releaseDate"].as_str()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(chrono::Utc::now);
                let is_pre_release = version["isPreRelease"].as_bool().unwrap_or(false);

                versions.push(ExtensionVersion {
                    version: version_str,
                    target_platform,
                    engine_version,
                    asset_uri,
                    file_size,
                    release_date,
                    is_pre_release,
                });
            }
        }

        Ok(versions)
    }

    /// Find icon URL in assets
    fn find_icon_url(&self, data: &Value) -> Option<String> {
        if let Some(assets) = data["versions"].as_array()?.first()?.get("files")?.as_array() {
            for asset in assets {
                if let Some(asset_type) = asset["assetType"].as_str() {
                    if asset_type == "Microsoft.VisualStudio.Services.Icons.Default" {
                        return asset["source"].as_str().map(|s| s.to_string());
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_marketplace_client() {
        let client = VSCodeMarketplace::new(
            "https://marketplace.visualstudio.com/_apis/public/gallery".to_string(),
            None,
        );

        // This would be an actual API call in a real test
        // For now, just verify client creation
        assert_eq!(client.base_url, "https://marketplace.visualstudio.com/_apis/public/gallery");
    }
}