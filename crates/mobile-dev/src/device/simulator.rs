//! iOS simulator management (macOS only)

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, anyhow};
use tokio::process::Command as TokioCommand;
use tracing::{info, warn, debug};

#[cfg(target_os = "macos")]
use super::DeviceInfo;

#[cfg(target_os = "macos")]
/// iOS simulator manager
#[derive(Clone)]
pub struct SimulatorManager {
    simctl_path: PathBuf,
}

#[cfg(target_os = "macos")]
/// Simulator configuration
#[derive(Debug, Clone)]
pub struct SimulatorConfig {
    pub name: String,
    pub device_type: String,
    pub runtime: String,
}

#[cfg(target_os = "macos")]
impl SimulatorManager {
    /// Create new simulator manager
    pub fn new() -> Result<Self> {
        // Find xcrun path
        let output = Command::new("xcrun")
            .arg("--find")
            .arg("simctl")
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("simctl not found"));
        }

        let path = String::from_utf8(output.stdout)?
            .trim()
            .to_string();

        Ok(Self {
            simctl_path: PathBuf::from(path),
        })
    }

    /// List available simulators
    pub async fn list_simulators(&self) -> Result<Vec<SimulatorInfo>> {
        let output = TokioCommand::new(&self.simctl_path)
            .args(&["list", "devices", "--json"])
            .output()
            .await?;

        let output_str = String::from_utf8(output.stdout)?;
        let json: serde_json::Value = serde_json::from_str(&output_str)?;

        let mut simulators = Vec::new();

        if let Some(devices) = json["devices"].as_object() {
            for (runtime, device_list) in devices {
                if let Some(devices) = device_list.as_array() {
                    for device in devices {
                        if let (Some(name), Some(state), Some(udid)) = (
                            device["name"].as_str(),
                            device["state"].as_str(),
                            device["udid"].as_str()
                        ) {
                            simulators.push(SimulatorInfo {
                                name: name.to_string(),
                                udid: udid.to_string(),
                                runtime: runtime.to_string(),
                                state: state.to_string(),
                                is_available: state == "Booted",
                            });
                        }
                    }
                }
            }
        }

        Ok(simulators)
    }

    /// Create new simulator
    pub async fn create_simulator(&self, config: SimulatorConfig) -> Result<String> {
        let output = TokioCommand::new(&self.simctl_path)
            .args(&["create", &config.name, &config.device_type, &config.runtime])
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to create simulator: {}", error));
        }

        let udid = String::from_utf8(output.stdout)?.trim().to_string();
        Ok(udid)
    }

    /// Start simulator
    pub async fn start_simulator(&self, udid: &str) -> Result<()> {
        let output = TokioCommand::new(&self.simctl_path)
            .args(&["boot", udid])
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to boot simulator: {}", error));
        }

        // Open Simulator.app if not already open
        let _ = Command::new("open")
            .arg("-a")
            .arg("Simulator")
            .spawn();

        Ok(())
    }

    /// Stop simulator
    pub async fn stop_simulator(&self, udid: &str) -> Result<()> {
        let output = TokioCommand::new(&self.simctl_path)
            .args(&["shutdown", udid])
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to shutdown simulator: {}", error));
        }

        Ok(())
    }

    /// Install app on simulator
    pub async fn install_app(&self, udid: &str, app_path: &Path) -> Result<()> {
        let output = TokioCommand::new(&self.simctl_path)
            .args(&["install", udid, app_path.to_str().unwrap()])
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to install app: {}", error));
        }

        Ok(())
    }

    /// Launch app on simulator
    pub async fn launch_app(&self, udid: &str, bundle_id: &str) -> Result<()> {
        let output = TokioCommand::new(&self.simctl_path)
            .args(&["launch", udid, bundle_id])
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to launch app: {}", error));
        }

        Ok(())
    }

    /// Get simulator info as DeviceInfo
    pub async fn get_simulator_info(&self, udid: &str) -> Result<DeviceInfo> {
        let simulators = self.list_simulators().await?;
        
        for sim in simulators {
            if sim.udid == udid {
                return Ok(DeviceInfo {
                    id: sim.udid,
                    name: sim.name,
                    device_type: crate::device::DeviceType::Simulator,
                    platform: "ios".to_string(),
                    os_version: sim.runtime,
                    api_level: None,
                    is_connected: sim.is_available,
                    is_authorized: true,
                    screen_size: None,
                    abi: None,
                });
            }
        }

        Err(anyhow!("Simulator not found: {}", udid))
    }
}

#[cfg(target_os = "macos")]
/// Simulator information
#[derive(Debug, Clone)]
pub struct SimulatorInfo {
    pub name: String,
    pub udid: String,
    pub runtime: String,
    pub state: String,
    pub is_available: bool,
}

#[cfg(not(target_os = "macos"))]
/// Simulator configuration (non-macOS stub)
#[derive(Debug, Clone)]
pub struct SimulatorConfig {
    pub name: String,
    pub device_type: String,
    pub runtime: String,
}

#[cfg(not(target_os = "macos"))]
/// Simulator information (non-macOS stub)
#[derive(Debug, Clone)]
pub struct SimulatorInfo {
    pub name: String,
    pub udid: String,
    pub runtime: String,
    pub state: String,
    pub is_available: bool,
}

#[cfg(not(target_os = "macos"))]
#[derive(Clone)]
pub struct SimulatorManager;

#[cfg(not(target_os = "macos"))]
impl SimulatorManager {
    pub fn new() -> Result<Self> { Ok(Self) }
    pub async fn list_simulators(&self) -> Result<Vec<SimulatorInfo>> { Ok(vec![]) }
    pub async fn create_simulator(&self, _config: SimulatorConfig) -> Result<String> { 
        Err(anyhow!("iOS simulators require macOS")) 
    }
    pub async fn start_simulator(&self, _udid: &str) -> Result<()> { 
        Err(anyhow!("iOS simulators require macOS")) 
    }
    pub async fn stop_simulator(&self, _udid: &str) -> Result<()> { 
        Err(anyhow!("iOS simulators require macOS")) 
    }
    pub async fn install_app(&self, _udid: &str, _app_path: &Path) -> Result<()> { 
        Err(anyhow!("iOS simulators require macOS")) 
    }
    pub async fn launch_app(&self, _udid: &str, _bundle_id: &str) -> Result<()> { 
        Err(anyhow!("iOS simulators require macOS")) 
    }
}