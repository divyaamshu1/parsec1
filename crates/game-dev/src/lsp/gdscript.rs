//! GDScript language server (Godot)

use super::ServerConfig;
use std::collections::HashMap;

pub fn get_gdscript_config() -> ServerConfig {
    ServerConfig {
        command: "godot".to_string(),
        args: vec!["--language-server".to_string()],
        env: HashMap::new(),
    }
}