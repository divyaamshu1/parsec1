//! C# language server (OmniSharp)

use super::ServerConfig;
use std::collections::HashMap;

pub fn get_csharp_config() -> ServerConfig {
    ServerConfig {
        command: "dotnet".to_string(),
        args: vec![
            "/usr/local/bin/omnisharp".to_string(),
            "-lsp".to_string(),
        ],
        env: HashMap::new(),
    }
}