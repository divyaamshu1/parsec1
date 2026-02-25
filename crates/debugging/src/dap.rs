//! Debug Adapter Protocol (DAP) implementation

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use tokio::process::{Command as TokioCommand, Child};
use tokio::sync::Mutex;
use tracing::{info, warn, debug};

use crate::{DebugAdapter, AdapterConfig, Breakpoint, StackFrame, VariableScope, Variable};

/// DAP client for communicating with debug adapters
pub struct DAPClient {
    process: Arc<Mutex<Option<Child>>>,
    next_seq: Arc<Mutex<usize>>,
    pending_requests: Arc<Mutex<HashMap<usize, tokio::sync::oneshot::Sender<DAPResponse>>>>,
}

/// DAP message
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DAPMessage {
    #[serde(rename = "request")]
    Request {
        seq: usize,
        command: String,
        arguments: Option<Value>,
    },
    #[serde(rename = "response")]
    Response {
        seq: usize,
        request_seq: usize,
        success: bool,
        command: String,
        message: Option<String>,
        body: Option<Value>,
    },
    #[serde(rename = "event")]
    Event {
        seq: usize,
        event: String,
        body: Option<Value>,
    },
}

/// DAP response
pub struct DAPResponse {
    pub success: bool,
    pub message: Option<String>,
    pub body: Option<Value>,
}

/// Generic DAP adapter implementation
pub struct GenericDAPAdapter {
    name: String,
    languages: Vec<String>,
    config: AdapterConfig,
    client: Arc<Mutex<Option<DAPClient>>>,
}

impl GenericDAPAdapter {
    /// Create new DAP adapter
    pub fn new(name: &str, languages: Vec<String>, config: AdapterConfig) -> Self {
        Self {
            name: name.to_string(),
            languages,
            config,
            client: Arc::new(Mutex::new(None)),
        }
    }

    /// Send DAP request
    async fn send_request(&self, command: &str, arguments: Option<Value>) -> Result<DAPResponse> {
        let client = self.client.lock().await;
        let client = client.as_ref().ok_or_else(|| anyhow!("Client not connected"))?;

        let seq = {
            let mut next_seq = client.next_seq.lock().await;
            let seq = *next_seq;
            *next_seq += 1;
            seq
        };

        let request = DAPMessage::Request {
            seq,
            command: command.to_string(),
            arguments,
        };

        let (tx, rx) = tokio::sync::oneshot::channel();
        client.pending_requests.lock().await.insert(seq, tx);

        // Send request
        // Would need to write to process stdin

        rx.await.map_err(|e| anyhow!("Request cancelled: {}", e))
    }
}

#[async_trait::async_trait]
impl DebugAdapter for GenericDAPAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn languages(&self) -> Vec<String> {
        self.languages.clone()
    }

    async fn start(&self, config: AdapterConfig) -> Result<String> {
        let mut cmd = TokioCommand::new(&config.command);
        cmd.args(&config.args);
        if let Some(cwd) = &config.cwd {
            cmd.current_dir(cwd);
        }
        cmd.envs(&config.env);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let process = cmd.spawn()?;
        let session_id = uuid::Uuid::new_v4().to_string();

        let client = DAPClient {
            process: Arc::new(Mutex::new(Some(process))),
            next_seq: Arc::new(Mutex::new(1)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        };

        *self.client.lock().await = Some(client);

        // Send initialize request
        let init_args = serde_json::json!({
            "clientID": "parsec-ide",
            "clientName": "Parsec IDE",
            "adapterID": self.name,
            "pathFormat": "path",
            "linesStartAt1": true,
            "columnsStartAt1": true,
            "supportsVariableType": true,
            "supportsVariablePaging": true,
            "supportsRunInTerminalRequest": true,
            "locale": "en"
        });

        let response = self.send_request("initialize", Some(init_args)).await?;
        if !response.success {
            return Err(anyhow!("Initialize failed: {:?}", response.message));
        }

        Ok(session_id)
    }

    async fn stop(&self, session_id: &str) -> Result<()> {
        let _ = self.send_request("disconnect", None).await?;
        
        let mut client = self.client.lock().await;
        if let Some(mut client) = client.take() {
            if let Some(mut process) = client.process.lock().await.take() {
                process.kill().await?;
            }
        }
        Ok(())
    }

    async fn attach(&self, process_id: u32) -> Result<String> {
        let args = serde_json::json!({
            "processId": process_id
        });
        let session_id = self.start(self.config.clone()).await?;
        self.send_request("attach", Some(args)).await?;
        Ok(session_id)
    }

    async fn launch(&self, program: &std::path::Path, args: Vec<String>) -> Result<String> {
        let launch_args = serde_json::json!({
            "program": program,
            "args": args,
            "cwd": program.parent(),
            "stopAtEntry": true,
            "console": "integratedTerminal"
        });
        let session_id = self.start(self.config.clone()).await?;
        self.send_request("launch", Some(launch_args)).await?;
        Ok(session_id)
    }

    async fn pause(&self, session_id: &str) -> Result<()> {
        self.send_request("pause", None).await?;
        Ok(())
    }

    async fn resume(&self, session_id: &str) -> Result<()> {
        self.send_request("continue", None).await?;
        Ok(())
    }

    async fn step_over(&self, session_id: &str, thread_id: usize) -> Result<()> {
        let args = serde_json::json!({ "threadId": thread_id });
        self.send_request("next", Some(args)).await?;
        Ok(())
    }

    async fn step_into(&self, session_id: &str, thread_id: usize) -> Result<()> {
        let args = serde_json::json!({ "threadId": thread_id });
        self.send_request("stepIn", Some(args)).await?;
        Ok(())
    }

    async fn step_out(&self, session_id: &str, thread_id: usize) -> Result<()> {
        let args = serde_json::json!({ "threadId": thread_id });
        self.send_request("stepOut", Some(args)).await?;
        Ok(())
    }

    async fn set_breakpoint(&self, session_id: &str, bp: Breakpoint) -> Result<()> {
        let args = serde_json::json!({
            "source": {
                "path": bp.file
            },
            "breakpoints": [{
                "line": bp.line,
                "condition": bp.condition
            }]
        });
        self.send_request("setBreakpoints", Some(args)).await?;
        Ok(())
    }

    async fn remove_breakpoint(&self, session_id: &str, bp_id: usize) -> Result<()> {
        // Would need source file
        Ok(())
    }

    async fn get_breakpoints(&self, session_id: &str) -> Result<Vec<Breakpoint>> {
        Ok(vec![])
    }

    async fn get_stack_trace(&self, session_id: &str, thread_id: usize) -> Result<Vec<StackFrame>> {
        let args = serde_json::json!({ "threadId": thread_id });
        let response = self.send_request("stackTrace", Some(args)).await?;
        
        let mut frames = Vec::new();
        if let Some(body) = response.body {
            if let Some(stack_frames) = body["stackFrames"].as_array() {
                for frame in stack_frames {
                    frames.push(StackFrame {
                        id: frame["id"].as_u64().unwrap_or(0) as usize,
                        name: frame["name"].as_str().unwrap_or("").to_string(),
                        file: frame["source"]["path"].as_str().map(|s| s.to_string()),
                        line: frame["line"].as_u64().unwrap_or(0) as usize,
                        column: frame["column"].as_u64().unwrap_or(0) as usize,
                        end_line: frame["endLine"].as_u64().map(|l| l as usize),
                    });
                }
            }
        }

        Ok(frames)
    }

    async fn get_scopes(&self, session_id: &str, frame_id: usize) -> Result<Vec<VariableScope>> {
        let args = serde_json::json!({ "frameId": frame_id });
        let response = self.send_request("scopes", Some(args)).await?;
        
        let mut scopes = Vec::new();
        if let Some(body) = response.body {
            if let Some(scopes_array) = body["scopes"].as_array() {
                for scope in scopes_array {
                    scopes.push(VariableScope {
                        name: scope["name"].as_str().unwrap_or("").to_string(),
                        var_ref: scope["variablesReference"].as_u64().unwrap_or(0) as usize,
                        expensive: scope["expensive"].as_bool().unwrap_or(false),
                    });
                }
            }
        }

        Ok(scopes)
    }

    async fn get_variables(&self, session_id: &str, var_ref: usize) -> Result<Vec<Variable>> {
        let args = serde_json::json!({ "variablesReference": var_ref });
        let response = self.send_request("variables", Some(args)).await?;
        
        let mut vars = Vec::new();
        if let Some(body) = response.body {
            if let Some(variables) = body["variables"].as_array() {
                for var in variables {
                    vars.push(Variable {
                        name: var["name"].as_str().unwrap_or("").to_string(),
                        value: var["value"].as_str().unwrap_or("").to_string(),
                        type_name: var["type"].as_str().map(|s| s.to_string()),
                        var_ref: var["variablesReference"].as_u64().unwrap_or(0) as usize,
                        indexed_variables: var["indexedVariables"].as_u64().map(|n| n as usize),
                        named_variables: var["namedVariables"].as_u64().map(|n| n as usize),
                    });
                }
            }
        }

        Ok(vars)
    }

    async fn evaluate(&self, session_id: &str, expression: &str, frame_id: Option<usize>) -> Result<Variable> {
        let mut args = serde_json::json!({
            "expression": expression
        });
        if let Some(frame) = frame_id {
            args["frameId"] = serde_json::json!(frame);
        }

        let response = self.send_request("evaluate", Some(args)).await?;
        
        if let Some(body) = response.body {
            Ok(Variable {
                name: expression.to_string(),
                value: body["result"].as_str().unwrap_or("").to_string(),
                type_name: body["type"].as_str().map(|s| s.to_string()),
                var_ref: body["variablesReference"].as_u64().unwrap_or(0) as usize,
                indexed_variables: None,
                named_variables: None,
            })
        } else {
            Err(anyhow!("Evaluate failed: {:?}", response.message))
        }
    }

    async fn terminate(&self, session_id: &str) -> Result<()> {
        self.send_request("terminate", None).await?;
        Ok(())
    }

    async fn disconnect(&self, session_id: &str) -> Result<()> {
        self.send_request("disconnect", None).await?;
        Ok(())
    }
}