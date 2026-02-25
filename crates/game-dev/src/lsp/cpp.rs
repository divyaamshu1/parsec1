//! C++ language server (clangd)

use super::ServerConfig;
use std::collections::HashMap;

pub fn get_cpp_config() -> ServerConfig {
    ServerConfig {
        command: "clangd".to_string(),
        args: vec![
            "--background-index".to_string(),
            "--clang-tidy".to_string(),
            "--header-insertion=iwyu".to_string(),
            "--completion-style=detailed".to_string(),
        ],
        env: HashMap::new(),
    }
}