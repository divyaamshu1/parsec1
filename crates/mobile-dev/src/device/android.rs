//! Android device management using direct ADB commands

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Result, anyhow};
use tokio::process::Command;
use tracing::{info, warn, debug};

use super::{DeviceInfo, DeviceType};

/// Android device manager using direct ADB commands
#[derive(Clone)]
pub struct AndroidDeviceManager {
    adb_path: PathBuf,
}

impl AndroidDeviceManager {
    /// Create new Android device manager
    pub fn new() -> Result<Self> {
        let adb_path = Self::find_adb()?;
        Ok(Self { adb_path })
    }

    /// Find ADB executable
    fn find_adb() -> Result<PathBuf> {
        // Check ANDROID_HOME
        if let Ok(home) = std::env::var("ANDROID_HOME") {
            let adb = PathBuf::from(home).join("platform-tools/adb");
            if adb.exists() {
                return Ok(adb);
            }
        }

        if let Ok(home) = std::env::var("ANDROID_SDK_ROOT") {
            let adb = PathBuf::from(home).join("platform-tools/adb");
            if adb.exists() {
                return Ok(adb);
            }
        }

        // Check PATH
        if let Ok(path) = which::which("adb") {
            return Ok(path);
        }

        // Check common locations
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let candidates: Vec<PathBuf> = vec![
            home.join("Android/Sdk/platform-tools/adb"),
            PathBuf::from("C:\\Android\\Sdk\\platform-tools\\adb.exe"),
            PathBuf::from("C:\\Program Files\\Android\\Sdk\\platform-tools\\adb.exe"),
            PathBuf::from("/usr/local/android-sdk/platform-tools/adb"),
            PathBuf::from("/opt/android-sdk/platform-tools/adb"),
        ];

        for path in candidates {
            if path.exists() {
                return Ok(path);
            }
        }

        Err(anyhow!("ADB not found. Please install Android SDK and ensure ADB is in PATH"))
    }

    /// Execute ADB command
    async fn adb_command(&self, args: &[&str]) -> Result<String> {
        let output = Command::new(&self.adb_path)
            .args(args)
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("ADB command failed: {}", error));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Execute ADB command for specific device
    async fn adb_device_command(&self, device_id: &str, args: &[&str]) -> Result<String> {
        let mut cmd_args = vec!["-s", device_id];
        cmd_args.extend_from_slice(args);
        self.adb_command(&cmd_args).await
    }

    /// Detect connected Android devices
    pub async fn detect_devices(&self) -> Result<Vec<DeviceInfo>> {
        let mut devices = Vec::new();

        let output = Command::new(&self.adb_path)
            .arg("devices")
            .output()
            .await?;

        let output_str = String::from_utf8(output.stdout)?;

        for line in output_str.lines().skip(1) {
            if line.contains("device") && !line.contains("offline") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if !parts.is_empty() {
                    let id = parts[0].to_string();
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
        let props = self.get_device_properties(device_id).await?;

        let name = props.get("ro.product.model")
            .unwrap_or(&"Unknown".to_string())
            .clone();

        let os_version = props.get("ro.build.version.release")
            .unwrap_or(&"Unknown".to_string())
            .clone();

        let api_level = props.get("ro.build.version.sdk")
            .and_then(|s| s.parse().ok());

        let abi = props.get("ro.product.cpu.abi").cloned();

        let is_emulator = device_id.starts_with("emulator-");

        Ok(DeviceInfo {
            id: device_id.to_string(),
            name,
            device_type: if is_emulator { DeviceType::Emulator } else { DeviceType::Physical },
            platform: "android".to_string(),
            os_version,
            api_level,
            is_connected: true,
            is_authorized: true,
            screen_size: None,
            abi,
        })
    }

    /// Get device properties via ADB
    async fn get_device_properties(&self, device_id: &str) -> Result<HashMap<String, String>> {
        let mut props = HashMap::new();
        let properties = vec![
            "ro.product.model",
            "ro.build.version.release",
            "ro.build.version.sdk",
            "ro.product.cpu.abi",
        ];

        for prop in properties {
            let output = self.adb_device_command(device_id, &["shell", "getprop", prop]).await?;
            let value = output.trim().to_string();
            if !value.is_empty() && value != "error: " {
                props.insert(prop.to_string(), value);
            }
        }

        Ok(props)
    }

    /// Install app on device
    pub async fn install_app(&self, device_id: &str, apk_path: &Path) -> Result<()> {
        info!("Installing {} on {}", apk_path.display(), device_id);

        let output = Command::new(&self.adb_path)
            .args(&["-s", device_id, "install", "-r", apk_path.to_str().unwrap()])
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Install failed: {}", error));
        }

        Ok(())
    }

    /// Run app on device
    pub async fn run_app(&self, device_id: &str, package_name: &str) -> Result<()> {
        info!("Running {} on {}", package_name, device_id);

        // Get main activity
        let main_activity = self.get_main_activity(device_id, package_name).await?;

        let output = Command::new(&self.adb_path)
            .args(&["-s", device_id, "shell", "am", "start", "-n", &format!("{}/{}", package_name, main_activity)])
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to start app: {}", error));
        }

        Ok(())
    }

    /// Get main activity for package
    async fn get_main_activity(&self, device_id: &str, package_name: &str) -> Result<String> {
        let output = self.adb_device_command(device_id, &["shell", "pm", "dump", package_name]).await?;

        for line in output.lines() {
            if line.contains("MAIN") && line.contains("android.intent.action.MAIN") {
                if let Some(activity) = line.split_whitespace().find(|w| w.contains(package_name)) {
                    return Ok(activity.trim_matches('/').to_string());
                }
            }
        }

        Err(anyhow!("Main activity not found for {}", package_name))
    }

    /// Take screenshot
    pub async fn take_screenshot(&self, device_id: &str, output_path: &Path) -> Result<()> {
        let temp_path = std::env::temp_dir().join("screenshot.png");

        let output = Command::new(&self.adb_path)
            .args(&["-s", device_id, "exec-out", "screencap", "-p"])
            .stdout(std::process::Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to take screenshot: {}", error));
        }

        std::fs::write(&temp_path, output.stdout)?;
        std::fs::copy(&temp_path, output_path)?;
        let _ = std::fs::remove_file(temp_path);

        Ok(())
    }

    /// Get device logs
    pub async fn get_logs(&self, device_id: &str) -> Result<Vec<String>> {
        let output = Command::new(&self.adb_path)
            .args(&["-s", device_id, "logcat", "-d"])
            .output()
            .await?;

        let logs = String::from_utf8(output.stdout)?
            .lines()
            .map(|s| s.to_string())
            .collect();

        Ok(logs)
    }

    /// Clear app data
    pub async fn clear_app_data(&self, device_id: &str, package_name: &str) -> Result<()> {
        self.adb_device_command(device_id, &["shell", "pm", "clear", package_name]).await?;
        Ok(())
    }

    /// Uninstall app
    pub async fn uninstall_app(&self, device_id: &str, package_name: &str) -> Result<()> {
        self.adb_device_command(device_id, &["uninstall", package_name]).await?;
        Ok(())
    }

    /// Forward port
    pub async fn forward_port(&self, device_id: &str, local: u16, remote: u16) -> Result<()> {
        self.adb_device_command(device_id, &["forward", &format!("tcp:{}", local), &format!("tcp:{}", remote)]).await?;
        Ok(())
    }

    /// Reverse port
    pub async fn reverse_port(&self, device_id: &str, remote: u16, local: u16) -> Result<()> {
        self.adb_device_command(device_id, &["reverse", &format!("tcp:{}", remote), &format!("tcp:{}", local)]).await?;
        Ok(())
    }

    /// Check if device is available
    pub async fn is_device_available(&self, device_id: &str) -> bool {
        self.adb_device_command(device_id, &["shell", "echo", "ok"]).await.is_ok()
    }

    /// Reboot device
    pub async fn reboot(&self, device_id: &str) -> Result<()> {
        self.adb_device_command(device_id, &["reboot"]).await?;
        Ok(())
    }
}