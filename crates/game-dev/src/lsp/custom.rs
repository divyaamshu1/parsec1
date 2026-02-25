//! Custom language server support

use super::ServerConfig;
use std::collections::HashMap;

pub fn create_custom_config(
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
) -> ServerConfig {
    ServerConfig { command, args, env }
}