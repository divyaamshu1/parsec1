//! Cloud provider detection and management

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tracing::{info, warn, debug};

use crate::{CloudProvider, CloudProviderType, CloudService, DeploymentConfig, DeploymentResult, ServiceMetrics};

/// Cloud provider detector
pub struct CloudProviderDetector {
    custom_detectors: Vec<Box<dyn CustomCloudDetector>>,
}

#[async_trait]
pub trait CustomCloudDetector: Send + Sync {
    fn name(&self) -> &str;
    async fn detect(&self) -> Option<Box<dyn CloudProvider>>;
}

impl CloudProviderDetector {
    pub fn new() -> Self {
        Self {
            custom_detectors: Vec::new(),
        }
    }

    pub fn register_detector(&mut self, detector: Box<dyn CustomCloudDetector>) {
        self.custom_detectors.push(detector);
    }

    /// Detect all available cloud providers
    pub async fn detect_all(&self) -> Vec<Box<dyn CloudProvider>> {
        let mut providers = Vec::new();

        #[cfg(feature = "aws")]
        if let Ok(aws) = crate::aws::AWSProvider::detect().await {
            providers.push(Box::new(aws) as Box<dyn CloudProvider>);
        }

        #[cfg(feature = "azure")]
        if let Ok(azure) = crate::azure::AzureProvider::detect().await {
            providers.push(Box::new(azure));
        }

        #[cfg(feature = "gcp")]
        if let Ok(gcp) = crate::gcp::GCPProvider::detect().await {
            providers.push(Box::new(gcp));
        }

        // Detect custom providers
        for detector in &self.custom_detectors {
            if let Some(provider) = detector.detect().await {
                providers.push(provider);
            }
        }

        providers
    }

    /// Detect current cloud environment (where the IDE is running)
    pub async fn detect_current_environment(&self) -> Option<CloudEnvironment> {
        // Check if running on AWS
        if let Ok(token) = tokio::fs::read_to_string("/sys/hypervisor/uuid").await {
            if token.to_lowercase().contains("ec2") {
                return Some(CloudEnvironment::AWS);
            }
        }

        // Check if running on Azure
        if Path::new("/var/lib/waagent").exists() {
            return Some(CloudEnvironment::Azure);
        }

        // Check if running on GCP
        if let Ok(metadata) = reqwest::get("http://metadata.google.internal/computeMetadata/v1/")
            .header("Metadata-Flavor", "Google")
            .send()
            .await
        {
            if metadata.status().is_success() {
                return Some(CloudEnvironment::GCP);
            }
        }

        None
    }

    /// Get credentials from environment
    pub fn get_credentials_from_env(&self) -> CloudCredentials {
        let mut creds = CloudCredentials::default();

        if let Ok(profile) = std::env::var("AWS_PROFILE") {
            creds.aws_profile = Some(profile);
        }

        if let Ok(region) = std::env::var("AWS_REGION") {
            creds.aws_region = Some(region);
        }

        if let Ok(tenant) = std::env::var("AZURE_TENANT_ID") {
            creds.azure_tenant = Some(tenant);
        }

        if let Ok(sub) = std::env::var("AZURE_SUBSCRIPTION_ID") {
            creds.azure_subscription = Some(sub);
        }

        if let Ok(project) = std::env::var("GOOGLE_CLOUD_PROJECT") {
            creds.gcp_project = Some(project);
        }

        creds
    }
}

/// Cloud environment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudEnvironment {
    AWS,
    Azure,
    GCP,
    OnPremise,
    Unknown,
}

/// Cloud credentials
#[derive(Debug, Clone, Default)]
pub struct CloudCredentials {
    pub aws_profile: Option<String>,
    pub aws_region: Option<String>,
    pub aws_access_key: Option<String>,
    pub aws_secret_key: Option<String>,
    pub aws_session_token: Option<String>,

    pub azure_tenant: Option<String>,
    pub azure_subscription: Option<String>,
    pub azure_client_id: Option<String>,
    pub azure_client_secret: Option<String>,

    pub gcp_project: Option<String>,
    pub gcp_service_account: Option<PathBuf>,
    pub gcp_access_token: Option<String>,
}

impl CloudCredentials {
    /// Check if AWS credentials are available
    pub fn has_aws(&self) -> bool {
        self.aws_profile.is_some() || (self.aws_access_key.is_some() && self.aws_secret_key.is_some())
    }

    /// Check if Azure credentials are available
    pub fn has_azure(&self) -> bool {
        self.azure_tenant.is_some() && self.azure_subscription.is_some()
    }

    /// Check if GCP credentials are available
    pub fn has_gcp(&self) -> bool {
        self.gcp_project.is_some()
    }
}

/// Credential provider
#[async_trait]
pub trait CredentialProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn get_credentials(&self) -> Result<CloudCredentials>;
    async fn refresh(&self) -> Result<()>;
}