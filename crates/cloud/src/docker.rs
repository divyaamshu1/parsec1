//! Docker container management

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use tracing::{info, warn, debug};

#[cfg(feature = "docker")]
use bollard::{Docker, container::*, image::*, exec::*};
#[cfg(feature = "docker")]
use futures::{StreamExt, TryStreamExt};

use crate::CloudConfig;

/// Docker client
pub struct DockerClient {
    #[cfg(feature = "docker")]
    docker: Docker,
    config: CloudConfig,
}

/// Docker container
#[derive(Debug, Clone)]
pub struct DockerContainer {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub state: String,
    pub ports: Vec<DockerPort>,
    pub created: chrono::DateTime<chrono::Utc>,
}

/// Docker port mapping
#[derive(Debug, Clone)]
pub struct DockerPort {
    pub ip: String,
    pub private_port: u16,
    pub public_port: Option<u16>,
    pub protocol: String,
}

/// Docker image
#[derive(Debug, Clone)]
pub struct DockerImage {
    pub id: String,
    pub tags: Vec<String>,
    pub size: u64,
    pub created: chrono::DateTime<chrono::Utc>,
}

impl DockerClient {
    /// Create new Docker client
    pub fn new(config: CloudConfig) -> Result<Self> {
        #[cfg(feature = "docker")]
        {
            let docker = if let Some(host) = &config.docker_host {
                Docker::connect_with_local(host, 120, bollard::API_DEFAULT_VERSION)?
            } else {
                Docker::connect_with_local_defaults()?
            };

            Ok(Self { docker, config })
        }

        #[cfg(not(feature = "docker"))]
        {
            Ok(Self { config })
        }
    }

    /// List containers
    pub async fn list_containers(&self, all: bool) -> Result<Vec<DockerContainer>> {
        #[cfg(feature = "docker")]
        {
            let mut filters = HashMap::new();
            if !all {
                filters.insert("status", vec!["running"]);
            }

            let containers = self.docker.list_containers(Some(ListContainersOptions {
                all,
                filters,
                ..Default::default()
            })).await?;

            let mut result = Vec::new();
            for c in containers {
                let ports = c.ports.unwrap_or_default()
                    .into_iter()
                    .map(|p| DockerPort {
                        ip: p.ip.unwrap_or_default(),
                        private_port: p.private_port as u16,
                        public_port: p.public_port.map(|p| p as u16),
                        protocol: p.typ.unwrap_or_default(),
                    })
                    .collect();

                result.push(DockerContainer {
                    id: c.id.unwrap_or_default(),
                    name: c.names.unwrap_or_default().first()
                        .cloned()
                        .unwrap_or_default()
                        .trim_start_matches('/')
                        .to_string(),
                    image: c.image.unwrap_or_default(),
                    status: c.status.unwrap_or_default(),
                    state: c.state.unwrap_or_default(),
                    ports,
                    created: chrono::DateTime::from_timestamp(c.created.unwrap_or(0), 0)
                        .unwrap_or_else(chrono::Utc::now),
                });
            }

            Ok(result)
        }

        #[cfg(not(feature = "docker"))]
        {
            Err(anyhow!("Docker support not enabled"))
        }
    }

    /// List images
    pub async fn list_images(&self) -> Result<Vec<DockerImage>> {
        #[cfg(feature = "docker")]
        {
            let images = self.docker.list_images(Some(ListImagesOptions::<String> {
                all: true,
                ..Default::default()
            })).await?;

            let mut result = Vec::new();
            for i in images {
                result.push(DockerImage {
                    id: i.id,
                    tags: i.repo_tags.unwrap_or_default(),
                    size: i.size as u64,
                    created: chrono::DateTime::from_timestamp(i.created, 0)
                        .unwrap_or_else(chrono::Utc::now),
                });
            }

            Ok(result)
        }

        #[cfg(not(feature = "docker"))]
        {
            Err(anyhow!("Docker support not enabled"))
        }
    }

    /// Start container
    pub async fn start_container(&self, container_id: &str) -> Result<()> {
        #[cfg(feature = "docker")]
        {
            self.docker.start_container(container_id, None).await?;
            Ok(())
        }

        #[cfg(not(feature = "docker"))]
        {
            Err(anyhow!("Docker support not enabled"))
        }
    }

    /// Stop container
    pub async fn stop_container(&self, container_id: &str) -> Result<()> {
        #[cfg(feature = "docker")]
        {
            self.docker.stop_container(container_id, None).await?;
            Ok(())
        }

        #[cfg(not(feature = "docker"))]
        {
            Err(anyhow!("Docker support not enabled"))
        }
    }

    /// Remove container
    pub async fn remove_container(&self, container_id: &str, force: bool) -> Result<()> {
        #[cfg(feature = "docker")]
        {
            self.docker.remove_container(container_id, Some(RemoveContainerOptions {
                force,
                ..Default::default()
            })).await?;
            Ok(())
        }

        #[cfg(not(feature = "docker"))]
        {
            Err(anyhow!("Docker support not enabled"))
        }
    }

    /// Pull image
    pub async fn pull_image(&self, image: &str) -> Result<()> {
        #[cfg(feature = "docker")]
        {
            use bollard::image::CreateImageOptions;

            let options = CreateImageOptions {
                from_image: image,
                ..Default::default()
            };

            let mut stream = self.docker.create_image(Some(options), None, None);
            while let Some(result) = stream.next().await {
                match result {
                    Ok(_) => continue,
                    Err(e) => return Err(anyhow!("Failed to pull image: {}", e)),
                }
            }

            Ok(())
        }

        #[cfg(not(feature = "docker"))]
        {
            Err(anyhow!("Docker support not enabled"))
        }
    }

    /// Build image
    pub async fn build_image(&self, context: &Path, tag: &str, dockerfile: &str) -> Result<()> {
        #[cfg(feature = "docker")]
        {
            use bollard::image::BuildImageOptions;
            use std::collections::HashMap;
            use tar::{Builder, Header};

            // Create tar archive of context
            let mut tar = Vec::new();
            {
                let mut builder = Builder::new(&mut tar);
                Self::add_dir_to_tar(&mut builder, context, "")?;
            }

            let options = BuildImageOptions {
                dockerfile,
                t: tag,
                pull: true,
                rm: true,
                ..Default::default()
            };

            let mut stream = self.docker.build_image(options, tar.into(), None);
            while let Some(result) = stream.next().await {
                match result {
                    Ok(_) => continue,
                    Err(e) => return Err(anyhow!("Build failed: {}", e)),
                }
            }

            Ok(())
        }

        #[cfg(not(feature = "docker"))]
        {
            Err(anyhow!("Docker support not enabled"))
        }
    }

    /// Get container logs
    pub async fn container_logs(&self, container_id: &str, tail: usize) -> Result<Vec<String>> {
        #[cfg(feature = "docker")]
        {
            let options = Some(LogsOptions::<String> {
                follow: false,
                stdout: true,
                stderr: true,
                tail: tail.to_string(),
                ..Default::default()
            });

            let mut logs = self.docker.logs(container_id, options);
            let mut result = Vec::new();

            while let Some(log) = logs.next().await {
                match log {
                    Ok(log_output) => {
                        match log_output {
                            bollard::container::LogOutput::StdOut { message } |
                            bollard::container::LogOutput::StdErr { message } => {
                                result.push(String::from_utf8_lossy(&message).to_string());
                            }
                            _ => {}
                        }
                    }
                    Err(e) => return Err(anyhow!("Failed to get logs: {}", e)),
                }
            }

            Ok(result)
        }

        #[cfg(not(feature = "docker"))]
        {
            Err(anyhow!("Docker support not enabled"))
        }
    }

    /// Execute command in container
    pub async fn exec(&self, container_id: &str, cmd: Vec<String>) -> Result<String> {
        #[cfg(feature = "docker")]
        {
            let exec = self.docker.create_exec(container_id, CreateExecOptions {
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                cmd: Some(cmd),
                ..Default::default()
            }).await?;

            let output = self.docker.start_exec(&exec.id, None).await?;
            Ok(output.to_string())
        }

        #[cfg(not(feature = "docker"))]
        {
            Err(anyhow!("Docker support not enabled"))
        }
    }

    /// Check if Docker is available
    pub async fn is_available(&self) -> bool {
        #[cfg(feature = "docker")]
        {
            self.docker.ping().await.is_ok()
        }

        #[cfg(not(feature = "docker"))]
        {
            false
        }
    }

    /// Add directory to tar for build context
    #[cfg(feature = "docker")]
    fn add_dir_to_tar<W: std::io::Write>(
        builder: &mut Builder<W>,
        dir: &Path,
        prefix: &str,
    ) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let tar_path = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", prefix, name)
            };

            if path.is_dir() {
                let mut header = Header::new_gnu();
                header.set_size(0);
                header.set_entry_type(tar::EntryType::Directory);
                header.set_mode(0o755);
                header.set_cksum();
                builder.append_data(&mut header, &tar_path, std::io::empty())?;
                Self::add_dir_to_tar(builder, &path, &tar_path)?;
            } else {
                let mut file = std::fs::File::open(&path)?;
                let metadata = file.metadata()?;
                let mut header = Header::new_gnu();
                header.set_size(metadata.len());
                header.set_mode(0o644);
                header.set_cksum();
                builder.append_data(&mut header, &tar_path, &mut file)?;
            }
        }
        Ok(())
    }
}