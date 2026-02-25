//! iOS build system (macOS only)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use tokio::process::Command;
use tracing::{info, warn, debug};

#[cfg(target_os = "macos")]
use super::{BuildConfig, BuildResult, BuildConfiguration};
#[cfg(target_os = "macos")]
use crate::{MobileProject, PlatformType};

// For non-macOS compilation
#[cfg(not(target_os = "macos"))]
use super::{BuildConfig, BuildResult, BuildConfiguration};
#[cfg(not(target_os = "macos"))]
use crate::{MobileProject, PlatformType};

#[cfg(target_os = "macos")]
/// iOS builder
#[derive(Clone)]
pub struct IOSBuilder {
    xcodebuild_path: PathBuf,
}

#[cfg(target_os = "macos")]
impl IOSBuilder {
    /// Create new iOS builder
    pub fn new() -> Result<Self> {
        // Find xcodebuild
        let output = std::process::Command::new("xcrun")
            .arg("--find")
            .arg("xcodebuild")
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("xcodebuild not found"));
        }

        let path = String::from_utf8(output.stdout)?
            .trim()
            .to_string();

        Ok(Self {
            xcodebuild_path: PathBuf::from(path),
        })
    }

    /// Build iOS project
    pub async fn build(&self, project: &MobileProject, config: BuildConfig) -> Result<BuildResult> {
        let start = std::time::Instant::now();

        // Find Xcode project
        let xcodeproj = self.find_xcode_project(&project.path).await?;

        // Determine scheme
        let scheme = self.find_scheme(&xcodeproj).await?;

        // Build configuration
        let configuration = match config.configuration {
            BuildConfiguration::Debug => "Debug",
            BuildConfiguration::Profile => "Profile",
            BuildConfiguration::Release => "Release",
        };

        // Build command
        let mut cmd = Command::new(&self.xcodebuild_path);
        cmd.current_dir(&project.path);
        cmd.args(&[
            "-project", xcodeproj.to_str().unwrap(),
            "-scheme", &scheme,
            "-configuration", configuration,
            "-sdk", "iphoneos",
            "build",
        ]);

        // Set environment
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }

        // Run build
        let output = cmd.output().await?;

        // Find output app
        let output_path = if output.status.success() {
            self.find_output_app(project, &scheme, configuration).await?
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

    /// Find Xcode project
    async fn find_xcode_project(&self, path: &Path) -> Result<PathBuf> {
        let mut entries = tokio::fs::read_dir(path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("xcodeproj") {
                return Ok(path);
            }
        }

        Err(anyhow!("No Xcode project found"))
    }

    /// Find scheme
    async fn find_scheme(&self, project: &Path) -> Result<String> {
        let output = Command::new(&self.xcodebuild_path)
            .arg("-project")
            .arg(project)
            .arg("-list")
            .output()
            .await?;

        let output_str = String::from_utf8(output.stdout)?;

        for line in output_str.lines() {
            if line.trim().starts_with("Scheme:") {
                return Ok(line.split(':').nth(1).unwrap().trim().to_string());
            }
        }

        Err(anyhow!("No scheme found"))
    }

    /// Find output app
    async fn find_output_app(&self, project: &MobileProject, scheme: &str, configuration: &str) -> Result<Option<PathBuf>> {
        let derived_data = dirs::home_dir()
            .unwrap()
            .join("Library/Developer/Xcode/DerivedData");

        let project_name = project.path.file_stem().unwrap().to_string_lossy();
        let derived_project = format!("{}-", project_name);

        // Find derived data folder
        let mut entries = tokio::fs::read_dir(derived_data).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(&derived_project) {
                    let app_path = path
                        .join("Build/Products")
                        .join(format!("{}-iphoneos", configuration))
                        .join(format!("{}.app", scheme));

                    if app_path.exists() {
                        return Ok(Some(app_path));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Create IPA
    pub async fn create_ipa(&self, app_path: &Path, output_path: &Path) -> Result<()> {
        let payload_dir = output_path.join("Payload");
        tokio::fs::create_dir_all(&payload_dir).await?;

        // Copy .app to Payload
        let app_name = app_path.file_name().unwrap();
        let dest_app = payload_dir.join(app_name);
        Self::copy_dir(app_path, &dest_app).await?;

        // Zip to IPA
        let output = Command::new("zip")
            .arg("-r")
            .arg(output_path.join("app.ipa"))
            .arg("Payload")
            .current_dir(output_path)
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("Failed to create IPA"));
        }

        Ok(())
    }

    /// Copy directory recursively
    async fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
        tokio::fs::create_dir_all(dst).await?;

        let mut entries = tokio::fs::read_dir(src).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let dest_path = dst.join(entry.file_name());

            if path.is_dir() {
                Self::copy_dir(&path, &dest_path).await?;
            } else {
                tokio::fs::copy(&path, &dest_path).await?;
            }
        }

        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
#[derive(Clone)]
pub struct IOSBuilder;

#[cfg(not(target_os = "macos"))]
impl IOSBuilder {
    pub fn new() -> Result<Self> { Ok(Self) }
    pub async fn build(&self, _project: &MobileProject, _config: BuildConfig) -> Result<BuildResult> {
        Err(anyhow!("iOS builds require macOS"))
    }
}