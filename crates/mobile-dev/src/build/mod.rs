//! Universal build system for mobile development

mod android;
mod ios;
mod common;

pub use android::*;
pub use ios::*;
pub use common::*;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::sync::{RwLock, Mutex};
use tracing::{info, warn, debug};

use crate::{MobileProject, PlatformType};

/// Build system
pub struct BuildSystem {
    build_dir: PathBuf,
    android: Arc<android::AndroidBuilder>,
    #[cfg(target_os = "macos")]
    ios: Arc<ios::IOSBuilder>,
    active_builds: Arc<RwLock<HashMap<usize, BuildHandle>>>,
    next_id: Arc<Mutex<usize>>,
}

/// Build configuration
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub target: PlatformType,
    pub configuration: BuildConfiguration,
    pub output_path: Option<PathBuf>,
    pub clean_before_build: bool,
    pub env_vars: HashMap<String, String>,
    pub extra_args: Vec<String>,
}

/// Build configuration type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildConfiguration {
    Debug,
    Profile,
    Release,
}

/// Build target
#[derive(Debug, Clone)]
pub struct BuildTarget {
    pub name: String,
    pub platform: String,
    pub arch: String,
    pub config: BuildConfiguration,
}

/// Build result
#[derive(Debug, Clone)]
pub struct BuildResult {
    pub success: bool,
    pub duration: std::time::Duration,
    pub output_path: Option<PathBuf>,
    pub logs: String,
    pub errors: String,
}

/// Build handle
#[derive(Debug)]
pub struct BuildHandle {
    pub id: usize,
    pub project: String,
    pub target: PlatformType,
    pub start_time: std::time::Instant,
    pub task: tokio::task::JoinHandle<BuildResult>,
}

impl BuildSystem {
    /// Create new build system
    pub fn new(build_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&build_dir)?;

        Ok(Self {
            build_dir,
            android: Arc::new(android::AndroidBuilder::new()?),
            #[cfg(target_os = "macos")]
            ios: Arc::new(ios::IOSBuilder::new()?),
            active_builds: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        })
    }

    /// Build a project
    pub async fn build(
        &self,
        project: &MobileProject,
        target: PlatformType,
        config: BuildConfig,
    ) -> Result<BuildResult> {
        info!("Building {} for {}", project.name, target);

        // Create output directory
        if let Some(output) = &config.output_path {
            std::fs::create_dir_all(output)?;
        }

        // Delegate to platform-specific builder
        let result = match target {
            PlatformType::Android => self.android.build(project, config).await,
            PlatformType::IOS => {
                #[cfg(target_os = "macos")]
                return self.ios.build(project, config).await;
                #[cfg(not(target_os = "macos"))]
                Err(anyhow!("iOS builds require macOS"))
            }
            _ => Err(anyhow!("Unsupported platform: {:?}", target)),
        }?;

        info!("Build {} for {}", 
            if result.success { "succeeded" } else { "failed" },
            project.name
        );

        Ok(result)
    }

    /// Start async build
    pub async fn start_build(
        &self,
        project: &MobileProject,
        target: PlatformType,
        config: BuildConfig,
    ) -> Result<usize> {
        let mut next_id = self.next_id.lock().await;
        let id = *next_id;
        *next_id += 1;

        let project_name = project.name.clone();
        let project_clone = project.clone();
        let build_dir = self.build_dir.clone();
        let android = self.android.clone();
        let target_clone = target.clone();
        #[cfg(target_os = "macos")]
        let ios = self.ios.clone();

        // Spawn build task
        let task = tokio::spawn(async move {
            let start = std::time::Instant::now();

            let result = match target_clone {
                PlatformType::Android => android.build(&project_clone, config).await,
                PlatformType::IOS => {
                    #[cfg(target_os = "macos")]
                    return ios.build(&project_clone, config).await;
                    #[cfg(not(target_os = "macos"))]
                    Err(anyhow!("iOS builds require macOS"))
                }
                _ => Err(anyhow!("Unsupported platform")),
            };

            match result {
                Ok(res) => res,
                Err(e) => BuildResult {
                    success: false,
                    duration: start.elapsed(),
                    output_path: None,
                    logs: String::new(),
                    errors: e.to_string(),
                }
            }
        });

        let handle = BuildHandle {
            id,
            project: project_name,
            target,
            start_time: std::time::Instant::now(),
            task,
        };

        self.active_builds.write().await.insert(id, handle);

        Ok(id)
    }

    /// Get build status
    pub async fn get_build_status(&self, id: usize) -> Option<BuildStatus> {
        let builds = self.active_builds.read().await;
        builds.get(&id).map(|handle| BuildStatus {
            id: handle.id,
            project: handle.project.clone(),
            target: handle.target.clone(),
            elapsed: handle.start_time.elapsed(),
            is_running: !handle.task.is_finished(),
        })
    }

    /// Cancel build
    pub async fn cancel_build(&self, id: usize) -> Result<()> {
        let mut builds = self.active_builds.write().await;
        if let Some(handle) = builds.remove(&id) {
            handle.task.abort();
        }
        Ok(())
    }

    /// List active builds
    pub async fn list_active_builds(&self) -> Vec<BuildStatus> {
        let builds = self.active_builds.read().await;
        builds.values().map(|handle| BuildStatus {
            id: handle.id,
            project: handle.project.clone(),
            target: handle.target.clone(),
            elapsed: handle.start_time.elapsed(),
            is_running: !handle.task.is_finished(),
        }).collect()
    }

    /// Get build targets for project
    pub async fn get_build_targets(&self, project: &MobileProject) -> Vec<BuildTarget> {
        match project.framework {
            crate::frameworks::FrameworkType::Flutter => {
                vec![
                    BuildTarget {
                        name: "android-arm64".to_string(),
                        platform: "android".to_string(),
                        arch: "arm64".to_string(),
                        config: BuildConfiguration::Debug,
                    },
                    BuildTarget {
                        name: "android-x64".to_string(),
                        platform: "android".to_string(),
                        arch: "x64".to_string(),
                        config: BuildConfiguration::Debug,
                    },
                    BuildTarget {
                        name: "ios-arm64".to_string(),
                        platform: "ios".to_string(),
                        arch: "arm64".to_string(),
                        config: BuildConfiguration::Debug,
                    },
                ]
            }
            crate::frameworks::FrameworkType::NativeAndroid => {
                vec![
                    BuildTarget {
                        name: "android-arm64-debug".to_string(),
                        platform: "android".to_string(),
                        arch: "arm64".to_string(),
                        config: BuildConfiguration::Debug,
                    },
                    BuildTarget {
                        name: "android-arm64-release".to_string(),
                        platform: "android".to_string(),
                        arch: "arm64".to_string(),
                        config: BuildConfiguration::Release,
                    },
                ]
            }
            _ => vec![],
        }
    }
}

/// Build status
#[derive(Debug, Clone)]
pub struct BuildStatus {
    pub id: usize,
    pub project: String,
    pub target: PlatformType,
    pub elapsed: std::time::Duration,
    pub is_running: bool,
}