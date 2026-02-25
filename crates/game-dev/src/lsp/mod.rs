//! Language Server Protocol integration for game development

mod csharp;
mod cpp;
mod gdscript;
mod custom;

pub use csharp::*;
pub use cpp::*;
pub use gdscript::*;
pub use custom::*;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use lsp_server::{Connection, Message};
use lsp_types::*;
use tokio::process::{Command, Child};
use tokio::sync::Mutex;
use tracing::{info, warn, debug};

/// Language server instance
#[derive(Debug)]
pub struct LanguageServerInstance {
    pub language: String,
    pub process: Child,
    pub connection: Connection,
}

/// Language server manager
pub struct LanguageServerManager {
    servers: Arc<Mutex<HashMap<String, LanguageServerInstance>>>,
    config: LSPConfig,
}

/// LSP configuration
#[derive(Debug, Clone)]
pub struct LSPConfig {
    pub enable_all: bool,
    pub csharp_enabled: bool,
    pub cpp_enabled: bool,
    pub gdscript_enabled: bool,
    pub custom_servers: HashMap<String, ServerConfig>,
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

impl Default for LSPConfig {
    fn default() -> Self {
        Self {
            enable_all: true,
            csharp_enabled: true,
            cpp_enabled: true,
            gdscript_enabled: true,
            custom_servers: HashMap::new(),
        }
    }
}

impl LanguageServerManager {
    /// Create new LSP manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            servers: Arc::new(Mutex::new(HashMap::new())),
            config: LSPConfig::default(),
        })
    }

    /// Initialize LSP for a project
    pub async fn init_for_project(&self, project: &crate::Project) -> Result<()> {
        let root_uri = Url::from_file_path(project.path())
            .map_err(|_| anyhow!("Invalid project path"))?;

        // Start C# server for Unity projects
        if project.engine_type() == crate::engine::EngineType::Unity {
            self.start_server("csharp", root_uri.clone()).await?;
        }

        // Start C++ server for Unreal projects
        if project.engine_type() == crate::engine::EngineType::Unreal {
            self.start_server("cpp", root_uri.clone()).await?;
        }

        // Start GDScript server for Godot projects
        if project.engine_type() == crate::engine::EngineType::Godot {
            self.start_server("gdscript", root_uri.clone()).await?;
        }

        Ok(())
    }

    /// Start a language server
    pub async fn start_server(&self, language: &str, root_uri: Url) -> Result<()> {
        let config = self.get_server_config(language)?;

        // Create LSP connection
        let (connection, io_threads) = Connection::stdio();

        // Spawn server process
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);
        cmd.envs(&config.env);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let process = cmd.spawn()?;

        // Initialize server
        let initialize_params = serde_json::to_value(InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(root_uri),
            capabilities: ClientCapabilities::default(),
            workspace_folders: None,
            initialization_options: None,
            ..Default::default()
        })?;

        let req_id = 1;
        connection.initialize(req_id, initialize_params)?;

        // Wait for server to be ready
        let response = connection.initialize_finish(req_id)?;
        if response.error.is_some() {
            return Err(anyhow!("Failed to initialize LSP server"));
        }

        // Store server instance
        let instance = LanguageServerInstance {
            language: language.to_string(),
            process,
            connection,
        };

        self.servers.lock().await.insert(language.to_string(), instance);

        info!("Started LSP server for {}", language);

        Ok(())
    }

    /// Get server configuration
    fn get_server_config(&self, language: &str) -> Result<ServerConfig> {
        match language {
            "csharp" if self.config.csharp_enabled => Ok(ServerConfig {
                command: "dotnet".to_string(),
                args: vec!["/usr/local/bin/omnisharp".to_string()],
                env: HashMap::new(),
            }),
            "cpp" if self.config.cpp_enabled => Ok(ServerConfig {
                command: "clangd".to_string(),
                args: vec![],
                env: HashMap::new(),
            }),
            "gdscript" if self.config.gdscript_enabled => Ok(ServerConfig {
                command: "godot".to_string(),
                args: vec!["--language-server".to_string()],
                env: HashMap::new(),
            }),
            _ => {
                if let Some(config) = self.config.custom_servers.get(language) {
                    Ok(config.clone())
                } else {
                    Err(anyhow!("No server configuration for language: {}", language))
                }
            }
        }
    }

    /// Send request to language server
    pub async fn request<R>(&self, language: &str, method: &str, params: R) -> Result<serde_json::Value>
    where
        R: serde::Serialize,
    {
        let mut servers = self.servers.lock().await;
        let server = servers.get_mut(language)
            .ok_or_else(|| anyhow!("Server not running for language: {}", language))?;

        let request = server.connection.request(method, params)?;
        let response = server.connection.recv()?;

        match response {
            Message::Response(resp) => Ok(resp.result.unwrap_or(serde_json::Value::Null)),
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// Get completions at position
    pub async fn get_completions(
        &self,
        language: &str,
        uri: &Url,
        line: usize,
        character: usize,
    ) -> Result<Vec<CompletionItem>> {
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(line as u32, character as u32),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: None,
        };

        let result = self.request(language, "textDocument/completion", params).await?;
        let completions: Vec<CompletionItem> = serde_json::from_value(result)?;

        Ok(completions)
    }

    /// Get hover info at position
    pub async fn get_hover(
        &self,
        language: &str,
        uri: &Url,
        line: usize,
        character: usize,
    ) -> Result<Option<Hover>> {
        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(line as u32, character as u32),
            },
            work_done_progress_params: Default::default(),
        };

        let result = self.request(language, "textDocument/hover", params).await?;
        if result.is_null() {
            Ok(None)
        } else {
            let hover: Hover = serde_json::from_value(result)?;
            Ok(Some(hover))
        }
    }

    /// Goto definition
    pub async fn goto_definition(
        &self,
        language: &str,
        uri: &Url,
        line: usize,
        character: usize,
    ) -> Result<Vec<Location>> {
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(line as u32, character as u32),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self.request(language, "textDocument/definition", params).await?;
        let locations: Vec<Location> = serde_json::from_value(result)?;

        Ok(locations)
    }

    /// Find references
    pub async fn find_references(
        &self,
        language: &str,
        uri: &Url,
        line: usize,
        character: usize,
    ) -> Result<Vec<Location>> {
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(line as u32, character as u32),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };

        let result = self.request(language, "textDocument/references", params).await?;
        let locations: Vec<Location> = serde_json::from_value(result)?;

        Ok(locations)
    }

    /// Format document
    pub async fn format_document(
        &self,
        language: &str,
        uri: &Url,
        options: FormattingOptions,
    ) -> Result<Vec<TextEdit>> {
        let params = DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            options,
            work_done_progress_params: Default::default(),
        };

        let result = self.request(language, "textDocument/formatting", params).await?;
        let edits: Vec<TextEdit> = serde_json::from_value(result)?;

        Ok(edits)
    }

    /// Shutdown all servers
    pub async fn shutdown_all(&self) -> Result<()> {
        let mut servers = self.servers.lock().await;
        
        for (_, mut server) in servers.drain() {
            // Send shutdown request
            let _ = server.connection.request::<serde_json::Value>("shutdown", None);
            let _ = server.process.kill().await;
        }

        Ok(())
    }
}