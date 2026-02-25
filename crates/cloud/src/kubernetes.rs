//! Kubernetes cluster management

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use tracing::{info, warn, debug};

#[cfg(feature = "kubernetes")]
use k8s_openapi::api::{
    core::v1::{Pod, Service, Namespace, ConfigMap, Secret},
    apps::v1::{Deployment, StatefulSet, DaemonSet},
    batch::v1::{Job, CronJob},
    networking::v1::{Ingress, NetworkPolicy},
};
#[cfg(feature = "kubernetes")]
use kube::{
    Client, Config, Api,
    api::{ListParams, PostParams, DeleteParams},
    runtime::watcher,
};

use crate::CloudConfig;

/// Kubernetes client
pub struct KubernetesClient {
    #[cfg(feature = "kubernetes")]
    client: Client,
    #[cfg(feature = "kubernetes")]
    config: Config,
    cloud_config: CloudConfig,
}

/// Kubernetes resource
#[derive(Debug, Clone)]
pub struct KubeResource {
    pub kind: String,
    pub name: String,
    pub namespace: String,
    pub api_version: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
}

/// Kubernetes pod
#[derive(Debug, Clone)]
pub struct KubePod {
    pub name: String,
    pub namespace: String,
    pub node: Option<String>,
    pub status: String,
    pub phase: String,
    pub containers: Vec<KubeContainer>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Kubernetes container
#[derive(Debug, Clone)]
pub struct KubeContainer {
    pub name: String,
    pub image: String,
    pub ready: bool,
    pub restart_count: i32,
    pub state: String,
}

impl KubernetesClient {
    /// Create new Kubernetes client
    pub fn new(config: CloudConfig) -> Result<Self> {
        #[cfg(feature = "kubernetes")]
        {
            let runtime = tokio::runtime::Handle::current();
            let _guard = runtime.enter();

            let kube_config = if let Some(kubeconfig) = &config.kubeconfig_path {
                Config::from_file(kubeconfig)?
            } else {
                Config::from_env()?
            };

            let client = Client::try_from(kube_config.clone())?;

            Ok(Self {
                client,
                config: kube_config,
                cloud_config: config,
            })
        }

        #[cfg(not(feature = "kubernetes"))]
        {
            Ok(Self { cloud_config: config })
        }
    }

    /// List pods
    pub async fn list_pods(&self, namespace: Option<&str>) -> Result<Vec<KubePod>> {
        #[cfg(feature = "kubernetes")]
        {
            let namespace = namespace.unwrap_or("default");
            let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
            
            let lp = ListParams::default();
            let pod_list = pods.list(&lp).await?;

            let mut result = Vec::new();
            for pod in pod_list {
                let containers = pod.spec
                    .and_then(|s| s.containers)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|c| KubeContainer {
                        name: c.name,
                        image: c.image.unwrap_or_default(),
                        ready: false, // Would get from status
                        restart_count: 0,
                        state: "unknown".to_string(),
                    })
                    .collect();

                result.push(KubePod {
                    name: pod.metadata.name.unwrap_or_default(),
                    namespace: namespace.to_string(),
                    node: pod.spec.and_then(|s| s.node_name),
                    status: pod.status
                        .and_then(|s| s.phase)
                        .unwrap_or_default(),
                    phase: pod.status
                        .and_then(|s| s.phase)
                        .unwrap_or_default(),
                    containers,
                    created_at: pod.metadata.creation_timestamp
                        .map(|t| chrono::DateTime::from(t.0))
                        .unwrap_or_else(chrono::Utc::now),
                });
            }

            Ok(result)
        }

        #[cfg(not(feature = "kubernetes"))]
        {
            Err(anyhow!("Kubernetes support not enabled"))
        }
    }

    /// List deployments
    pub async fn list_deployments(&self, namespace: Option<&str>) -> Result<Vec<KubeResource>> {
        #[cfg(feature = "kubernetes")]
        {
            let namespace = namespace.unwrap_or("default");
            let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);
            
            let lp = ListParams::default();
            let deploy_list = deployments.list(&lp).await?;

            let mut result = Vec::new();
            for deploy in deploy_list {
                let status = if let Some(status) = deploy.status {
                    if status.available_replicas == status.replicas {
                        "Running".to_string()
                    } else {
                        "Pending".to_string()
                    }
                } else {
                    "Unknown".to_string()
                };

                result.push(KubeResource {
                    kind: "Deployment".to_string(),
                    name: deploy.metadata.name.unwrap_or_default(),
                    namespace: namespace.to_string(),
                    api_version: deploy.api_version.unwrap_or_default(),
                    status,
                    created_at: deploy.metadata.creation_timestamp
                        .map(|t| chrono::DateTime::from(t.0))
                        .unwrap_or_else(chrono::Utc::now),
                    labels: deploy.metadata.labels.unwrap_or_default(),
                    annotations: deploy.metadata.annotations.unwrap_or_default(),
                });
            }

            Ok(result)
        }

        #[cfg(not(feature = "kubernetes"))]
        {
            Err(anyhow!("Kubernetes support not enabled"))
        }
    }

    /// List services
    pub async fn list_services(&self, namespace: Option<&str>) -> Result<Vec<KubeResource>> {
        #[cfg(feature = "kubernetes")]
        {
            let namespace = namespace.unwrap_or("default");
            let services: Api<Service> = Api::namespaced(self.client.clone(), namespace);
            
            let lp = ListParams::default();
            let svc_list = services.list(&lp).await?;

            let mut result = Vec::new();
            for svc in svc_list {
                result.push(KubeResource {
                    kind: "Service".to_string(),
                    name: svc.metadata.name.unwrap_or_default(),
                    namespace: namespace.to_string(),
                    api_version: svc.api_version.unwrap_or_default(),
                    status: "Active".to_string(),
                    created_at: svc.metadata.creation_timestamp
                        .map(|t| chrono::DateTime::from(t.0))
                        .unwrap_or_else(chrono::Utc::now),
                    labels: svc.metadata.labels.unwrap_or_default(),
                    annotations: svc.metadata.annotations.unwrap_or_default(),
                });
            }

            Ok(result)
        }

        #[cfg(not(feature = "kubernetes"))]
        {
            Err(anyhow!("Kubernetes support not enabled"))
        }
    }

    /// List namespaces
    pub async fn list_namespaces(&self) -> Result<Vec<String>> {
        #[cfg(feature = "kubernetes")]
        {
            let namespaces: Api<Namespace> = Api::all(self.client.clone());
            let lp = ListParams::default();
            let ns_list = namespaces.list(&lp).await?;

            Ok(ns_list.into_iter()
                .filter_map(|ns| ns.metadata.name)
                .collect())
        }

        #[cfg(not(feature = "kubernetes"))]
        {
            Err(anyhow!("Kubernetes support not enabled"))
        }
    }

    /// Create deployment
    pub async fn create_deployment(
        &self,
        name: &str,
        image: &str,
        replicas: i32,
        namespace: Option<&str>,
    ) -> Result<()> {
        #[cfg(feature = "kubernetes")]
        {
            use k8s_openapi::api::apps::v1::DeploymentSpec;
            use k8s_openapi::api::core::v1::{Container, PodSpec, PodTemplateSpec};
            use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};

            let namespace = namespace.unwrap_or("default");
            let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);

            let deployment = Deployment {
                metadata: ObjectMeta {
                    name: Some(name.to_string()),
                    namespace: Some(namespace.to_string()),
                    labels: Some({
                        let mut labels = HashMap::new();
                        labels.insert("app".to_string(), name.to_string());
                        labels
                    }),
                    ..Default::default()
                },
                spec: Some(DeploymentSpec {
                    replicas: Some(replicas),
                    selector: LabelSelector {
                        match_labels: Some({
                            let mut labels = HashMap::new();
                            labels.insert("app".to_string(), name.to_string());
                            labels
                        }),
                        ..Default::default()
                    },
                    template: PodTemplateSpec {
                        metadata: Some(ObjectMeta {
                            labels: Some({
                                let mut labels = HashMap::new();
                                labels.insert("app".to_string(), name.to_string());
                                labels
                            }),
                            ..Default::default()
                        }),
                        spec: Some(PodSpec {
                            containers: vec![Container {
                                name: name.to_string(),
                                image: Some(image.to_string()),
                                ..Default::default()
                            }],
                            ..Default::default()
                        }),
                    },
                    ..Default::default()
                }),
                ..Default::default()
            };

            let pp = PostParams::default();
            deployments.create(&pp, &deployment).await?;

            Ok(())
        }

        #[cfg(not(feature = "kubernetes"))]
        {
            Err(anyhow!("Kubernetes support not enabled"))
        }
    }

    /// Delete deployment
    pub async fn delete_deployment(&self, name: &str, namespace: Option<&str>) -> Result<()> {
        #[cfg(feature = "kubernetes")]
        {
            let namespace = namespace.unwrap_or("default");
            let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);
            
            let dp = DeleteParams::default();
            deployments.delete(name, &dp).await?;

            Ok(())
        }

        #[cfg(not(feature = "kubernetes"))]
        {
            Err(anyhow!("Kubernetes support not enabled"))
        }
    }

    /// Get pod logs
    pub async fn pod_logs(&self, name: &str, namespace: Option<&str>, tail: usize) -> Result<Vec<String>> {
        #[cfg(feature = "kubernetes")]
        {
            let namespace = namespace.unwrap_or("default");
            let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
            
            let logs = pods.logs(name, &Default::default()).await?;
            let lines: Vec<String> = logs.lines().take(tail).map(|l| l.to_string()).collect();

            Ok(lines)
        }

        #[cfg(not(feature = "kubernetes"))]
        {
            Err(anyhow!("Kubernetes support not enabled"))
        }
    }

    /// Check if Kubernetes is available
    pub async fn is_available(&self) -> bool {
        #[cfg(feature = "kubernetes")]
        {
            self.list_namespaces().await.is_ok()
        }

        #[cfg(not(feature = "kubernetes"))]
        {
            false
        }
    }

    /// Get current context
    pub async fn current_context(&self) -> Option<String> {
        #[cfg(feature = "kubernetes")]
        {
            Some(self.config.current_context.clone())
        }

        #[cfg(not(feature = "kubernetes"))]
        {
            None
        }
    }
}