//! Common build utilities

use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow};

/// Clean build directory
pub async fn clean_build_dir(path: &Path) -> Result<()> {
    if path.exists() {
        tokio::fs::remove_dir_all(path).await?;
    }
    tokio::fs::create_dir_all(path).await?;
    Ok(())
}

/// Parse build output for errors
pub fn parse_errors(output: &str) -> Vec<String> {
    let mut errors = Vec::new();

    for line in output.lines() {
        if line.contains("error:") || line.contains("FAILURE:") {
            errors.push(line.to_string());
        }
    }

    errors
}

/// Get timestamp for build
pub fn get_build_timestamp() -> String {
    chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
}