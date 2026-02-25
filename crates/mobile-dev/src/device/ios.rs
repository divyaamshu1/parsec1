//! iOS device management (macOS only)

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, anyhow};
use tokio::process::Command as TokioCommand;
use tracing::{info, warn, debug};

use super::{DeviceInfo, DeviceType};

#[cfg(target_os = "macos")]
/// iOS device manager
#[derive(Clone)]
pub struct IOSDeviceManager {
    idevice_id_path: Option<PathBuf>,
    idevicesyslog_path: Option<PathBuf>,
    idevicescreenshot_path: Option<PathBuf>,
}

#[cfg(target_os = "macos")]
impl IOSDeviceManager {
    /// Create new iOS device manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            idevice_id_path: which::which("idevice_id").ok(),
            idevicesyslog_path: which::which("idevicesyslog").ok(),
            idevicescreenshot_path: which::which("idevicescreenshot").ok(),
        })
    }

    /// Detect connected iOS devices
    pub async fn detect_devices(&self) -> Result<Vec<DeviceInfo>> {
        let mut devices = Vec::new();

        if let Some(idevice_id) = &self.idevice_id_path {
            let output = TokioCommand::new(idevice_id)
                .arg("-l")
                .output()
                .await?;

            let output_str = String::from_utf8(output.stdout)?;

            for line in output_str.lines() {
                if !line.is_empty() {
                    let id = line.trim().to_string();
                    if let Ok(info) = self.get_device_info(&id).await {
                        devices.push(info);
                    }
                }
            }
        }

        Ok(devices)
    }

    /// Get detailed device information
    async fn get_device_info(&self, device_id: &str) -> Result<DeviceInfo> {
        // Get device name
        let name = if let Some(idevicename) = which::which("idevicename").ok() {
            let output = TokioCommand::new(idevicename)
                .arg("-u")
                .arg(device_id)
                .output()
                .await?;
            String::from_utf8(output.stdout)?.trim().to_string()
        } else {
            "iPhone".to_string()
        };

        // Get device version
        let version = if let Some(ideviceinfo) = which::which("ideviceinfo").ok() {
            let output = TokioCommand::new(ideviceinfo)
                .arg("-u")
                .arg(device_id)
                .arg("-k")
                .arg("ProductVersion")
                .output()
                .await?;
            String::from_utf8(output.stdout)?.trim().to_string()
        } else {
            "unknown".to_string()
        };

        Ok(DeviceInfo {
            id: device_id.to_string(),
            name,
            device_type: DeviceType::Physical,
            platform: "ios".to_string(),
            os_version: version,
            api_level: None,
            is_connected: true,
            is_authorized: true,
            screen_size: None,
            abi: None,
        })
    }

    /// Install app on device
    pub async fn install_app(&self, device_id: &str, ipa_path: &Path) -> Result<()> {
        if let Some(ideviceinstaller) = which::which("ideviceinstaller").ok() {
            let output = TokioCommand::new(ideviceinstaller)
                .arg("-u")
                .arg(device_id)
                .arg("-i")
                .arg(ipa_path)
                .output()
                .await?;

            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!("Install failed: {}", error));
            }

            Ok(())
        } else {
            Err(anyhow!("ideviceinstaller not found"))
        }
    }

    /// Run app on device
    pub async fn run_app(&self, device_id: &str, bundle_id: &str) -> Result<()> {
        if let Some(idevicesyslog) = &self.idevicesyslog_path {
            // Just launch the app - we can't easily launch from command line
            info!("Please launch {} manually on device {}", bundle_id, device_id);
            Ok(())
        } else {
            Err(anyhow!("iOS device tools not found"))
        }
    }

    /// Take screenshot
    pub async fn take_screenshot(&self, device_id: &str, output_path: &Path) -> Result<()> {
        if let Some(idevicescreenshot) = &self.idevicescreenshot_path {
            let output = TokioCommand::new(idevicescreenshot)
                .arg("-u")
                .arg(device_id)
                .arg(output_path)
                .output()
                .await?;

            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!("Screenshot failed: {}", error));
            }

            Ok(())
        } else {
            Err(anyhow!("idevicescreenshot not found"))
        }
    }

    /// Get device logs
    pub async fn get_logs(&self, device_id: &str) -> Result<Vec<String>> {
        if let Some(idevicesyslog) = &self.idevicesyslog_path {
            // Capture last 100 lines of logs
            let output = TokioCommand::new(idevicesyslog)
                .arg("-u")
                .arg(device_id)
                .arg("--last")
                .arg("100")
                .output()
                .await?;

            let logs = String::from_utf8(output.stdout)?
                .lines()
                .map(|s| s.to_string())
                .collect();

            Ok(logs)
        } else {
            Err(anyhow!("idevicesyslog not found"))
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[derive(Clone)]
pub struct IOSDeviceManager;

#[cfg(not(target_os = "macos"))]
impl IOSDeviceManager {
    pub fn new() -> Result<Self> { Ok(Self) }
    pub async fn detect_devices(&self) -> Result<Vec<DeviceInfo>> { Ok(vec![]) }
    pub async fn install_app(&self, _device_id: &str, _ipa_path: &Path) -> Result<()> { 
        Err(anyhow!("iOS development requires macOS")) 
    }
    pub async fn run_app(&self, _device_id: &str, _bundle_id: &str) -> Result<()> { 
        Err(anyhow!("iOS development requires macOS")) 
    }
    pub async fn take_screenshot(&self, _device_id: &str, _output_path: &Path) -> Result<()> { 
        Err(anyhow!("iOS development requires macOS")) 
    }
    pub async fn get_logs(&self, _device_id: &str) -> Result<Vec<String>> { 
        Err(anyhow!("iOS development requires macOS")) 
    }
}