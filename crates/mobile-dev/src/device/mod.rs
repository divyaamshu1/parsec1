//! Device management for mobile development

mod android;
mod ios;
mod emulator;
mod simulator;

pub use android::*;
pub use ios::*;
pub use emulator::*;
pub use simulator::*;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::sync::{RwLock, Mutex};
use tracing::{info, warn, debug};

/// Device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Physical,
    Emulator,
    Simulator,
}

/// Device information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub platform: String,
    pub os_version: String,
    pub api_level: Option<u32>,
    pub is_connected: bool,
    pub is_authorized: bool,
    pub screen_size: Option<(u32, u32)>,
    pub abi: Option<String>,
}

/// Device manager
pub struct DeviceManager {
    devices: Arc<RwLock<HashMap<String, DeviceInfo>>>,
    android: Arc<android::AndroidDeviceManager>,
    #[cfg(target_os = "macos")]
    ios: Arc<ios::IOSDeviceManager>,
    refresh_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl DeviceManager {
    /// Create new device manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            android: Arc::new(android::AndroidDeviceManager::new()?),
            #[cfg(target_os = "macos")]
            ios: Arc::new(ios::IOSDeviceManager::new()?),
            refresh_task: Arc::new(Mutex::new(None)),
        })
    }

    /// Refresh device list
    pub async fn refresh(&self) -> Result<()> {
        let mut devices = self.devices.write().await;
        devices.clear();

        // Detect Android devices
        if let Ok(android_devices) = self.android.detect_devices().await {
            for device in android_devices {
                devices.insert(device.id.clone(), device);
            }
        }

        // Detect iOS devices (macOS only)
        #[cfg(target_os = "macos")]
        if let Ok(ios_devices) = self.ios.detect_devices().await {
            for device in ios_devices {
                devices.insert(device.id.clone(), device);
            }
        }

        info!("Found {} connected devices", devices.len());
        Ok(())
    }

    /// Start auto-refresh task
    pub async fn start_auto_refresh(&self, interval_secs: u64) {
        let manager = self.clone();
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));

        let task = tokio::spawn(async move {
            loop {
                interval.tick().await;
                let _ = manager.refresh().await;
            }
        });

        *self.refresh_task.lock().await = Some(task);
    }

    /// List all devices
    pub async fn list_devices(&self) -> Vec<DeviceInfo> {
        self.devices.read().await.values().cloned().collect()
    }

    /// Get device by ID
    pub async fn get_device(&self, id: &str) -> Option<DeviceInfo> {
        self.devices.read().await.get(id).cloned()
    }

    /// Install app on device
    pub async fn install_app(&self, device_id: &str, apk_path: &Path) -> Result<()> {
        let devices = self.devices.read().await;
        let device = devices.get(device_id)
            .ok_or_else(|| anyhow!("Device not found: {}", device_id))?;

        match device.platform.as_str() {
            "android" => self.android.install_app(device_id, apk_path).await,
            "ios" => {
                #[cfg(target_os = "macos")]
                return self.ios.install_app(device_id, apk_path).await;
                #[cfg(not(target_os = "macos"))]
                Err(anyhow!("iOS development requires macOS"))
            }
            _ => Err(anyhow!("Unsupported platform: {}", device.platform)),
        }
    }

    /// Run app on device
    pub async fn run_app(&self, device_id: &str, package_name: &str) -> Result<()> {
        let devices = self.devices.read().await;
        let device = devices.get(device_id)
            .ok_or_else(|| anyhow!("Device not found: {}", device_id))?;

        match device.platform.as_str() {
            "android" => self.android.run_app(device_id, package_name).await,
            "ios" => {
                #[cfg(target_os = "macos")]
                return self.ios.run_app(device_id, package_name).await;
                #[cfg(not(target_os = "macos"))]
                Err(anyhow!("iOS development requires macOS"))
            }
            _ => Err(anyhow!("Unsupported platform: {}", device.platform)),
        }
    }

    /// Take screenshot from device
    pub async fn take_screenshot(&self, device_id: &str, output_path: &Path) -> Result<()> {
        let devices = self.devices.read().await;
        let device = devices.get(device_id)
            .ok_or_else(|| anyhow!("Device not found: {}", device_id))?;

        match device.platform.as_str() {
            "android" => self.android.take_screenshot(device_id, output_path).await,
            "ios" => {
                #[cfg(target_os = "macos")]
                return self.ios.take_screenshot(device_id, output_path).await;
                #[cfg(not(target_os = "macos"))]
                Err(anyhow!("iOS development requires macOS"))
            }
            _ => Err(anyhow!("Unsupported platform: {}", device.platform)),
        }
    }

    /// Get device logs
    pub async fn get_logs(&self, device_id: &str) -> Result<Vec<String>> {
        let devices = self.devices.read().await;
        let device = devices.get(device_id)
            .ok_or_else(|| anyhow!("Device not found: {}", device_id))?;

        match device.platform.as_str() {
            "android" => self.android.get_logs(device_id).await,
            "ios" => {
                #[cfg(target_os = "macos")]
                return self.ios.get_logs(device_id).await;
                #[cfg(not(target_os = "macos"))]
                Err(anyhow!("iOS development requires macOS"))
            }
            _ => Err(anyhow!("Unsupported platform: {}", device.platform)),
        }
    }
}

impl Clone for DeviceManager {
    fn clone(&self) -> Self {
        Self {
            devices: self.devices.clone(),
            android: self.android.clone(),
            #[cfg(target_os = "macos")]
            ios: self.ios.clone(),
            refresh_task: self.refresh_task.clone(),
        }
    }
}