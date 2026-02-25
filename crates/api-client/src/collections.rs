//! API collections manager (Postman/Insomnia/Bruno collections)

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use tracing::{info, warn, debug};

use crate::rest::{RESTRequest, HTTPMethod, Auth, RequestBody};
use crate::environments::EnvironmentManager;
use crate::CollectionFormat;

/// API collection
#[derive(Debug, Clone)]
pub struct APICollection {
    pub name: String,
    pub description: Option<String>,
    pub folders: Vec<CollectionFolder>,
    pub requests: Vec<CollectionRequest>,
    pub variables: HashMap<String, String>,
    pub auth: Option<Auth>,
    pub version: String,
}

/// Collection folder
#[derive(Debug, Clone)]
pub struct CollectionFolder {
    pub name: String,
    pub description: Option<String>,
    pub requests: Vec<CollectionRequest>,
    pub folders: Vec<CollectionFolder>,
}

/// Collection request
#[derive(Debug, Clone)]
pub struct CollectionRequest {
    pub name: String,
    pub method: HTTPMethod,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub body: Option<RequestBody>,
    pub auth: Option<Auth>,
    pub tests: Option<String>,
    pub scripts: Option<Scripts>,
}

/// Request scripts
#[derive(Debug, Clone)]
pub struct Scripts {
    pub pre_request: Option<String>,
    pub post_response: Option<String>,
}

/// Collection manager
pub struct CollectionManager {
    collections: Arc<tokio::sync::RwLock<HashMap<String, APICollection>>>,
    active_collection: Arc<tokio::sync::RwLock<Option<String>>>,
}

impl CollectionManager {
    /// Create new collection manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            collections: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            active_collection: Arc::new(tokio::sync::RwLock::new(None)),
        })
    }

    /// Import collection from file
    pub async fn import(&self, path: &Path, format: CollectionFormat) -> Result<()> {
        let content = fs::read_to_string(path)?;
        
        let collection = match format {
            CollectionFormat::PostmanV2 => self.import_postman_v2(&content)?,
            CollectionFormat::OpenAPI3 => self.import_openapi_v3(&content)?,
            CollectionFormat::HAR => self.import_har(&content)?,
            CollectionFormat::Curl => self.import_curl(&content)?,
            CollectionFormat::Insomnia => self.import_insomnia(&content)?,
            CollectionFormat::Bruno => self.import_bruno(&content)?,
        };

        self.collections.write().await.insert(collection.name.clone(), collection);
        Ok(())
    }

    /// Export collection
    pub async fn export(&self, name: &str, format: CollectionFormat) -> Result<String> {
        let collections = self.collections.read().await;
        let collection = collections.get(name)
            .ok_or_else(|| anyhow!("Collection not found: {}", name))?;

        match format {
            CollectionFormat::PostmanV2 => self.export_postman_v2(collection),
            CollectionFormat::OpenAPI3 => self.export_openapi_v3(collection),
            CollectionFormat::HAR => self.export_har(collection),
            CollectionFormat::Curl => self.export_curl(collection),
            CollectionFormat::Insomnia => self.export_insomnia(collection),
            CollectionFormat::Bruno => self.export_bruno(collection),
        }
    }

    /// Import Postman v2 collection
    fn import_postman_v2(&self, content: &str) -> Result<APICollection> {
        let json: Value = serde_json::from_str(content)?;
        
        let name = json["info"]["name"].as_str().unwrap_or("Unnamed").to_string();
        let description = json["info"]["description"].as_str().map(|s| s.to_string());
        
        // Parse items (simplified)
        let folders: Vec<CollectionFolder> = Vec::new();
        let mut requests = Vec::new();

        if let Some(items) = json["item"].as_array() {
            for item in items {
                if let Some(_sub_items) = item["item"].as_array() {
                    // This is a folder
                    // Would parse recursively
                } else {
                    // This is a request
                    if let Ok(req) = self.parse_postman_request(item) {
                        requests.push(req);
                    }
                }
            }
        }

        Ok(APICollection {
            name,
            description,
            folders,
            requests,
            variables: HashMap::new(),
            auth: None,
            version: "2.1.0".to_string(),
        })
    }

    /// Parse Postman request
    fn parse_postman_request(&self, item: &Value) -> Result<CollectionRequest> {
        let name = item["name"].as_str().unwrap_or("Unnamed").to_string();
        let request = &item["request"];

        let method = request["method"].as_str().unwrap_or("GET").into();
        let url = request["url"]["raw"].as_str().unwrap_or("").to_string();

        // Parse headers
        let mut headers = HashMap::new();
        if let Some(header_array) = request["header"].as_array() {
            for h in header_array {
                if let (Some(key), Some(value)) = (h["key"].as_str(), h["value"].as_str()) {
                    headers.insert(key.to_string(), value.to_string());
                }
            }
        }

        // Parse body
        let body = if let Some(body_obj) = request["body"].as_object() {
            match body_obj["mode"].as_str() {
                Some("raw") => {
                    if let Some(raw) = body_obj["raw"].as_str() {
                        if headers.get("Content-Type").map(|ct| ct.contains("json")).unwrap_or(false) {
                            if let Ok(json) = serde_json::from_str(raw) {
                                Some(RequestBody::JSON(json))
                            } else {
                                Some(RequestBody::Text(raw.to_string()))
                            }
                        } else {
                            Some(RequestBody::Text(raw.to_string()))
                        }
                    } else {
                        None
                    }
                }
                Some("formdata") => {
                    let _fields: Vec<(String, String)> = Vec::new();
                    // Would parse form data
                    None
                }
                _ => None,
            }
        } else {
            None
        };

        Ok(CollectionRequest {
            name,
            method,
            url,
            headers,
            params: HashMap::new(),
            body,
            auth: None,
            tests: None,
            scripts: None,
        })
    }

    /// Import OpenAPI v3 spec
    fn import_openapi_v3(&self, content: &str) -> Result<APICollection> {
        // Parse OpenAPI spec and convert to collection
        Err(anyhow!("OpenAPI import not yet implemented"))
    }

    /// Import HAR file
    fn import_har(&self, content: &str) -> Result<APICollection> {
        // Parse HAR file and convert to collection
        Err(anyhow!("HAR import not yet implemented"))
    }

    /// Import curl command
    fn import_curl(&self, content: &str) -> Result<APICollection> {
        // Parse curl command and convert to collection
        Err(anyhow!("Curl import not yet implemented"))
    }

    /// Import Insomnia collection
    fn import_insomnia(&self, content: &str) -> Result<APICollection> {
        // Parse Insomnia export
        Err(anyhow!("Insomnia import not yet implemented"))
    }

    /// Import Bruno collection
    fn import_bruno(&self, content: &str) -> Result<APICollection> {
        // Parse Bruno collection (YAML)
        Err(anyhow!("Bruno import not yet implemented"))
    }

    /// Export Postman v2 collection
    fn export_postman_v2(&self, collection: &APICollection) -> Result<String> {
        // Convert to Postman format
        Err(anyhow!("Postman export not yet implemented"))
    }

    /// Export OpenAPI v3 spec
    fn export_openapi_v3(&self, collection: &APICollection) -> Result<String> {
        // Convert to OpenAPI
        Err(anyhow!("OpenAPI export not yet implemented"))
    }

    /// Export HAR file
    fn export_har(&self, collection: &APICollection) -> Result<String> {
        // Convert to HAR
        Err(anyhow!("HAR export not yet implemented"))
    }

    /// Export curl commands
    fn export_curl(&self, collection: &APICollection) -> Result<String> {
        // Generate curl commands
        let mut output = String::new();
        
        for req in &collection.requests {
            output.push_str(&format!("curl -X {} '{}'", 
                req.method.as_str(), req.url));
            
            for (k, v) in &req.headers {
                output.push_str(&format!(" -H '{}: {}'", k, v));
            }
            
            if let Some(body) = &req.body {
                match body {
                    RequestBody::JSON(json) => {
                        output.push_str(&format!(" -d '{}'", serde_json::to_string(json)?));
                    }
                    RequestBody::Text(text) => {
                        output.push_str(&format!(" -d '{}'", text));
                    }
                    _ => {}
                }
            }
            
            output.push_str("\n\n");
        }

        Ok(output)
    }

    /// Export Insomnia collection
    fn export_insomnia(&self, collection: &APICollection) -> Result<String> {
        // Convert to Insomnia format
        Err(anyhow!("Insomnia export not yet implemented"))
    }

    /// Export Bruno collection
    fn export_bruno(&self, collection: &APICollection) -> Result<String> {
        // Convert to Bruno format (YAML)
        Err(anyhow!("Bruno export not yet implemented"))
    }

    /// Get collection by name
    pub async fn get(&self, name: &str) -> Option<APICollection> {
        self.collections.read().await.get(name).cloned()
    }

    /// List all collections
    pub async fn list(&self) -> Vec<String> {
        self.collections.read().await.keys().cloned().collect()
    }

    /// Set active collection
    pub async fn set_active(&self, name: Option<&str>) {
        *self.active_collection.write().await = name.map(|s| s.to_string());
    }

    /// Get active collection
    pub async fn active(&self) -> Option<APICollection> {
        let active = self.active_collection.read().await.clone();
        if let Some(name) = active {
            self.get(&name).await
        } else {
            None
        }
    }

    /// Run collection tests
    pub async fn run_tests(&self, name: &str) -> Result<TestResults> {
        // Would execute all requests and run test scripts
        Err(anyhow!("Test execution not yet implemented"))
    }
}

/// Test results
#[derive(Debug, Clone)]
pub struct TestResults {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration: std::time::Duration,
    pub results: Vec<TestResult>,
}

/// Test result
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
    pub duration: std::time::Duration,
    pub response_status: Option<u16>,
}