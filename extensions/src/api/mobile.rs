//! Mobile Development API for Extensions
//!
//! This module provides APIs for mobile development extensions to interact with
//! Android SDK, iOS tooling, emulators, and device management.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock, mpsc};
use tokio::process::Command;

use crate::runtime::ExtensionHandle;

/// Mobile platform types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MobilePlatform {
    Android,
    iOS,
    Flutter,
    ReactNative,
    Ionic,
}

/// Mobile device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileDevice {
    pub id: String,
    pub name: String,
    pub platform: MobilePlatform,
    pub model: Option<String>,
    pub os_version: String,
    pub is_emulator: bool,
    pub is_connected: bool,
    pub api_level: Option<u32>,
    pub screen_size: Option<(u32, u32)>,
    pub abi: Option<String>,
}

/// Emulator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorConfig {
    pub name: String,
    pub platform: MobilePlatform,
    pub device_type: String,
    pub system_image: String,
    pub ram_mb: Option<u32>,
    pub disk_mb: Option<u32>,
    pub screen_resolution: Option<(u32, u32)>,
    pub screen_density: Option<u32>,
    pub show_skins: bool,
    pub headless: bool,
}

/// Build configuration for mobile apps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub platform: MobilePlatform,
    pub configuration: BuildConfiguration,
    pub output_path: Option<PathBuf>,
    pub signing_config: Option<SigningConfig>,
    pub extra_args: Vec<String>,
}

/// Build configuration type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildConfiguration {
    Debug,
    Release,
    Profile,
}

/// Signing configuration for apps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningConfig {
    pub keystore_path: PathBuf,
    pub keystore_password: String,
    pub key_alias: String,
    pub key_password: Option<String>,
    pub provisioning_profile: Option<PathBuf>,
}

/// Mobile API for extensions
pub struct MobileAPI {
    /// Connected devices
    devices: Arc<RwLock<HashMap<String, MobileDevice>>>,
    /// Running emulators
    emulators: Arc<RwLock<HashMap<String, EmulatorHandle>>>,
    /// Build processes
    builds: Arc<RwLock<HashMap<String, BuildHandle>>>,
    /// Extension handle
    extension: ExtensionHandle,
}

/// Handle to a running emulator
pub struct EmulatorHandle {
    pub id: String,
    pub config: EmulatorConfig,
    pub process: tokio::process::Child,
    pub vnc_port: Option<u16>,
    pub adb_port: Option<u16>,
}

/// Handle to a build process
pub struct BuildHandle {
    pub id: String,
    pub config: BuildConfig,
    pub process: tokio::process::Child,
    pub output_path: Option<PathBuf>,
    pub start_time: std::time::Instant,
}

impl MobileAPI {
    /// Create a new MobileAPI instance
    pub fn new(extension: ExtensionHandle) -> Self {
        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            emulators: Arc::new(RwLock::new(HashMap::new())),
            builds: Arc::new(RwLock::new(HashMap::new())),
            extension,
        }
    }

    // ==================== Device Management ====================

    /// List connected devices
    pub async fn list_devices(&self, platform: Option<MobilePlatform>) -> Vec<MobileDevice> {
        let devices = self.devices.read().await;
        devices
            .values()
            .filter(|d| platform.map_or(true, |p| d.platform == p))
            .cloned()
            .collect()
    }

    /// Get device details
    pub async fn get_device(&self, id: &str) -> Option<MobileDevice> {
        self.devices.read().await.get(id).cloned()
    }

    /// Refresh device list (discover new devices)
    pub async fn refresh_devices(&self) -> Result<Vec<MobileDevice>> {
        let mut devices = self.devices.write().await;
        devices.clear();

        // Detect Android devices via ADB
        if let Ok(adb_devices) = self.detect_android_devices().await {
            for device in adb_devices {
                devices.insert(device.id.clone(), device);
            }
        }

        // Detect iOS devices via idevice_id (macOS only)
        #[cfg(target_os = "macos")]
        if let Ok(ios_devices) = self.detect_ios_devices().await {
            for device in ios_devices {
                devices.insert(device.id.clone(), device);
            }
        }

        Ok(devices.values().cloned().collect())
    }

    /// Connect to a device
    pub async fn connect_device(&self, device_id: &str) -> Result<()> {
        // For ADB devices
        if device_id.starts_with("emulator-") || device_id.contains('.') {
            let output = Command::new("adb")
                .args(&["connect", device_id])
                .output()
                .await?;

            if !output.status.success() {
                return Err(anyhow!("Failed to connect to device: {}", 
                    String::from_utf8_lossy(&output.stderr)));
            }
        }

        self.refresh_devices().await?;
        Ok(())
    }

    /// Disconnect from a device
    pub async fn disconnect_device(&self, device_id: &str) -> Result<()> {
        if device_id.starts_with("emulator-") || device_id.contains('.') {
            let output = Command::new("adb")
                .args(&["disconnect", device_id])
                .output()
                .await?;

            if !output.status.success() {
                return Err(anyhow!("Failed to disconnect device: {}", 
                    String::from_utf8_lossy(&output.stderr)));
            }
        }

        self.devices.write().await.remove(device_id);
        Ok(())
    }

    // ==================== Emulator Management ====================

    /// Create a new emulator
    pub async fn create_emulator(&self, config: EmulatorConfig) -> Result<String> {
        match config.platform {
            MobilePlatform::Android => {
                self.create_android_emulator(config).await
            }
            MobilePlatform::iOS => {
                #[cfg(target_os = "macos")]
                return self.create_ios_simulator(config).await;
                #[cfg(not(target_os = "macos"))]
                Err(anyhow!("iOS emulators only available on macOS"))
            }
            _ => Err(anyhow!("Emulator not supported for {:?}", config.platform)),
        }
    }

    /// Start an emulator
    pub async fn start_emulator(&self, name: &str) -> Result<String> {
        // Check if already running
        if self.emulators.read().await.contains_key(name) {
            return Ok(name.to_string());
        }

        let mut child = Command::new("emulator")
            .args(&["-avd", name, "-no-boot-anim", "-gpu", "auto"])
            .spawn()?;

        // Wait for boot
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let id = uuid::Uuid::new_v4().to_string();
        let handle = EmulatorHandle {
            id: id.clone(),
            config: EmulatorConfig {
                name: name.to_string(),
                platform: MobilePlatform::Android,
                device_type: "default".to_string(),
                system_image: "default".to_string(),
                ram_mb: None,
                disk_mb: None,
                screen_resolution: None,
                screen_density: None,
                show_skins: false,
                headless: false,
            },
            process: child,
            vnc_port: Some(5900), // Would detect actual port
            adb_port: Some(5554),
        };

        self.emulators.write().await.insert(id.clone(), handle);
        Ok(id)
    }

    /// Stop an emulator
    pub async fn stop_emulator(&self, id: &str) -> Result<()> {
        let mut emulators = self.emulators.write().await;
        if let Some(mut handle) = emulators.remove(id) {
            handle.process.kill().await?;
        }
        Ok(())
    }

    /// List running emulators
    pub async fn list_emulators(&self) -> Vec<EmulatorInfo> {
        let emulators = self.emulators.read().await;
        emulators
            .values()
            .map(|h| EmulatorInfo {
                id: h.id.clone(),
                name: h.config.name.clone(),
                platform: h.config.platform,
                vnc_port: h.vnc_port,
                adb_port: h.adb_port,
            })
            .collect()
    }

    // ==================== Build Management ====================

    /// Build a mobile app
    pub async fn build_app(&self, project_path: &Path, config: BuildConfig) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();

        let mut cmd = match config.platform {
            MobilePlatform::Android => {
                let mut cmd = Command::new("./gradlew");
                cmd.current_dir(project_path);
                
                match config.configuration {
                    BuildConfiguration::Debug => cmd.arg("assembleDebug"),
                    BuildConfiguration::Release => cmd.arg("assembleRelease"),
                    BuildConfiguration::Profile => cmd.arg("assembleProfile"),
                };
                cmd
            }
            MobilePlatform::iOS => {
                let mut cmd = Command::new("xcodebuild");
                cmd.current_dir(project_path);
                
                match config.configuration {
                    BuildConfiguration::Debug => cmd.args(&["-configuration", "Debug"]),
                    BuildConfiguration::Release => cmd.args(&["-configuration", "Release"]),
                    BuildConfiguration::Profile => cmd.args(&["-configuration", "Profile"]),
                };
                cmd
            }
            MobilePlatform::Flutter => {
                let mut cmd = Command::new("flutter");
                cmd.current_dir(project_path);
                
                cmd.arg("build");
                match config.platform {
                    MobilePlatform::Android => cmd.arg("apk"),
                    MobilePlatform::iOS => cmd.arg("ios"),
                    _ => cmd.arg("web"),
                };
                cmd
            }
            _ => return Err(anyhow!("Build not supported for {:?}", config.platform)),
        };

        // Add extra args
        if !config.extra_args.is_empty() {
            cmd.args(&config.extra_args);
        }

        let child = cmd.spawn()?;

        let handle = BuildHandle {
            id: id.clone(),
            config,
            process: child,
            output_path: None,
            start_time: std::time::Instant::now(),
        };

        self.builds.write().await.insert(id.clone(), handle);
        Ok(id)
    }

    /// Get build status
    pub async fn build_status(&self, id: &str) -> Option<BuildStatus> {
        let builds = self.builds.read().await;
        builds.get(id).map(|handle| BuildStatus {
            id: handle.id.clone(),
            is_running: handle.process.try_wait().ok().flatten().is_none(),
            elapsed: handle.start_time.elapsed(),
            output_path: handle.output_path.clone(),
        })
    }

    /// Cancel a build
    pub async fn cancel_build(&self, id: &str) -> Result<()> {
        let mut builds = self.builds.write().await;
        if let Some(mut handle) = builds.remove(id) {
            handle.process.kill().await?;
        }
        Ok(())
    }

    // ==================== App Installation ====================

    /// Install app on device
    pub async fn install_app(&self, device_id: &str, apk_path: &Path) -> Result<()> {
        let output = Command::new("adb")
            .args(&["-s", device_id, "install", "-r", apk_path.to_str().unwrap()])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("Install failed: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }

        Ok(())
    }

    /// Uninstall app from device
    pub async fn uninstall_app(&self, device_id: &str, package_name: &str) -> Result<()> {
        let output = Command::new("adb")
            .args(&["-s", device_id, "uninstall", package_name])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("Uninstall failed: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }

        Ok(())
    }

    /// Run app on device
    pub async fn run_app(&self, device_id: &str, package_name: &str, activity: &str) -> Result<()> {
        let output = Command::new("adb")
            .args(&["-s", device_id, "shell", "am", "start", "-n", 
                    &format!("{}/{}", package_name, activity)])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("Failed to start app: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }

        Ok(())
    }

    // ==================== Logcat / Device Logs ====================

    /// Get device logs
    pub async fn get_logs(&self, device_id: &str, filter: Option<&str>) -> Result<Vec<String>> {
        let mut cmd = Command::new("adb");
        cmd.args(&["-s", device_id, "logcat", "-d"]);

        if let Some(f) = filter {
            cmd.arg("-s").arg(f);
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get logs: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }

        let logs = String::from_utf8(output.stdout)?
            .lines()
            .map(|s| s.to_string())
            .collect();

        Ok(logs)
    }

    /// Clear device logs
    pub async fn clear_logs(&self, device_id: &str) -> Result<()> {
        let output = Command::new("adb")
            .args(&["-s", device_id, "logcat", "-c"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("Failed to clear logs: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }

        Ok(())
    }

    // ==================== Screenshot / Screen Recording ====================

    /// Take screenshot
    pub async fn take_screenshot(&self, device_id: &str, output_path: &Path) -> Result<()> {
        let temp_path = std::env::temp_dir().join("screenshot.png");

        let output = Command::new("adb")
            .args(&["-s", device_id, "exec-out", "screencap", "-p"])
            .stdout(std::process::Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("Failed to take screenshot: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }

        std::fs::write(&temp_path, output.stdout)?;
        std::fs::copy(&temp_path, output_path)?;
        let _ = std::fs::remove_file(temp_path);

        Ok(())
    }

    /// Start screen recording
    pub async fn start_recording(&self, device_id: &str, output_path: &Path) -> Result<()> {
        let output = Command::new("adb")
            .args(&["-s", device_id, "shell", "screenrecord", 
                    "/sdcard/record.mp4"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("Failed to start recording: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }

        Ok(())
    }

    /// Stop screen recording and pull file
    pub async fn stop_recording(&self, device_id: &str, output_path: &Path) -> Result<()> {
        // Stop recording (Ctrl+C)
        let output = Command::new("adb")
            .args(&["-s", device_id, "shell", "killall", "-s", "INT", "screenrecord"])
            .output()
            .await?;

        // Pull the file
        let output = Command::new("adb")
            .args(&["-s", device_id, "pull", "/sdcard/record.mp4", 
                    output_path.to_str().unwrap()])
            .output()
            .await?;

        // Clean up
        let _ = Command::new("adb")
            .args(&["-s", device_id, "shell", "rm", "/sdcard/record.mp4"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("Failed to stop recording: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }

        Ok(())
    }

    // ==================== Device Interactions ====================

    /// Send key event to device
    pub async fn send_key(&self, device_id: &str, key: &str) -> Result<()> {
        let output = Command::new("adb")
            .args(&["-s", device_id, "shell", "input", "keyevent", key])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("Failed to send key: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }

        Ok(())
    }

    /// Send touch event to device
    pub async fn send_touch(&self, device_id: &str, x: u32, y: u32, action: TouchAction) -> Result<()> {
        let cmd = match action {
            TouchAction::Tap => vec!["tap", &x.to_string(), &y.to_string()],
            TouchAction::Swipe(x2, y2) => vec!["swipe", &x.to_string(), &y.to_string(), 
                                                &x2.to_string(), &y2.to_string()],
        };

        let output = Command::new("adb")
            .args(&["-s", device_id, "shell", "input"])
            .args(cmd)
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("Failed to send touch: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }

        Ok(())
    }

    /// Send text to device
    pub async fn send_text(&self, device_id: &str, text: &str) -> Result<()> {
        let output = Command::new("adb")
            .args(&["-s", device_id, "shell", "input", "text", text])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("Failed to send text: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }

        Ok(())
    }

    // ==================== Private Helpers ====================

    /// Detect Android devices via ADB
    async fn detect_android_devices(&self) -> Result<Vec<MobileDevice>> {
        let mut devices = Vec::new();

        let output = Command::new("adb")
            .arg("devices")
            .output()
            .await?;

        let output_str = String::from_utf8(output.stdout)?;

        for line in output_str.lines().skip(1) {
            if line.contains("device") && !line.contains("offline") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if !parts.is_empty() {
                    let id = parts[0].to_string();
                    
                    // Get device details
                    let props = self.get_device_properties(&id).await?;
                    
                    devices.push(MobileDevice {
                        id,
                        name: props.get("ro.product.model").unwrap_or(&"Unknown".to_string()).clone(),
                        platform: MobilePlatform::Android,
                        model: props.get("ro.product.model").cloned(),
                        os_version: props.get("ro.build.version.release").unwrap_or(&"Unknown".to_string()).clone(),
                        is_emulator: id.starts_with("emulator-"),
                        is_connected: true,
                        api_level: props.get("ro.build.version.sdk").and_then(|s| s.parse().ok()),
                        screen_size: None,
                        abi: props.get("ro.product.cpu.abi").cloned(),
                    });
                }
            }
        }

        Ok(devices)
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
            let output = Command::new("adb")
                .args(&["-s", device_id, "shell", "getprop", prop])
                .output()
                .await?;

            let value = String::from_utf8(output.stdout)?.trim().to_string();
            if !value.is_empty() {
                props.insert(prop.to_string(), value);
            }
        }

        Ok(props)
    }

    /// Detect iOS devices (macOS only)
    #[cfg(target_os = "macos")]
    async fn detect_ios_devices(&self) -> Result<Vec<MobileDevice>> {
        let mut devices = Vec::new();

        let output = Command::new("idevice_id")
            .arg("-l")
            .output()
            .await?;

        let output_str = String::from_utf8(output.stdout)?;

        for line in output_str.lines() {
            if !line.is_empty() {
                let id = line.trim().to_string();
                
                // Get device name
                let name_output = Command::new("idevicename")
                    .arg("-u")
                    .arg(&id)
                    .output()
                    .await?;
                
                let name = String::from_utf8(name_output.stdout)?.trim().to_string();

                devices.push(MobileDevice {
                    id,
                    name: if name.is_empty() { "iPhone".to_string() } else { name },
                    platform: MobilePlatform::iOS,
                    model: None,
                    os_version: "Unknown".to_string(),
                    is_emulator: false,
                    is_connected: true,
                    api_level: None,
                    screen_size: None,
                    abi: None,
                });
            }
        }

        Ok(devices)
    }

    /// Create Android emulator
    async fn create_android_emulator(&self, config: EmulatorConfig) -> Result<String> {
        let output = Command::new("avdmanager")
            .args(&["create", "avd", "-n", &config.name, "-k", &config.system_image])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("Failed to create emulator: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }

        Ok(config.name)
    }

    /// Create iOS simulator (macOS only)
    #[cfg(target_os = "macos")]
    async fn create_ios_simulator(&self, config: EmulatorConfig) -> Result<String> {
        let output = Command::new("xcrun")
            .args(&["simctl", "create", &config.name, &config.device_type, &config.system_image])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("Failed to create simulator: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }

        Ok(config.name)
    }
}

/// Touch action types
#[derive(Debug, Clone)]
pub enum TouchAction {
    Tap,
    Swipe(u32, u32), // x2, y2
}

/// Emulator information
#[derive(Debug, Clone)]
pub struct EmulatorInfo {
    pub id: String,
    pub name: String,
    pub platform: MobilePlatform,
    pub vnc_port: Option<u16>,
    pub adb_port: Option<u16>,
}

/// Build status
#[derive(Debug, Clone)]
pub struct BuildStatus {
    pub id: String,
    pub is_running: bool,
    pub elapsed: std::time::Duration,
    pub output_path: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // Mock extension handle for testing
    fn mock_handle() -> ExtensionHandle {
        // This would need a real runtime - simplified for test
        unimplemented!("Mock handle for testing")
    }

    #[tokio::test]
    async fn test_device_detection() {
        let api = MobileAPI::new(mock_handle());
        let devices = api.refresh_devices().await;
        // May or may not have devices, just ensure it doesn't crash
        assert!(devices.is_ok());
    }

    #[tokio::test]
    async fn test_emulator_management() {
        let api = MobileAPI::new(mock_handle());
        
        let config = EmulatorConfig {
            name: "test_avd".to_string(),
            platform: MobilePlatform::Android,
            device_type: "pixel_4".to_string(),
            system_image: "system-images;android-30;google_apis;x86".to_string(),
            ram_mb: Some(2048),
            disk_mb: Some(2048),
            screen_resolution: Some((1080, 1920)),
            screen_density: Some(420),
            show_skins: false,
            headless: true,
        };

        // Creation might fail if AVD not available, but shouldn't crash
                let _ = api.create_emulator(config).await;
    }

    #[tokio::test]
    async fn test_build_management() {
        let api = MobileAPI::new(mock_handle());
        let temp_dir = tempdir().unwrap();

        let config = BuildConfig {
            platform: MobilePlatform::Android,
            configuration: BuildConfiguration::Debug,
            output_path: None,
            signing_config: None,
            extra_args: vec![],
        };

        let result = api.build_app(temp_dir.path(), config).await;
        // Will fail because no gradlew in temp dir, but shouldn't crash
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_device_operations() {
        let api = MobileAPI::new(mock_handle());
        
        // These operations will fail without real devices, but shouldn't crash
        let _ = api.connect_device("emulator-5554").await;
        let _ = api.disconnect_device("emulator-5554").await;
        let _ = api.send_key("emulator-5554", "KEYCODE_HOME").await;
        let _ = api.send_touch("emulator-5554", 100, 100, TouchAction::Tap).await;
        let _ = api.send_text("emulator-5554", "Hello").await;
        let _ = api.get_logs("emulator-5554", None).await;
        let _ = api.clear_logs("emulator-5554").await;
    }

    #[tokio::test]
    async fn test_screenshot() {
        let api = MobileAPI::new(mock_handle());
        let temp_dir = tempdir().unwrap();
        let screenshot_path = temp_dir.path().join("screenshot.png");

        let result = api.take_screenshot("emulator-5554", &screenshot_path).await;
        // Will fail without device, but shouldn't crash
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_install_operations() {
        let api = MobileAPI::new(mock_handle());
        let temp_dir = tempdir().unwrap();
        let apk_path = temp_dir.path().join("app.apk");

        let result = api.install_app("emulator-5554", &apk_path).await;
        assert!(result.is_err());

        let result = api.uninstall_app("emulator-5554", "com.example.app").await;
        assert!(result.is_err());

        let result = api.run_app("emulator-5554", "com.example.app", ".MainActivity").await;
        assert!(result.is_err());
    }
}

/// Mobile API module exports
pub mod prelude {
    pub use super::{
        MobileAPI,
        MobilePlatform,
        MobileDevice,
        EmulatorConfig,
        BuildConfig,
        BuildConfiguration,
        SigningConfig,
        TouchAction,
        EmulatorInfo,
        BuildStatus,
    };
}

/// Re-export main types
pub use prelude::*;

/// Check if Android SDK is available
pub fn is_android_sdk_available() -> bool {
    std::env::var("ANDROID_HOME").is_ok() || std::env::var("ANDROID_SDK_ROOT").is_ok()
}

/// Check if iOS tools are available (macOS only)
#[cfg(target_os = "macos")]
pub fn is_ios_tools_available() -> bool {
    which::which("xcodebuild").is_ok() && which::which("simctl").is_ok()
}

/// Get Android SDK path
pub fn android_sdk_path() -> Option<PathBuf> {
    std::env::var("ANDROID_HOME")
        .or_else(|_| std::env::var("ANDROID_SDK_ROOT"))
        .ok()
        .map(PathBuf::from)
}

/// Get Android AVD path
pub fn android_avd_path() -> Option<PathBuf> {
    android_sdk_path().map(|p| p.join("avd"))
}

/// Default Android emulator options
pub fn default_android_emulator(name: &str) -> EmulatorConfig {
    EmulatorConfig {
        name: name.to_string(),
        platform: MobilePlatform::Android,
        device_type: "pixel_4".to_string(),
        system_image: "system-images;android-30;google_apis;x86".to_string(),
        ram_mb: Some(2048),
        disk_mb: Some(2048),
        screen_resolution: Some((1080, 1920)),
        screen_density: Some(420),
        show_skins: false,
        headless: false,
    }
}

/// Default iOS simulator options (macOS only)
#[cfg(target_os = "macos")]
pub fn default_ios_simulator(name: &str) -> EmulatorConfig {
    EmulatorConfig {
        name: name.to_string(),
        platform: MobilePlatform::iOS,
        device_type: "iPhone-14".to_string(),
        system_image: "iOS-16-4".to_string(),
        ram_mb: None,
        disk_mb: None,
        screen_resolution: None,
        screen_density: None,
        show_skins: true,
        headless: false,
    }
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");