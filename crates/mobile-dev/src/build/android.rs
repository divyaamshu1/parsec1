//! Android build system

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use tokio::process::Command;
use tracing::{info, warn, debug};

use super::{BuildConfig, BuildResult, BuildConfiguration};
use crate::{MobileProject, PlatformType};

/// Android builder
#[derive(Clone)]
pub struct AndroidBuilder {
    gradle_wrapper: Option<PathBuf>,
}

impl AndroidBuilder {
    /// Create new Android builder
    pub fn new() -> Result<Self> {
        Ok(Self {
            gradle_wrapper: None,
        })
    }

    /// Build Android project
    pub async fn build(&self, project: &MobileProject, config: BuildConfig) -> Result<BuildResult> {
        let start = std::time::Instant::now();

        // Check if it's a Gradle project
        let gradle_wrapper = project.path.join("gradlew");
        if !gradle_wrapper.exists() {
            return Err(anyhow!("Not a Gradle project"));
        }

        // Determine build task
        let task = match config.configuration {
            BuildConfiguration::Debug => "assembleDebug",
            BuildConfiguration::Profile => "assembleProfile",
            BuildConfiguration::Release => "assembleRelease",
        };

        // Build command
        let mut cmd = Command::new(&gradle_wrapper);
        cmd.current_dir(&project.path);
        cmd.arg(task);

        // Add extra args
        for arg in &config.extra_args {
            cmd.arg(arg);
        }

        // Set environment variables
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }

        // Run build
        let output = cmd.output().await?;

        // Find output APK
        let output_path = if output.status.success() {
            self.find_output_apk(project, &config).await?
        } else {
            None
        };

        Ok(BuildResult {
            success: output.status.success(),
            duration: start.elapsed(),
            output_path,
            logs: String::from_utf8_lossy(&output.stdout).to_string(),
            errors: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    /// Find output APK after build
    async fn find_output_apk(&self, project: &MobileProject, config: &BuildConfig) -> Result<Option<PathBuf>> {
        let build_type = match config.configuration {
            BuildConfiguration::Debug => "debug",
            BuildConfiguration::Profile => "profile",
            BuildConfiguration::Release => "release",
        };

        let apk_path = project.path
            .join("app/build/outputs/apk")
            .join(build_type)
            .join(format!("app-{}.apk", build_type));

        if apk_path.exists() {
            Ok(Some(apk_path))
        } else {
            Ok(None)
        }
    }

    /// Build Android app bundle
    pub async fn build_bundle(&self, project: &MobileProject, config: BuildConfig) -> Result<BuildResult> {
        let start = std::time::Instant::now();

        let gradle_wrapper = project.path.join("gradlew");
        if !gradle_wrapper.exists() {
            return Err(anyhow!("Not a Gradle project"));
        }

        let task = match config.configuration {
            BuildConfiguration::Debug => "bundleDebug",
            BuildConfiguration::Profile => "bundleProfile",
            BuildConfiguration::Release => "bundleRelease",
        };

        let output = Command::new(&gradle_wrapper)
            .current_dir(&project.path)
            .arg(task)
            .output()
            .await?;

        let output_path = if output.status.success() {
            let build_type = match config.configuration {
                BuildConfiguration::Debug => "debug",
                BuildConfiguration::Profile => "profile",
                BuildConfiguration::Release => "release",
            };

            let aab_path = project.path
                .join("app/build/outputs/bundle")
                .join(build_type)
                .join(format!("app-{}.aab", build_type));

            if aab_path.exists() {
                Some(aab_path)
            } else {
                None
            }
        } else {
            None
        };

        Ok(BuildResult {
            success: output.status.success(),
            duration: start.elapsed(),
            output_path,
            logs: String::from_utf8_lossy(&output.stdout).to_string(),
            errors: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    /// Sign APK
    pub async fn sign_apk(&self, apk_path: &Path, keystore: &Path, password: &str, alias: &str) -> Result<PathBuf> {
        let signed_path = apk_path.with_extension("signed.apk");

        let output = Command::new("apksigner")
            .args(&[
                "sign",
                "--ks", keystore.to_str().unwrap(),
                "--ks-pass", &format!("pass:{}", password),
                "--ks-key-alias", alias,
                "--out", signed_path.to_str().unwrap(),
                apk_path.to_str().unwrap(),
            ])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("Failed to sign APK"));
        }

        Ok(signed_path)
    }
}