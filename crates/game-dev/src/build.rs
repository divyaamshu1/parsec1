//! Universal build system for game engines

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::sync::Mutex;
use tracing::{info, warn, debug};

use crate::engine::GameEngine;
use crate::project::Project;

/// Build system
pub struct BuildSystem {
    build_dir: PathBuf,
    active_builds: Arc<Mutex<HashMap<usize, BuildHandle>>>,
    next_id: Arc<Mutex<usize>>,
}

/// Build configuration
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub target: BuildTarget,
    pub configuration: BuildConfiguration,
    pub output_path: Option<PathBuf>,
    pub clean_before_build: bool,
    pub env_vars: HashMap<String, String>,
    pub extra_args: Vec<String>,
}

/// Build target
#[derive(Debug, Clone)]
pub struct BuildTarget {
    pub name: String,
    pub platform: String,
    pub arch: String,
    pub config: String,
}

/// Build configuration type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildConfiguration {
    Debug,
    Development,
    Release,
    Shipping,
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

/// Build handle for active builds
#[derive(Debug)]
pub struct BuildHandle {
    pub id: usize,
    pub project: String,
    pub target: BuildTarget,
    pub start_time: std::time::Instant,
    pub process: tokio::process::Child,
}

impl BuildSystem {
    /// Create new build system
    pub fn new(build_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&build_dir)?;

        Ok(Self {
            build_dir,
            active_builds: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        })
    }

    /// Build a project
    pub async fn build(
        &self,
        project: &Project,
        engine: &dyn GameEngine,
        config: BuildConfig,
    ) -> Result<BuildResult> {
        info!("Building {} for {}", project.name(), config.target.name);

        // Clean if requested
        if config.clean_before_build {
            if let Some(output) = &config.output_path {
                if output.exists() {
                    std::fs::remove_dir_all(output)?;
                }
            }
        }

        // Create output directory
        if let Some(output) = &config.output_path {
            std::fs::create_dir_all(output)?;
        }

        // Delegate to engine-specific build
        let result = engine.build(project, &config).await?;

        info!("Build {} for {}", 
            if result.success { "succeeded" } else { "failed" },
            project.name()
        );

        Ok(result)
    }

    /// Start an async build
    pub async fn start_build(
        &self,
        project: &Project,
        engine: &dyn GameEngine,
        config: BuildConfig,
    ) -> Result<usize> {
        let mut next_id = self.next_id.lock().await;
        let id = *next_id;
        *next_id += 1;

        // Clone what we need for the build task
        let project_path = project.path().to_path_buf();
        let engine_type = engine.engine_type();
        let build_dir = self.build_dir.clone();
        let config_clone = config.clone();

        // Spawn build task
        let handle = tokio::spawn(async move {
            // Build logic here
            // This would call the actual build process
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            BuildResult {
                success: true,
                duration: std::time::Duration::from_secs(5),
                output_path: config_clone.output_path,
                logs: "Build completed".to_string(),
                errors: String::new(),
            }
        });

        // Store handle
        let build_handle = BuildHandle {
            id,
            project: project.name().to_string(),
            target: config.target,
            start_time: std::time::Instant::now(),
            process: tokio::process::Command::new("cmd").arg("/c").arg("echo").spawn()?,
        };

        self.active_builds.lock().await.insert(id, build_handle);

        Ok(id)
    }

    /// Get build status
    pub async fn get_build_status(&self, id: usize) -> Option<BuildStatus> {
        let builds = self.active_builds.lock().await;
        builds.get(&id).map(|handle| BuildStatus {
            id: handle.id,
            project: handle.project.clone(),
            target: handle.target.clone(),
            elapsed: handle.start_time.elapsed(),
            is_running: true, // Would need to check process
        })
    }

    /// Cancel a build
    pub async fn cancel_build(&self, id: usize) -> Result<()> {
        let mut builds = self.active_builds.lock().await;
        if let Some(handle) = builds.remove(&id) {
            // Kill process
            // handle.process.kill().await?;
        }
        Ok(())
    }

    /// List active builds
    pub async fn list_active_builds(&self) -> Vec<BuildStatus> {
        let builds = self.active_builds.lock().await;
        builds.values().map(|handle| BuildStatus {
            id: handle.id,
            project: handle.project.clone(),
            target: handle.target.clone(),
            elapsed: handle.start_time.elapsed(),
            is_running: true,
        }).collect()
    }

    /// Get available build targets for a project
    pub async fn get_build_targets(&self, engine: &dyn GameEngine) -> Vec<BuildTarget> {
        engine.build_targets()
    }
}

/// Build status
#[derive(Debug, Clone)]
pub struct BuildStatus {
    pub id: usize,
    pub project: String,
    pub target: BuildTarget,
    pub elapsed: std::time::Duration,
    pub is_running: bool,
}