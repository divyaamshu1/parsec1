//! Serverless functions management (AWS Lambda, Azure Functions, GCP Functions)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use tracing::{info, warn, debug};

use crate::{CloudConfig, CloudService, ServiceType, ServiceStatus};

/// Serverless function
#[derive(Debug, Clone)]
pub struct ServerlessFunction {
    pub name: String,
    pub provider: String,
    pub region: String,
    pub runtime: String,
    pub handler: String,
    pub memory_mb: u32,
    pub timeout_secs: u32,
    pub environment: HashMap<String, String>,
    pub last_modified: chrono::DateTime<chrono::Utc>,
    pub status: String,
}

/// Serverless deployment
#[derive(Debug, Clone)]
pub struct ServerlessDeployment {
    pub id: String,
    pub function: String,
    pub version: String,
    pub status: String,
    pub logs: Vec<String>,
    pub deployed_at: chrono::DateTime<chrono::Utc>,
}

/// Serverless manager
pub struct ServerlessManager {
    config: CloudConfig,
}

impl ServerlessManager {
    /// Create new serverless manager
    pub fn new(config: CloudConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// List functions from AWS
    #[cfg(feature = "aws")]
    async fn list_aws_functions(&self) -> Result<Vec<ServerlessFunction>> {
        use aws_config::BehaviorVersion;
        use aws_sdk_lambda::Client;

        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(self.config.aws_region.clone())
            .load()
            .await;

        let client = Client::new(&aws_config);
        let mut functions = Vec::new();
        let mut next_marker = None;

        loop {
            let mut req = client.list_functions();
            if let Some(marker) = next_marker {
                req = req.marker(marker);
            }

            let resp = req.send().await?;

            if let Some(funcs) = resp.functions {
                for f in funcs {
                    functions.push(ServerlessFunction {
                        name: f.function_name.unwrap_or_default(),
                        provider: "AWS".to_string(),
                        region: self.config.aws_region.clone().unwrap_or_default(),
                        runtime: f.runtime.map(|r| r.as_str().to_string()).unwrap_or_default(),
                        handler: f.handler.unwrap_or_default(),
                        memory_mb: f.memory_size.unwrap_or(128) as u32,
                        timeout_secs: f.timeout.unwrap_or(3) as u32,
                        environment: f.environment
                            .and_then(|e| e.variables)
                            .unwrap_or_default(),
                        last_modified: f.last_modified
                            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .unwrap_or_else(chrono::Utc::now),
                        status: "Active".to_string(),
                    });
                }
            }

            next_marker = resp.next_marker;
            if next_marker.is_none() {
                break;
            }
        }

        Ok(functions)
    }

    /// List functions from Azure
    #[cfg(feature = "azure")]
    async fn list_azure_functions(&self) -> Result<Vec<ServerlessFunction>> {
        // Would call Azure Functions API
        Ok(vec![])
    }

    /// List functions from GCP
    #[cfg(feature = "gcp")]
    async fn list_gcp_functions(&self) -> Result<Vec<ServerlessFunction>> {
        // Would call GCP Cloud Functions API
        Ok(vec![])
    }

    /// List all serverless functions
    pub async fn list_functions(&self) -> Result<Vec<ServerlessFunction>> {
        let mut all_functions = Vec::new();

        #[cfg(feature = "aws")]
        if let Ok(aws_funcs) = self.list_aws_functions().await {
            all_functions.extend(aws_funcs);
        }

        #[cfg(feature = "azure")]
        if let Ok(azure_funcs) = self.list_azure_functions().await {
            all_functions.extend(azure_funcs);
        }

        #[cfg(feature = "gcp")]
        if let Ok(gcp_funcs) = self.list_gcp_functions().await {
            all_functions.extend(gcp_funcs);
        }

        Ok(all_functions)
    }

    /// Deploy function to AWS
    #[cfg(feature = "aws")]
    pub async fn deploy_to_aws(
        &self,
        name: &str,
        runtime: &str,
        handler: &str,
        zip_path: &Path,
    ) -> Result<ServerlessDeployment> {
        use aws_config::BehaviorVersion;
        use aws_sdk_lambda::{Client, types::{FunctionCode, Runtime}};

        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(self.config.aws_region.clone())
            .load()
            .await;

        let client = Client::new(&aws_config);

        // Read zip file
        let zip_bytes = tokio::fs::read(zip_path).await?;

        // Create function
        let create_resp = client.create_function()
            .function_name(name)
            .runtime(Runtime::from(runtime.as_str()))
            .role(format!("arn:aws:iam::{}:role/lambda-execution-role", "YOUR_ACCOUNT_ID"))
            .handler(handler)
            .code(FunctionCode::builder().zip_file(zip_bytes.into()).build())
            .send()
            .await?;

        Ok(ServerlessDeployment {
            id: create_resp.function_arn.unwrap_or_default(),
            function: name.to_string(),
            version: create_resp.version.unwrap_or_default(),
            status: "Created".to_string(),
            logs: vec![],
            deployed_at: chrono::Utc::now(),
        })
    }

    /// Invoke function
    #[cfg(feature = "aws")]
    pub async fn invoke_aws_function(&self, name: &str, payload: &[u8]) -> Result<Vec<u8>> {
        use aws_config::BehaviorVersion;
        use aws_sdk_lambda::{Client, types::Blob};

        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(self.config.aws_region.clone())
            .load()
            .await;

        let client = Client::new(&aws_config);

        let resp = client.invoke()
            .function_name(name)
            .payload(Blob::new(payload.to_vec()))
            .send()
            .await?;

        Ok(resp.payload.unwrap().into_inner())
    }

    /// Get function logs
    #[cfg(feature = "aws")]
    pub async fn get_aws_function_logs(&self, name: &str, tail: usize) -> Result<Vec<String>> {
        use aws_config::BehaviorVersion;
        use aws_sdk_cloudwatchlogs::Client;

        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(self.config.aws_region.clone())
            .load()
            .await;

        let client = Client::new(&aws_config);

        let log_group = format!("/aws/lambda/{}", name);

        let resp = client.filter_log_events()
            .log_group_name(log_group)
            .limit(tail as i32)
            .send()
            .await?;

        let logs = resp.events()
            .iter()
            .map(|e| e.message().unwrap_or("").to_string())
            .collect();

        Ok(logs)
    }

    /// Convert to CloudService
    pub fn to_cloud_service(&self, func: &ServerlessFunction) -> CloudService {
        CloudService {
            name: func.name.clone(),
            provider: crate::CloudProviderType::AWS, // Would need to map
            service_type: ServiceType::Serverless,
            region: func.region.clone(),
            status: ServiceStatus::Running,
            url: None,
            created_at: func.last_modified,
            updated_at: func.last_modified,
            tags: HashMap::new(),
        }
    }
}