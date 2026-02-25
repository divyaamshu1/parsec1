//! Universal debugger using Debug Adapter Protocol (DAP)

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tracing::{info, warn, debug};

/// Debugger using DAP
pub struct Debugger {
    active_sessions: Arc<tokio::sync::Mutex<HashMap<String, DebugSession>>>,
}

/// Debug session
#[derive(Debug)]
pub struct DebugSession {
    pub id: String,
    pub process: Option<Child>,
    pub stdin: Option<ChildStdin>,
    pub stdout: Option<ChildStdout>,
    pub breakpoints: Vec<Breakpoint>,
    pub state: DebugState,
}

/// Debug state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugState {
    Initializing,
    Running,
    Paused,
    Stopped,
    Terminated,
}

/// Breakpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub id: usize,
    pub file: PathBuf,
    pub line: usize,
    pub condition: Option<String>,
    pub enabled: bool,
}

/// DAP Protocol messages
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DAPMessage {
    #[serde(rename = "request")]
    Request {
        command: String,
        arguments: Option<serde_json::Value>,
    },
    #[serde(rename = "response")]
    Response {
        request_seq: usize,
        success: bool,
        command: String,
        body: Option<serde_json::Value>,
    },
    #[serde(rename = "event")]
    Event {
        event: String,
        body: Option<serde_json::Value>,
    },
}

impl Debugger {
    /// Create new debugger
    pub fn new() -> Result<Self> {
        Ok(Self {
            active_sessions: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        })
    }

    /// Start a debug session
    pub async fn start_session(
        &self,
        project: &crate::project::Project,
        engine: &dyn crate::engine::GameEngine,
        config: crate::DebugConfig,
    ) -> Result<DebugSession> {
        let session_id = uuid::Uuid::new_v4().to_string();

        // Find DAP adapter for this engine
        let adapter = self.find_dap_adapter(engine).await?;

        // Start debug adapter process
        let mut cmd = Command::new(&adapter.command);
        cmd.args(&adapter.args);
        cmd.envs(&adapter.env);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        let stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to get stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to get stdout"))?;

        // Initialize DAP session
        self.initialize_dap(&stdin, &stdout, &config).await?;

        let session = DebugSession {
            id: session_id.clone(),
            process: Some(child),
            stdin: Some(stdin),
            stdout: Some(stdout),
            breakpoints: config.breakpoints.clone(),
            state: DebugState::Initializing,
        };

        self.active_sessions.lock().await.insert(session_id, session);

        Ok(session)
    }

    /// Find DAP adapter for engine
    async fn find_dap_adapter(&self, engine: &dyn crate::engine::GameEngine) -> Result<DAPAdapter> {
        match engine.engine_type() {
            crate::engine::EngineType::Unity => Ok(DAPAdapter {
                command: "dotnet".to_string(),
                args: vec!["/path/to/unity-debugger.dll".to_string()],
                env: HashMap::new(),
            }),
            crate::engine::EngineType::Unreal => Ok(DAPAdapter {
                command: "unreal-debugger".to_string(),
                args: vec![],
                env: HashMap::new(),
            }),
            crate::engine::EngineType::Godot => Ok(DAPAdapter {
                command: "godot".to_string(),
                args: vec!["--debug".to_string()],
                env: HashMap::new(),
            }),
            crate::engine::EngineType::Custom(name) => {
                // Look for adapter in SDK
                Err(anyhow!("No DAP adapter found for custom engine: {}", name))
            }
        }
    }

    /// Initialize DAP session
    async fn initialize_dap(
        &self,
        stdin: &ChildStdin,
        stdout: &ChildStdout,
        config: &crate::DebugConfig,
    ) -> Result<()> {
        let mut stdin = stdin;
        let mut reader = BufReader::new(stdout);

        // Send initialize request
        let init_request = serde_json::json!({
            "type": "request",
            "command": "initialize",
            "arguments": {
                "clientID": "parsec-ide",
                "clientName": "Parsec IDE",
                "adapterID": "game-debugger",
                "pathFormat": "path",
                "linesStartAt1": true,
                "columnsStartAt1": true,
                "supportsVariableType": true,
                "supportsVariablePaging": true,
                "supportsRunInTerminalRequest": true,
                "locale": "en-US"
            }
        });

        let init_json = serde_json::to_string(&init_request)?;
        stdin.write_all(format!("Content-Length: {}\r\n\r\n{}", init_json.len(), init_json).as_bytes()).await?;
        stdin.flush().await?;

        // Read response
        let mut response = String::new();
        reader.read_line(&mut response).await?;

        // Send configurationDone request
        let config_done = serde_json::json!({
            "type": "request",
            "command": "configurationDone"
        });

        let config_json = serde_json::to_string(&config_done)?;
        stdin.write_all(format!("Content-Length: {}\r\n\r\n{}", config_json.len(), config_json).as_bytes()).await?;
        stdin.flush().await?;

        // Set breakpoints
        for bp in &config.breakpoints {
            self.set_breakpoint(stdin, bp).await?;
        }

        // Launch request
        let launch_request = serde_json::json!({
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": config.executable,
                "args": config.args,
                "cwd": config.cwd,
                "env": config.env,
                "stopAtEntry": true,
                "console": "integratedTerminal"
            }
        });

        let launch_json = serde_json::to_string(&launch_request)?;
        stdin.write_all(format!("Content-Length: {}\r\n\r\n{}", launch_json.len(), launch_json).as_bytes()).await?;
        stdin.flush().await?;

        Ok(())
    }

    /// Set breakpoint
    async fn set_breakpoint(&self, stdin: &ChildStdin, bp: &Breakpoint) -> Result<()> {
        let set_bp_request = serde_json::json!({
            "type": "request",
            "command": "setBreakpoints",
            "arguments": {
                "source": {
                    "path": bp.file,
                },
                "breakpoints": [{
                    "line": bp.line,
                    "condition": bp.condition,
                }],
                "lines": [bp.line],
                "sourceModified": false
            }
        });

        let set_bp_json = serde_json::to_string(&set_bp_request)?;
        stdin.write_all(format!("Content-Length: {}\r\n\r\n{}", set_bp_json.len(), set_bp_json).as_bytes()).await?;
        stdin.flush().await?;

        Ok(())
    }

    /// Continue execution
    pub async fn continue_execution(&self, session_id: &str) -> Result<()> {
        let sessions = self.active_sessions.lock().await;
        if let Some(session) = sessions.get(session_id) {
            if let Some(stdin) = &session.stdin {
                let continue_request = serde_json::json!({
                    "type": "request",
                    "command": "continue"
                });

                let continue_json = serde_json::to_string(&continue_request)?;
                let mut stdin = stdin;
                stdin.write_all(format!("Content-Length: {}\r\n\r\n{}", continue_json.len(), continue_json).as_bytes()).await?;
                stdin.flush().await?;
            }
        }
        Ok(())
    }

    /// Pause execution
    pub async fn pause(&self, session_id: &str) -> Result<()> {
        let sessions = self.active_sessions.lock().await;
        if let Some(session) = sessions.get(session_id) {
            if let Some(stdin) = &session.stdin {
                let pause_request = serde_json::json!({
                    "type": "request",
                    "command": "pause"
                });

                let pause_json = serde_json::to_string(&pause_request)?;
                let mut stdin = stdin;
                stdin.write_all(format!("Content-Length: {}\r\n\r\n{}", pause_json.len(), pause_json).as_bytes()).await?;
                stdin.flush().await?;
            }
        }
        Ok(())
    }

    /// Step over
    pub async fn step_over(&self, session_id: &str) -> Result<()> {
        let sessions = self.active_sessions.lock().await;
        if let Some(session) = sessions.get(session_id) {
            if let Some(stdin) = &session.stdin {
                let step_request = serde_json::json!({
                    "type": "request",
                    "command": "next"
                });

                let step_json = serde_json::to_string(&step_request)?;
                let mut stdin = stdin;
                stdin.write_all(format!("Content-Length: {}\r\n\r\n{}", step_json.len(), step_json).as_bytes()).await?;
                stdin.flush().await?;
            }
        }
        Ok(())
    }

    /// Step into
    pub async fn step_into(&self, session_id: &str) -> Result<()> {
        let sessions = self.active_sessions.lock().await;
        if let Some(session) = sessions.get(session_id) {
            if let Some(stdin) = &session.stdin {
                let step_request = serde_json::json!({
                    "type": "request",
                    "command": "stepIn"
                });

                let step_json = serde_json::to_string(&step_request)?;
                let mut stdin = stdin;
                stdin.write_all(format!("Content-Length: {}\r\n\r\n{}", step_json.len(), step_json).as_bytes()).await?;
                stdin.flush().await?;
            }
        }
        Ok(())
    }

    /// Step out
    pub async fn step_out(&self, session_id: &str) -> Result<()> {
        let sessions = self.active_sessions.lock().await;
        if let Some(session) = sessions.get(session_id) {
            if let Some(stdin) = &session.stdin {
                let step_request = serde_json::json!({
                    "type": "request",
                    "command": "stepOut"
                });

                let step_json = serde_json::to_string(&step_request)?;
                let mut stdin = stdin;
                stdin.write_all(format!("Content-Length: {}\r\n\r\n{}", step_json.len(), step_json).as_bytes()).await?;
                stdin.flush().await?;
            }
        }
        Ok(())
    }

    /// Stop debugging
    pub async fn stop(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.active_sessions.lock().await;
        if let Some(mut session) = sessions.remove(session_id) {
            if let Some(mut process) = session.process.take() {
                process.kill().await?;
            }
        }
        Ok(())
    }
}

/// DAP adapter configuration
#[derive(Debug, Clone)]
pub struct DAPAdapter {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}