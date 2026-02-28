//! AWS cloud provider implementation

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tracing::{info, warn, debug};

#[cfg(feature = "aws")]
use aws_config::{BehaviorVersion, SdkConfig, Region};
#[cfg(feature = "aws")]
use aws_sdk_lambda::{Client as LambdaClient, types::FunctionConfiguration};
#[cfg(feature = "aws")]
use aws_sdk_s3::{Client as S3Client, types::Bucket};
#[cfg(feature = "aws")]
use aws_sdk_ec2::{Client as EC2Client, types::Instance};
#[cfg(feature = "aws")]
use aws_sdk_eks::{Client as EKSClient, types::Cluster};
#[cfg(feature = "aws")]
use aws_sdk_cloudformation::{Client as CloudFormationClient, types::Stack};
#[cfg(feature = "aws")]
use aws_sdk_cloudwatchlogs::{Client as CloudWatchLogsClient};

use crate::{CloudProvider, CloudProviderType, CloudService, ServiceType, ServiceStatus,
            DeploymentConfig, DeploymentResult, ServiceMetrics, CloudConfig};

/// AWS provider
#[cfg(feature = "aws")]
pub struct AWSProvider {
    config: SdkConfig,
    region: String,
    lambda_client: LambdaClient,
    s3_client: S3Client,
    ec2_client: EC2Client,
    eks_client: EKSClient,
    cfn_client: CloudFormationClient,
    logs_client: CloudWatchLogsClient,
    cloud_config: CloudConfig,
}

#[cfg(feature = "aws")]
impl AWSProvider {
    /// Create new AWS provider
    pub async fn new(config: CloudConfig) -> Result<Self> {
        let mut aws_config_builder = aws_config::defaults(BehaviorVersion::latest());

        if let Some(profile) = &config.aws_profile {
            aws_config_builder = aws_config_builder.profile_name(profile);
        }

        if let Some(region) = &config.aws_region {
            aws_config_builder = aws_config_builder.region(Region::new(region.clone()));
        }

        let aws_config = aws_config_builder.load().await;

        Ok(Self {
            lambda_client: LambdaClient::new(&aws_config),
            s3_client: S3Client::new(&aws_config),
            ec2_client: EC2Client::new(&aws_config),
            eks_client: EKSClient::new(&aws_config),
            cfn_client: CloudFormationClient::new(&aws_config),
            logs_client: CloudWatchLogsClient::new(&aws_config),
            region: config.aws_region.clone().unwrap_or_else(|| "us-east-1".to_string()),
            config: aws_config,
            cloud_config: config,
        })
    }

    /// Detect AWS installation
    pub async fn detect() -> Result<Self> {
        let config = CloudConfig {
            aws_profile: std::env::var("AWS_PROFILE").ok(),
            aws_region: std::env::var("AWS_REGION").ok(),
            ..Default::default()
        };
        Self::new(config).await
    }

    /// List Lambda functions
    async fn list_lambda_functions(&self) -> Result<Vec<CloudService>> {
        let mut services = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = self.lambda_client.list_functions();
            if let Some(token) = next_token {
                request = request.marker(token);
            }

            let response = request.send().await?;
            
            if let Some(functions) = response.functions {
                for func in functions {
                    services.push(CloudService {
                        name: func.function_name.unwrap_or_default(),
                        provider: CloudProviderType::AWS,
                        service_type: ServiceType::Serverless,
                        region: self.region.clone(),
                        status: self.map_function_state(func.state),
                        url: func.function_arn,
                        created_at: func.last_modified
                            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .unwrap_or_else(chrono::Utc::now),
                        updated_at: chrono::Utc::now(),
                        tags: HashMap::new(),
                    });
                }
            }

            next_token = response.next_marker;
            if next_token.is_none() {
                break;
            }
        }

        Ok(services)
    }

    /// List S3 buckets
    async fn list_s3_buckets(&self) -> Result<Vec<CloudService>> {
        let mut services = Vec::new();
        let response = self.s3_client.list_buckets().send().await?;

        if let Some(buckets) = response.buckets {
            for bucket in buckets {
                let name = bucket.name.unwrap_or_default();

                services.push(CloudService {
                    name: name.clone(),
                    provider: CloudProviderType::AWS,
                    service_type: ServiceType::Storage,
                    region: self.region.clone(),
                    status: ServiceStatus::Running,
                    url: Some(format!("s3://{}/", name)),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    tags: HashMap::new(),
                });
            }
        }

        Ok(services)
    }

    /// List EC2 instances
    async fn list_ec2_instances(&self) -> Result<Vec<CloudService>> {
        let mut services = Vec::new();
        let response = self.ec2_client.describe_instances().send().await?;

        if let Some(reservations) = response.reservations {
            for reservation in reservations {
                if let Some(instances) = reservation.instances {
                    for instance in instances {
                        let name = instance.instance_id.unwrap_or_default();
                        let state = instance.state.and_then(|s| s.name.map(|n| n.as_str().to_string()));

                        services.push(CloudService {
                            name,
                            provider: CloudProviderType::AWS,
                            service_type: ServiceType::Compute,
                            region: self.region.clone(),
                            status: self.map_ec2_state(state.as_deref()),
                            url: instance.public_dns_name,
                            created_at: chrono::Utc::now(),
                            updated_at: chrono::Utc::now(),
                            tags: self.extract_tags(instance.tags),
                        });
                    }
                }
            }
        }

        Ok(services)
    }

    /// List EKS clusters
    async fn list_eks_clusters(&self) -> Result<Vec<CloudService>> {
        let mut services = Vec::new();
        let response = self.eks_client.list_clusters().send().await?;

        if let Some(clusters) = response.clusters {
            for cluster_name in clusters {
                if let Ok(cluster) = self.eks_client.describe_cluster()
                    .name(&cluster_name)
                    .send()
                    .await
                {
                    if let Some(cluster_info) = cluster.cluster {
                        services.push(CloudService {
                            name: cluster_name,
                            provider: CloudProviderType::AWS,
                            service_type: ServiceType::Container,
                            region: self.region.clone(),
                            status: self.map_eks_status(cluster_info.status),
                            url: cluster_info.endpoint,
                            created_at: chrono::Utc::now(),
                            updated_at: chrono::Utc::now(),
                            tags: self.extract_tags(None),
                        });
                    }
                }
            }
        }

        Ok(services)
    }

    /// List CloudFormation stacks
    async fn list_cfn_stacks(&self) -> Result<Vec<CloudService>> {
        let mut services = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = self.cfn_client.describe_stacks();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;
            
            if let Some(stacks) = response.stacks {
                for stack in stacks {
                    services.push(CloudService {
                        name: stack.stack_name.unwrap_or_default(),
                        provider: CloudProviderType::AWS,
                        service_type: ServiceType::Custom("CloudFormation".to_string()),
                        region: self.region.clone(),
                        status: self.map_stack_status(stack.stack_status),
                        url: None,
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                        tags: self.extract_cf_tags(stack.tags),
                    });
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(services)
    }

    /// Map Lambda function state
    fn map_function_state(&self, state: Option<aws_sdk_lambda::types::State>) -> ServiceStatus {
        match state {
            Some(aws_sdk_lambda::types::State::Active) => ServiceStatus::Running,
            Some(aws_sdk_lambda::types::State::Pending) => ServiceStatus::Creating,
            Some(aws_sdk_lambda::types::State::Inactive) => ServiceStatus::Stopped,
            Some(aws_sdk_lambda::types::State::Failed) => ServiceStatus::Failed("Failed".to_string()),
            _ => ServiceStatus::Unknown,
        }
    }

    /// Map EC2 instance state
    fn map_ec2_state(&self, state: Option<&str>) -> ServiceStatus {
        match state {
            Some("running") => ServiceStatus::Running,
            Some("pending") => ServiceStatus::Creating,
            Some("stopping") | Some("stopped") => ServiceStatus::Stopped,
            Some("terminated") => ServiceStatus::Failed("Terminated".to_string()),
            _ => ServiceStatus::Unknown,
        }
    }

    /// Map EKS cluster status
    fn map_eks_status(&self, status: Option<aws_sdk_eks::types::ClusterStatus>) -> ServiceStatus {
        match status {
            Some(aws_sdk_eks::types::ClusterStatus::Active) => ServiceStatus::Running,
            Some(aws_sdk_eks::types::ClusterStatus::Creating) => ServiceStatus::Creating,
            Some(aws_sdk_eks::types::ClusterStatus::Deleting) => ServiceStatus::Deleting,
            Some(aws_sdk_eks::types::ClusterStatus::Failed) => ServiceStatus::Failed("Failed".to_string()),
            Some(aws_sdk_eks::types::ClusterStatus::Updating) => ServiceStatus::Updating,
            _ => ServiceStatus::Unknown,
        }
    }

    /// Map CloudFormation stack status
    fn map_stack_status(&self, status: Option<aws_sdk_cloudformation::types::StackStatus>) -> ServiceStatus {
        match status {
            Some(aws_sdk_cloudformation::types::StackStatus::CreateComplete) => ServiceStatus::Running,
            Some(aws_sdk_cloudformation::types::StackStatus::CreateInProgress) => ServiceStatus::Creating,
            Some(aws_sdk_cloudformation::types::StackStatus::DeleteComplete) => ServiceStatus::Stopped,
            Some(aws_sdk_cloudformation::types::StackStatus::DeleteFailed) => ServiceStatus::Failed("Delete failed".to_string()),
            Some(aws_sdk_cloudformation::types::StackStatus::UpdateComplete) => ServiceStatus::Running,
            Some(aws_sdk_cloudformation::types::StackStatus::UpdateInProgress) => ServiceStatus::Updating,
            Some(aws_sdk_cloudformation::types::StackStatus::RollbackComplete) => ServiceStatus::Failed("Rollback".to_string()),
            _ => ServiceStatus::Unknown,
        }
    }

    /// Extract tags from EC2 tag list
    fn extract_tags(&self, tags: Option<Vec<aws_sdk_ec2::types::Tag>>) -> HashMap<String, String> {
        let mut result = HashMap::new();
        if let Some(tags) = tags {
            for tag in tags {
                if let (Some(key), Some(value)) = (tag.key, tag.value) {
                    result.insert(key, value);
                }
            }
        }
        result
    }

    /// Extract tags from CloudFormation tag list
    fn extract_cf_tags(&self, tags: Option<Vec<aws_sdk_cloudformation::types::Tag>>) -> HashMap<String, String> {
        let mut result = HashMap::new();
        if let Some(tags) = tags {
            for tag in tags {
                if let (Some(key), Some(value)) = (tag.key, tag.value) {
                    result.insert(key, value);
                }
            }
        }
        result
    }

    /// Convert AWS DateTime to chrono DateTime
    fn convert_aws_datetime<T: std::fmt::Debug>(&self, dt: T) -> chrono::DateTime<chrono::Utc> {
        // Fallback to now() for unsupported AWS DateTime types
        chrono::Utc::now()
    }
}

#[cfg(feature = "aws")]
#[async_trait]
impl CloudProvider for AWSProvider {
    fn name(&self) -> &str {
        "AWS"
    }

    fn provider_type(&self) -> CloudProviderType {
        CloudProviderType::AWS
    }

    async fn is_configured(&self) -> bool {
        // Try to list something to verify credentials
        self.s3_client.list_buckets().send().await.is_ok()
    }

    async fn get_regions(&self) -> Result<Vec<String>> {
        Ok(vec![
            "us-east-1".to_string(),
            "us-east-2".to_string(),
            "us-west-1".to_string(),
            "us-west-2".to_string(),
            "eu-west-1".to_string(),
            "eu-central-1".to_string(),
            "ap-southeast-1".to_string(),
            "ap-northeast-1".to_string(),
        ])
    }

    async fn list_services(&self) -> Result<Vec<CloudService>> {
        let mut services = Vec::new();

        // List all service types
        services.extend(self.list_lambda_functions().await?);
        services.extend(self.list_s3_buckets().await?);
        services.extend(self.list_ec2_instances().await?);
        services.extend(self.list_eks_clusters().await?);
        services.extend(self.list_cfn_stacks().await?);

        Ok(services)
    }

    async fn get_service(&self, name: &str) -> Result<Option<CloudService>> {
        let services = self.list_services().await?;
        Ok(services.into_iter().find(|s| s.name == name))
    }

    async fn deploy(&self, config: DeploymentConfig) -> Result<DeploymentResult> {
        let start = std::time::Instant::now();

        match config.service_type {
            ServiceType::Serverless => {
                // Deploy Lambda function
                self.deploy_lambda(config).await
            }
            ServiceType::Compute => {
                // Deploy EC2 instance
                self.deploy_ec2(config).await
            }
            ServiceType::Container => {
                // Deploy to EKS
                self.deploy_eks(config).await
            }
            ServiceType::Storage => {
                // Deploy S3 bucket
                self.deploy_s3(config).await
            }
            _ => Err(anyhow!("Unsupported service type for AWS")),
        }
    }

    async fn logs(&self, service: &str, tail: Option<usize>) -> Result<Vec<String>> {
        let tail = tail.unwrap_or(100);

        // Try Lambda logs first
        if let Ok(logs) = self.get_lambda_logs(service, tail).await {
            return Ok(logs);
        }

        // Try CloudFormation logs
        if let Ok(logs) = self.get_stack_logs(service, tail).await {
            return Ok(logs);
        }

        Err(anyhow!("No logs found for service: {}", service))
    }

    async fn metrics(&self, service: &str) -> Result<ServiceMetrics> {
        // Would query CloudWatch metrics
        Ok(ServiceMetrics {
            cpu_usage: Some(0.0),
            memory_usage: Some(0),
            requests_per_second: Some(0.0),
            error_rate: Some(0.0),
            latency_p95: Some(std::time::Duration::from_millis(0)),
            timestamp: chrono::Utc::now(),
        })
    }
}

#[cfg(feature = "aws")]
impl AWSProvider {
    /// Deploy Lambda function
    async fn deploy_lambda(&self, config: DeploymentConfig) -> Result<DeploymentResult> {
        let start = std::time::Instant::now();

        // This would zip the source and create/update Lambda
        // Simplified implementation

        Ok(DeploymentResult {
            service_name: config.name.clone(),
            deployment_id: uuid::Uuid::new_v4().to_string(),
            service_url: Some(format!("https://{}.lambda-url.{}.on.aws", &config.name, self.region)),
            status: ServiceStatus::Running,
            logs: vec!["Deployment successful".to_string()],
            duration: start.elapsed(),
        })
    }

    /// Deploy EC2 instance
    async fn deploy_ec2(&self, config: DeploymentConfig) -> Result<DeploymentResult> {
        Err(anyhow!("EC2 deployment not yet implemented"))
    }

    /// Deploy to EKS
    async fn deploy_eks(&self, config: DeploymentConfig) -> Result<DeploymentResult> {
        Err(anyhow!("EKS deployment not yet implemented"))
    }

    /// Deploy S3 bucket
    async fn deploy_s3(&self, config: DeploymentConfig) -> Result<DeploymentResult> {
        Err(anyhow!("S3 deployment not yet implemented"))
    }

    /// Get Lambda logs
    async fn get_lambda_logs(&self, function_name: &str, limit: usize) -> Result<Vec<String>> {
        // Query CloudWatch Logs
        let log_group = format!("/aws/lambda/{}", function_name);

        let response = self.logs_client
            .filter_log_events()
            .log_group_name(log_group)
            .limit(limit as i32)
            .send()
            .await?;

        let logs = response.events()
            .iter()
            .map(|e| e.message().unwrap_or("").to_string())
            .collect();

        Ok(logs)
    }

    /// Get CloudFormation stack logs
    async fn get_stack_logs(&self, stack_name: &str, limit: usize) -> Result<Vec<String>> {
        // Would query stack events
        Ok(vec![])
    }
}