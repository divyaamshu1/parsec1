//! Android emulator management

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, anyhow};
use tokio::process::Command as TokioCommand;
use tracing::{info, warn, debug};

use super::DeviceInfo;

/// Android emulator manager
#[derive(Clone)]
pub struct EmulatorManager {
    avdmanager_path: PathBuf,
    emulator_path: PathBuf,
    adb_path: PathBuf,
}

/// Emulator configuration
#[derive(Debug, Clone)]
pub struct EmulatorConfig {
    pub name: String,
    pub device: String,
    pub target: String,
    pub abi: String,
    pub skin: Option<String>,
    pub ram: Option<u32>,
    pub disk: Option<u32>,
}

impl EmulatorManager {
    /// Create new emulator manager
    pub fn new() -> Result<Self> {
        let sdk_path = std::env::var("ANDROID_HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("ANDROID_SDK_ROOT").map(PathBuf::from))
            .map_err(|_| anyhow!("Android SDK not found"))?;

        Ok(Self {
            avdmanager_path: sdk_path.join("tools/bin/avdmanager"),
            emulator_path: sdk_path.join("emulator/emulator"),
            adb_path: sdk_path.join("platform-tools/adb"),
        })
    }

    /// List available emulators
    pub async fn list_emulators(&self) -> Result<Vec<String>> {
        let output = TokioCommand::new(&self.emulator_path)
            .arg("-list-avds")
            .output()
            .await?;

        let emulators = String::from_utf8(output.stdout)?
            .lines()
            .map(|s| s.to_string())
            .collect();

        Ok(emulators)
    }

    /// Create new emulator
    pub async fn create_emulator(&self, config: EmulatorConfig) -> Result<String> {
        let mut cmd = TokioCommand::new(&self.avdmanager_path);
        cmd.args(&[
            "create", "avd",
            "-n", &config.name,
            "-k", &format!("system-images;{};{};{}", config.target, config.abi, config.device),
            "-d", &config.device,
        ]);

        if let Some(skin) = &config.skin {
            cmd.arg("-s").arg(skin);
        }

        if let Some(ram) = config.ram {
            cmd.arg("-r").arg(ram.to_string());
        }

        if let Some(disk) = config.disk {
            cmd.arg("-d").arg(disk.to_string());
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to create emulator: {}", error));
        }

        Ok(config.name)
    }

    /// Start emulator
    pub async fn start_emulator(&self, name: &str, headless: bool) -> Result<()> {
        let mut cmd = TokioCommand::new(&self.emulator_path);
        cmd.arg("-avd").arg(name);

        if headless {
            cmd.arg("-no-window");
        }

        // Start detached
        cmd.spawn()?;

        // Wait for emulator to boot
        self.wait_for_boot(name).await?;

        Ok(())
    }

    /// Stop emulator
    pub async fn stop_emulator(&self, name: &str) -> Result<()> {
        // Find emulator process
        let output = TokioCommand::new("pgrep")
            .arg("-f")
            .arg(format!("emulator.*-avd.*{}", name))
            .output()
            .await?;

        if let Ok(pid_str) = String::from_utf8(output.stdout) {
            if let Some(pid) = pid_str.lines().next() {
                // Kill process
                TokioCommand::new("kill")
                    .arg("-9")
                    .arg(pid)
                    .spawn()?;
            }
        }

        Ok(())
    }

    /// Wait for emulator to boot
    async fn wait_for_boot(&self, name: &str) -> Result<()> {
        let mut attempts = 0;
        let max_attempts = 60;

        while attempts < max_attempts {
            let output = TokioCommand::new(&self.adb_path)
                .args(&["-s", name, "shell", "getprop", "sys.boot_completed"])
                .output()
                .await?;

            let boot_completed = String::from_utf8(output.stdout)?
                .trim()
                .parse::<u32>()
                .unwrap_or(0);

            if boot_completed == 1 {
                return Ok(());
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            attempts += 1;
        }

        Err(anyhow!("Emulator failed to boot"))
    }

    /// Get emulator info
    pub async fn get_emulator_info(&self, name: &str) -> Result<DeviceInfo> {
        use crate::device::DeviceType;

        Ok(DeviceInfo {
            id: name.to_string(),
            name: name.to_string(),
            device_type: DeviceType::Emulator,
            platform: "android".to_string(),
            os_version: "unknown".to_string(),
            api_level: None,
            is_connected: true,
            is_authorized: true,
            screen_size: None,
            abi: None,
        })
    }
}