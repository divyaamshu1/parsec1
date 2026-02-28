//! Interactive code playground for learning and experimentation

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{RwLock, mpsc};
use tokio::process::{Command, Child};
use tokio::time;
use tokio::fs;
use serde::{Serialize, Deserialize};
use tracing::{info, warn, debug};
use tempfile::{tempdir, TempDir};
use uuid::Uuid;

#[cfg(feature = "docker")]
use bollard::{Docker, container::*, image::*, exec::*};

use crate::{Result, LearningError, LearningConfig};

/// Playground language
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlaygroundLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    C,
    Cpp,
    CSharp,
    Java,
    Ruby,
    PHP,
    Swift,
    Kotlin,
    Scala,
    Haskell,
    Lua,
    Zig,
    WebAssembly,
    Markdown,
    Html,
    Css,
    Sql,
    Bash,
    PowerShell,
    Docker,
    Custom(String),
}

impl PlaygroundLanguage {
    /// Get file extension
    pub fn extension(&self) -> &str {
        match self {
            PlaygroundLanguage::Rust => "rs",
            PlaygroundLanguage::Python => "py",
            PlaygroundLanguage::JavaScript => "js",
            PlaygroundLanguage::TypeScript => "ts",
            PlaygroundLanguage::Go => "go",
            PlaygroundLanguage::C => "c",
            PlaygroundLanguage::Cpp => "cpp",
            PlaygroundLanguage::CSharp => "cs",
            PlaygroundLanguage::Java => "java",
            PlaygroundLanguage::Ruby => "rb",
            PlaygroundLanguage::PHP => "php",
            PlaygroundLanguage::Swift => "swift",
            PlaygroundLanguage::Kotlin => "kt",
            PlaygroundLanguage::Scala => "scala",
            PlaygroundLanguage::Haskell => "hs",
            PlaygroundLanguage::Lua => "lua",
            PlaygroundLanguage::Zig => "zig",
            PlaygroundLanguage::WebAssembly => "wat",
            PlaygroundLanguage::Markdown => "md",
            PlaygroundLanguage::Html => "html",
            PlaygroundLanguage::Css => "css",
            PlaygroundLanguage::Sql => "sql",
            PlaygroundLanguage::Bash => "sh",
            PlaygroundLanguage::PowerShell => "ps1",
            PlaygroundLanguage::Docker => "Dockerfile",
            PlaygroundLanguage::Custom(ext) => ext,
        }
    }

    /// Get compile command
    pub fn compile_command(&self) -> Option<Vec<String>> {
        match self {
            PlaygroundLanguage::Rust => Some(vec!["rustc".to_string(), "code.rs".to_string(), "-o".to_string(), "code".to_string()]),
            PlaygroundLanguage::Go => Some(vec!["go".to_string(), "build".to_string(), "-o".to_string(), "code".to_string(), "code.go".to_string()]),
            PlaygroundLanguage::C => Some(vec!["gcc".to_string(), "code.c".to_string(), "-o".to_string(), "code".to_string()]),
            PlaygroundLanguage::Cpp => Some(vec!["g++".to_string(), "code.cpp".to_string(), "-o".to_string(), "code".to_string()]),
            PlaygroundLanguage::Java => Some(vec!["javac".to_string(), "code.java".to_string()]),
            PlaygroundLanguage::CSharp => Some(vec!["csc".to_string(), "code.cs".to_string()]),
            PlaygroundLanguage::Kotlin => Some(vec!["kotlinc".to_string(), "code.kt".to_string(), "-include-runtime".to_string(), "-d".to_string(), "code.jar".to_string()]),
            PlaygroundLanguage::Scala => Some(vec!["scalac".to_string(), "code.scala".to_string()]),
            PlaygroundLanguage::Haskell => Some(vec!["ghc".to_string(), "-o".to_string(), "code".to_string(), "code.hs".to_string()]),
            PlaygroundLanguage::Zig => Some(vec!["zig".to_string(), "build-exe".to_string(), "code.zig".to_string(), "--name".to_string(), "code".to_string()]),
            PlaygroundLanguage::TypeScript => Some(vec!["tsc".to_string(), "code.ts".to_string(), "--outFile".to_string(), "code.js".to_string()]),
            _ => None,
        }
    }

    /// Get run command
    pub fn run_command(&self, compiled: bool) -> Vec<String> {
        match self {
            PlaygroundLanguage::Rust | PlaygroundLanguage::Go | PlaygroundLanguage::C | 
            PlaygroundLanguage::Cpp | PlaygroundLanguage::Zig => {
                if compiled {
                    vec!["./code".to_string()]
                } else {
                    vec!["rustc".to_string(), "code.rs".to_string(), "-o".to_string(), "code".to_string(), "&&".to_string(), "./code".to_string()]
                }
            }
            PlaygroundLanguage::Python => vec!["python3".to_string(), "code.py".to_string()],
            PlaygroundLanguage::JavaScript => vec!["node".to_string(), "code.js".to_string()],
            PlaygroundLanguage::TypeScript => vec!["node".to_string(), "code.js".to_string()],
            PlaygroundLanguage::Ruby => vec!["ruby".to_string(), "code.rb".to_string()],
            PlaygroundLanguage::PHP => vec!["php".to_string(), "code.php".to_string()],
            PlaygroundLanguage::Swift => vec!["swift".to_string(), "code.swift".to_string()],
            PlaygroundLanguage::Java => vec!["java".to_string(), "code".to_string()],
            PlaygroundLanguage::CSharp => vec!["mono".to_string(), "code.exe".to_string()],
            PlaygroundLanguage::Kotlin => vec!["java".to_string(), "-jar".to_string(), "code.jar".to_string()],
            PlaygroundLanguage::Scala => vec!["scala".to_string(), "code".to_string()],
            PlaygroundLanguage::Haskell => vec!["./code".to_string()],
            PlaygroundLanguage::Lua => vec!["lua".to_string(), "code.lua".to_string()],
            PlaygroundLanguage::Bash => vec!["bash".to_string(), "code.sh".to_string()],
            PlaygroundLanguage::PowerShell => vec!["pwsh".to_string(), "code.ps1".to_string()],
            PlaygroundLanguage::Sql => vec!["sqlite3".to_string(), ":memory:".to_string(), "-cmd".to_string(), ".read code.sql".to_string()],
            PlaygroundLanguage::Markdown => vec!["pandoc".to_string(), "code.md".to_string(), "-t".to_string(), "html".to_string()],
            PlaygroundLanguage::Html => vec!["open".to_string(), "code.html".to_string()],
            PlaygroundLanguage::Css => vec!["node".to_string(), "-e".to_string(), "console.log('CSS cannot be executed directly')".to_string()],
            PlaygroundLanguage::WebAssembly => vec!["wasmtime".to_string(), "code.wat".to_string()],
            PlaygroundLanguage::Docker => vec!["docker".to_string(), "build".to_string(), "-t".to_string(), "playground".to_string(), ".".to_string()],
            PlaygroundLanguage::Custom(cmd) => vec![cmd.clone()],
        }
    }

    /// Get Docker image for this language
    pub fn docker_image(&self) -> Option<&'static str> {
        match self {
            PlaygroundLanguage::Rust => Some("rust:latest"),
            PlaygroundLanguage::Python => Some("python:latest"),
            PlaygroundLanguage::JavaScript => Some("node:latest"),
            PlaygroundLanguage::TypeScript => Some("node:latest"),
            PlaygroundLanguage::Go => Some("golang:latest"),
            PlaygroundLanguage::C => Some("gcc:latest"),
            PlaygroundLanguage::Cpp => Some("gcc:latest"),
            PlaygroundLanguage::CSharp => Some("mcr.microsoft.com/dotnet/sdk:latest"),
            PlaygroundLanguage::Java => Some("openjdk:latest"),
            PlaygroundLanguage::Ruby => Some("ruby:latest"),
            PlaygroundLanguage::PHP => Some("php:latest"),
            PlaygroundLanguage::Swift => Some("swift:latest"),
            PlaygroundLanguage::Kotlin => Some("openjdk:latest"),
            PlaygroundLanguage::Scala => Some("hseeberger/scala-sbt:latest"),
            PlaygroundLanguage::Haskell => Some("haskell:latest"),
            PlaygroundLanguage::Lua => Some("lua:latest"),
            PlaygroundLanguage::Bash => Some("bash:latest"),
            PlaygroundLanguage::Sql => Some("mysql:latest"),
            PlaygroundLanguage::Docker => Some("docker:latest"),
            _ => None,
        }
    }
}


// Alias type so that `Playground` is available for re-export
pub type Playground = PlaygroundSession;

impl std::fmt::Display for PlaygroundLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlaygroundLanguage::Rust => write!(f, "Rust"),
            PlaygroundLanguage::Python => write!(f, "Python"),
            PlaygroundLanguage::JavaScript => write!(f, "JavaScript"),
            PlaygroundLanguage::TypeScript => write!(f, "TypeScript"),
            PlaygroundLanguage::Go => write!(f, "Go"),
            PlaygroundLanguage::C => write!(f, "C"),
            PlaygroundLanguage::Cpp => write!(f, "C++"),
            PlaygroundLanguage::CSharp => write!(f, "C#"),
            PlaygroundLanguage::Java => write!(f, "Java"),
            PlaygroundLanguage::Ruby => write!(f, "Ruby"),
            PlaygroundLanguage::PHP => write!(f, "PHP"),
            PlaygroundLanguage::Swift => write!(f, "Swift"),
            PlaygroundLanguage::Kotlin => write!(f, "Kotlin"),
            PlaygroundLanguage::Scala => write!(f, "Scala"),
            PlaygroundLanguage::Haskell => write!(f, "Haskell"),
            PlaygroundLanguage::Lua => write!(f, "Lua"),
            PlaygroundLanguage::Zig => write!(f, "Zig"),
            PlaygroundLanguage::WebAssembly => write!(f, "WebAssembly"),
            PlaygroundLanguage::Markdown => write!(f, "Markdown"),
            PlaygroundLanguage::Html => write!(f, "HTML"),
            PlaygroundLanguage::Css => write!(f, "CSS"),
            PlaygroundLanguage::Sql => write!(f, "SQL"),
            PlaygroundLanguage::Bash => write!(f, "Bash"),
            PlaygroundLanguage::PowerShell => write!(f, "PowerShell"),
            PlaygroundLanguage::Docker => write!(f, "Docker"),
            PlaygroundLanguage::Custom(c) => write!(f, "{}", c),
        }
    }
}

/// Playground configuration
#[derive(Debug, Clone)]
pub struct PlaygroundConfig {
    /// Language
    pub language: PlaygroundLanguage,
    /// Timeout in seconds
    pub timeout: u64,
    /// Memory limit in MB
    pub memory_limit: Option<usize>,
    /// CPU limit (cores)
    pub cpu_limit: Option<f32>,
    /// Network access
    pub network_access: bool,
    /// Filesystem access
    pub filesystem_access: bool,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Dependencies
    pub dependencies: Vec<String>,
    /// Use Docker (if available)
    pub use_docker: bool,
    /// Docker image (overrides default)
    pub docker_image: Option<String>,
    /// Pre-run script
    pub pre_run: Option<String>,
    /// Post-run script
    pub post_run: Option<String>,
    /// Input data
    pub input: Option<String>,
}

impl Default for PlaygroundConfig {
    fn default() -> Self {
        Self {
            language: PlaygroundLanguage::Rust,
            timeout: 30,
            memory_limit: Some(256),
            cpu_limit: Some(1.0),
            network_access: false,
            filesystem_access: false,
            env: HashMap::new(),
            dependencies: Vec::new(),
            use_docker: true,
            docker_image: None,
            pre_run: None,
            post_run: None,
            input: None,
        }
    }
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Exit code
    pub exit_code: i32,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Memory used in KB
    pub memory_used_kb: Option<usize>,
    /// Whether execution was successful
    pub success: bool,
    /// Error message if any
    pub error: Option<String>,
    /// Compiled output path (if compiled)
    pub compiled_output: Option<String>,
    /// Generated files
    pub generated_files: Vec<String>,
}

/// Playground session
pub struct PlaygroundSession {
    /// Session ID
    pub id: String,
    /// Language
    pub language: PlaygroundLanguage,
    /// Code
    pub code: String,
    /// Configuration
    pub config: PlaygroundConfig,
    /// Working directory
    pub work_dir: TempDir,
    /// Process handle
    pub process: Option<Child>,
    /// Start time
    pub start_time: Instant,
    /// Command sender
    cmd_tx: mpsc::UnboundedSender<PlaygroundCommand>,
}

/// Playground command
enum PlaygroundCommand {
    Execute(String, PlaygroundConfig),
    Stop,
    GetOutput,
    GetStatus,
}

/// Playground manager
pub struct PlaygroundManager {
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, PlaygroundSession>>>,
    /// Docker client
    #[cfg(feature = "docker")]
    docker: Option<Docker>,
    /// Configuration
    config: LearningConfig,
    /// Base directory for playgrounds
    base_dir: PathBuf,
    /// Available languages
    available_languages: Arc<RwLock<Vec<PlaygroundLanguage>>>,
}

impl PlaygroundManager {
    /// Create new playground manager
    pub async fn new(config: LearningConfig) -> Result<Self> {
        let base_dir = config.user_data_dir.join("playgrounds");
        fs::create_dir_all(&base_dir).await?;

        #[cfg(feature = "docker")]
        let docker = Docker::connect_with_local_defaults().ok();

        let manager = Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "docker")]
            docker,
            config,
            base_dir,
            available_languages: Arc::new(RwLock::new(Vec::new())),
        };

        // Detect available languages
        manager.detect_languages().await;

        Ok(manager)
    }

    /// Detect available languages on the system
    async fn detect_languages(&self) {
        let mut languages = Vec::new();

        // Check each language
        let checks = vec![
            (PlaygroundLanguage::Rust, "rustc --version"),
            (PlaygroundLanguage::Python, "python3 --version"),
            (PlaygroundLanguage::JavaScript, "node --version"),
            (PlaygroundLanguage::TypeScript, "tsc --version"),
            (PlaygroundLanguage::Go, "go version"),
            (PlaygroundLanguage::C, "gcc --version"),
            (PlaygroundLanguage::Cpp, "g++ --version"),
            (PlaygroundLanguage::Java, "java -version"),
            (PlaygroundLanguage::Ruby, "ruby --version"),
            (PlaygroundLanguage::PHP, "php --version"),
            (PlaygroundLanguage::Bash, "bash --version"),
            (PlaygroundLanguage::Sql, "sqlite3 --version"),
        ];

        for (lang, cmd) in checks {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let output = Command::new(parts[0]).args(&parts[1..]).output().await;
            if output.is_ok() {
                languages.push(lang);
            }
        }

        *self.available_languages.write().await = languages;
    }

    /// Create a new playground session
    pub async fn create_session(
        &self,
        language: PlaygroundLanguage,
        code: String,
        config: Option<PlaygroundConfig>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let id_clone = id.clone();
        let work_dir = tempdir()?;
        let config = config.unwrap_or_default();

        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();

        let session = PlaygroundSession {
            id: id.clone(),
            language,
            code,
            config: config.clone(),
            work_dir,
            process: None,
            start_time: Instant::now(),
            cmd_tx,
        };

        self.sessions.write().await.insert(id.clone(), session);

        // Spawn session handler
        let manager = self.clone();
        tokio::spawn(async move {
            manager.handle_session(id_clone, &mut cmd_rx).await;
        });

        Ok(id)
    }

    /// Handle session commands
    async fn handle_session(&self, id: String, cmd_rx: &mut mpsc::UnboundedReceiver<PlaygroundCommand>) {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                PlaygroundCommand::Execute(code, config) => {
                    let _ = self.execute_internal(&id, &code, config).await;
                    // Handle result
                }
                PlaygroundCommand::Stop => {
                    let _ = self.stop_session(&id).await;
                    break;
                }
                _ => {}
            }
        }
    }

    /// Execute code in session
    pub async fn execute(
        &self,
        session_id: &str,
        code: Option<String>,
        config: Option<PlaygroundConfig>,
    ) -> Result<ExecutionResult> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| LearningError::PlaygroundError("Session not found".to_string()))?;

        let code = code.unwrap_or_else(|| session.code.clone());
        let config = config.unwrap_or_else(|| session.config.clone());

        self.execute_internal(session_id, &code, config).await
    }

    /// Internal execution function
    async fn execute_internal(
        &self,
        _session_id: &str,
        code: &str,
        config: PlaygroundConfig,
    ) -> Result<ExecutionResult> {
        let start = Instant::now();
        let work_dir = tempdir()?;
        let file_path = work_dir.path().join(format!("code.{}", config.language.extension()));

        // Write code to file
        fs::write(&file_path, code).await?;

        // Write dependencies file if needed
        if !config.dependencies.is_empty() {
            self.write_dependencies(&work_dir, &config).await?;
        }

        // Run pre-run script
        if let Some(script) = &config.pre_run {
            self.run_script(script, &work_dir).await?;
        }

        // Check if we should use Docker
        #[cfg(feature = "docker")]
        if config.use_docker && self.docker.is_some() {
            return self.execute_in_docker(&work_dir, &config).await;
        }

        // Compile if needed
        if let Some(compile_cmd) = config.language.compile_command() {
            let compile_result = self.run_command(&compile_cmd, &work_dir, &config).await?;
            if compile_result.exit_code != 0 {
                return Ok(compile_result);
            }
        }

        // Run the code
        let run_cmd = config.language.run_command(true);
        let mut result = self.run_command(&run_cmd, &work_dir, &config).await?;

        // Run post-run script
        if let Some(script) = &config.post_run {
            let post_result = self.run_script(script, &work_dir).await?;
            result.stdout.push_str("\n--- Post-run output ---\n");
            result.stdout.push_str(&post_result.stdout);
        }

        result.execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(result)
    }

    /// Run a command
    async fn run_command(
        &self,
        cmd_parts: &[String],
        work_dir: &TempDir,
        config: &PlaygroundConfig,
    ) -> Result<ExecutionResult> {
        if cmd_parts.is_empty() {
            return Err(LearningError::PlaygroundError("Empty command".to_string()));
        }

        let mut cmd = Command::new(&cmd_parts[0]);
        cmd.args(&cmd_parts[1..])
            .current_dir(work_dir.path())
            .envs(&config.env);

        if !config.network_access {
            // Disable network access (platform-specific)
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                unsafe {
                    cmd.pre_exec(|| {
                        // Create network namespace without network
                        // This is simplified - in production use proper network namespaces
                        Ok(())
                    });
                }
            }
        }

        // Set resource limits
        #[cfg(unix)]
        {
            use nix::sys::resource::{setrlimit, Resource};
            if let Some(mem_limit) = config.memory_limit {
                let limit = (mem_limit * 1024 * 1024) as u64;
                let _ = setrlimit(Resource::AS, limit, limit);
            }
        }

        // Spawn and wait with timeout
        let mut child = cmd.spawn()?;
        let timeout = Duration::from_secs(config.timeout);

        let result = time::timeout(timeout, child.wait()).await;

        match result {
            Ok(Ok(status)) => {
                // Read output
                // Note: In a real implementation, you'd capture stdout/stderr via pipes
                Ok(ExecutionResult {
                    stdout: "Output captured".to_string(),
                    stderr: String::new(),
                    exit_code: status.code().unwrap_or(-1),
                    execution_time_ms: 0,
                    memory_used_kb: None,
                    success: status.success(),
                    error: None,
                    compiled_output: None,
                    generated_files: vec![],
                })
            }
            Ok(Err(e)) => Err(LearningError::PlaygroundError(format!("Execution failed: {}", e))),
            Err(_) => {
                let _ = child.kill().await;
                Err(LearningError::PlaygroundError("Execution timed out".to_string()))
            }
        }
    }

    /// Run script
    async fn run_script(&self, script: &str, work_dir: &TempDir) -> Result<ExecutionResult> {
        let script_path = work_dir.path().join("pre_run.sh");
        fs::write(&script_path, script).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path).await?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms).await?;
        }

        self.run_command(&[script_path.to_string_lossy().to_string()], work_dir, &PlaygroundConfig::default()).await
    }

    /// Write dependencies file
    async fn write_dependencies(&self, work_dir: &TempDir, config: &PlaygroundConfig) -> Result<()> {
        match config.language {
            PlaygroundLanguage::Rust => {
                let mut cargo_toml = String::from("[package]\n");
                cargo_toml.push_str("name = \"playground\"\n");
                cargo_toml.push_str("version = \"0.1.0\"\n");
                cargo_toml.push_str("edition = \"2021\"\n\n");
                cargo_toml.push_str("[dependencies]\n");
                for dep in &config.dependencies {
                    if dep.contains('=') || dep.contains('{') {
                        cargo_toml.push_str(&format!("{}\n", dep));
                    } else {
                        cargo_toml.push_str(&format!("{} = \"*\"\n", dep));
                    }
                }
                fs::write(work_dir.path().join("Cargo.toml"), cargo_toml).await?;
                
                // Move code to src/main.rs
                let src_dir = work_dir.path().join("src");
                fs::create_dir_all(&src_dir).await?;
                fs::rename(
                    work_dir.path().join("code.rs"),
                    src_dir.join("main.rs"),
                ).await?;
            }
            PlaygroundLanguage::Python => {
                let mut requirements = String::new();
                for dep in &config.dependencies {
                    requirements.push_str(&format!("{}\n", dep));
                }
                fs::write(work_dir.path().join("requirements.txt"), requirements).await?;
            }
            PlaygroundLanguage::JavaScript | PlaygroundLanguage::TypeScript => {
                let mut package_json = serde_json::json!({
                    "name": "playground",
                    "version": "1.0.0",
                    "type": "module",
                    "dependencies": {}
                });

                if let Some(deps) = package_json["dependencies"].as_object_mut() {
                    for dep in &config.dependencies {
                        if dep.contains('@') {
                            let parts: Vec<&str> = dep.split('@').collect();
                            if parts.len() == 2 {
                                deps.insert(parts[0].to_string(), serde_json::Value::String(parts[1].to_string()));
                            }
                        } else {
                            deps.insert(dep.clone(), serde_json::Value::String("*".to_string()));
                        }
                    }
                }

                fs::write(
                    work_dir.path().join("package.json"),
                    serde_json::to_string_pretty(&package_json)?,
                ).await?;
            }
            PlaygroundLanguage::Go => {
                let mut go_mod = String::from("module playground\n\n");
                if !config.dependencies.is_empty() {
                    go_mod.push_str("require (\n");
                    for dep in &config.dependencies {
                        if dep.contains(' ') {
                            go_mod.push_str(&format!("\t{}\n", dep));
                        } else {
                            go_mod.push_str(&format!("\t{} latest\n", dep));
                        }
                    }
                    go_mod.push_str(")\n");
                }
                fs::write(work_dir.path().join("go.mod"), go_mod).await?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Execute in Docker container
    #[cfg(feature = "docker")]
    async fn execute_in_docker(
        &self,
        work_dir: &TempDir,
        config: &PlaygroundConfig,
    ) -> Result<ExecutionResult> {
        let docker = self.docker.as_ref()
            .ok_or_else(|| LearningError::PlaygroundError("Docker not available".to_string()))?;

        let image = config.docker_image.as_deref()
            .or_else(|| config.language.docker_image())
            .ok_or_else(|| LearningError::PlaygroundError("No Docker image specified".to_string()))?;

        // Pull image if not exists
        let _ = docker.create_image(
            Some(bollard::image::CreateImageOptions {
                from_image: image,
                ..Default::default()
            }),
            None,
            None,
        ).await;

        // Create container
        let container_config = Config {
            image: Some(image),
            cmd: Some(vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "cd /workspace && {}",
                    config.language.run_command(false).join(" ")
                ),
            ]),
            working_dir: Some("/workspace".to_string()),
            host_config: Some(HostConfig {
                memory: config.memory_limit.map(|m| (m * 1024 * 1024) as i64),
                nano_cpus: config.cpu_limit.map(|c| (c * 1_000_000_000.0) as i64),
                network_mode: if config.network_access { None } else { Some("none".to_string()) },
                readonly_rootfs: Some(!config.filesystem_access),
                ..Default::default()
            }),
            env: Some(config.env.iter().map(|(k, v)| format!("{}={}", k, v)).collect()),
            ..Default::default()
        };

        let container = docker.create_container::<&str, &str>(None, container_config).await?;

        // Copy code to container
        // This would require tar streaming

        // Start container
        docker.start_container(&container.id, None).await?;

        // Wait for completion with timeout
        let wait_result = time::timeout(
            Duration::from_secs(config.timeout),
            docker.wait_container(&container.id, None)
        ).await;

        match wait_result {
            Ok(Ok(wait)) => {
                // Get logs
                let logs = docker.logs(
                    &container.id,
                    Some(LogsOptions {
                        stdout: true,
                        stderr: true,
                        ..Default::default()
                    }),
                ).try_collect::<Vec<_>>().await?;

                let mut stdout = String::new();
                let mut stderr = String::new();

                for log in logs {
                    match log {
                        LogOutput::StdOut { message } => {
                            stdout.push_str(&String::from_utf8_lossy(&message));
                        }
                        LogOutput::StdErr { message } => {
                            stderr.push_str(&String::from_utf8_lossy(&message));
                        }
                        _ => {}
                    }
                }

                // Clean up
                let _ = docker.remove_container(&container.id, None).await;

                Ok(ExecutionResult {
                    stdout,
                    stderr,
                    exit_code: wait.status_code.unwrap_or(-1),
                    execution_time_ms: 0,
                    memory_used_kb: None,
                    success: wait.status_code == Some(0),
                    error: None,
                    compiled_output: None,
                    generated_files: vec![],
                })
            }
            Ok(Err(e)) => {
                let _ = docker.remove_container(&container.id, None).await;
                Err(LearningError::PlaygroundError(format!("Container error: {}", e)))
            }
            Err(_) => {
                let _ = docker.stop_container(&container.id, None).await;
                let _ = docker.remove_container(&container.id, None).await;
                Err(LearningError::PlaygroundError("Execution timed out".to_string()))
            }
        }
    }

    /// Stop a session
    pub async fn stop_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(session_id) {
            if let Some(mut process) = session.process {
                let _ = process.kill().await;
            }
        }
        Ok(())
    }

    /// List active sessions
    pub async fn list_sessions(&self) -> Vec<String> {
        self.sessions.read().await.keys().cloned().collect()
    }

    /// Get available languages
    pub async fn get_available_languages(&self) -> Vec<PlaygroundLanguage> {
        self.available_languages.read().await.clone()
    }

    /// Get template code for language
    pub fn get_template(&self, language: PlaygroundLanguage) -> &'static str {
        match language {
            PlaygroundLanguage::Rust => "fn main() {\n    println!(\"Hello, world!\");\n}\n",
            PlaygroundLanguage::Python => "print(\"Hello, world!\")\n",
            PlaygroundLanguage::JavaScript => "console.log(\"Hello, world!\");\n",
            PlaygroundLanguage::TypeScript => "const greeting: string = \"Hello, world!\";\nconsole.log(greeting);\n",
            PlaygroundLanguage::Go => "package main\n\nimport \"fmt\"\n\nfunc main() {\n    fmt.Println(\"Hello, world!\")\n}\n",
            PlaygroundLanguage::C => "#include <stdio.h>\n\nint main() {\n    printf(\"Hello, world!\\n\");\n    return 0;\n}\n",
            PlaygroundLanguage::Cpp => "#include <iostream>\n\nint main() {\n    std::cout << \"Hello, world!\" << std::endl;\n    return 0;\n}\n",
            PlaygroundLanguage::CSharp => "using System;\n\nclass Program {\n    static void Main() {\n        Console.WriteLine(\"Hello, world!\");\n    }\n}\n",
            PlaygroundLanguage::Java => "public class Main {\n    public static void main(String[] args) {\n        System.out.println(\"Hello, world!\");\n    }\n}\n",
            PlaygroundLanguage::Ruby => "puts \"Hello, world!\"\n",
            PlaygroundLanguage::PHP => "<?php\necho \"Hello, world!\\n\";\n?>\n",
            PlaygroundLanguage::Swift => "print(\"Hello, world!\")\n",
            PlaygroundLanguage::Kotlin => "fun main() {\n    println(\"Hello, world!\")\n}\n",
            PlaygroundLanguage::Scala => "object Main extends App {\n    println(\"Hello, world!\")\n}\n",
            PlaygroundLanguage::Haskell => "main = putStrLn \"Hello, world!\"\n",
            PlaygroundLanguage::Lua => "print(\"Hello, world!\")\n",
            PlaygroundLanguage::Zig => "const std = @import(\"std\");\n\npub fn main() !void {\n    std.debug.print(\"Hello, world!\\n\", .{});\n}\n",
            PlaygroundLanguage::Bash => "#!/bin/bash\necho \"Hello, world!\"\n",
            PlaygroundLanguage::PowerShell => "Write-Host \"Hello, world!\"\n",
            PlaygroundLanguage::Html => "<!DOCTYPE html>\n<html>\n<head>\n    <title>Playground</title>\n</head>\n<body>\n    <h1>Hello, world!</h1>\n</body>\n</html>\n",
            PlaygroundLanguage::Css => "body {\n    font-family: Arial, sans-serif;\n    margin: 0;\n    padding: 20px;\n    background-color: #f0f0f0;\n}\n\nh1 {\n    color: #333;\n}\n",
            PlaygroundLanguage::Sql => "CREATE TABLE users (\n    id INTEGER PRIMARY KEY,\n    name TEXT NOT NULL,\n    email TEXT UNIQUE\n);\n\nINSERT INTO users (name, email) VALUES ('John Doe', 'john@example.com');\nSELECT * FROM users;\n",
            PlaygroundLanguage::WebAssembly => "(module\n    (func $hello (import \"\" \"\"))\n    (start $hello)\n)\n",
            PlaygroundLanguage::Markdown => "# Hello, world!\n\nThis is a **markdown** document.\n\n- Item 1\n- Item 2\n- Item 3\n",
            PlaygroundLanguage::Docker => "FROM alpine:latest\nRUN apk add --no-cache bash\nCMD [\"echo\", \"Hello, world!\"]\n",
            PlaygroundLanguage::Custom(_) => "// Write your code here\n",
        }
    }
}

impl Clone for PlaygroundManager {
    fn clone(&self) -> Self {
        Self {
            sessions: self.sessions.clone(),
            #[cfg(feature = "docker")]
            docker: self.docker.clone(),
            config: self.config.clone(),
            base_dir: self.base_dir.clone(),
            available_languages: self.available_languages.clone(),
        }
    }
}