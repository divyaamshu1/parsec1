//! Sandboxing for secure extension execution
//!
//! Provides resource limits, capability restrictions, and security policies.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

use crate::ExtensionManifest;

/// Sandbox configuration for an extension
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Maximum memory in bytes
    pub max_memory: usize,
    /// Maximum CPU time in milliseconds
    pub max_cpu_time_ms: u64,
    /// Maximum file size in bytes
    pub max_file_size: usize,
    /// Maximum number of open files
    pub max_open_files: usize,
    /// Allowed file system paths
    pub allowed_paths: Vec<PathBuf>,
    /// Allowed network domains
    pub allowed_domains: Vec<String>,
    /// Allowed environment variables
    pub allowed_env_vars: Vec<String>,
    /// Enable networking
    pub enable_networking: bool,
    /// Enable file system
    pub enable_filesystem: bool,
    /// Enable subprocesses
    pub enable_subprocesses: bool,
    /// Enable clipboard access
    pub enable_clipboard: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_memory: 50 * 1024 * 1024, // 50MB
            max_cpu_time_ms: 5000,         // 5 seconds
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_open_files: 50,
            allowed_paths: vec![std::env::temp_dir()],
            allowed_domains: vec!["localhost".to_string()],
            allowed_env_vars: vec!["PATH".to_string(), "HOME".to_string()],
            enable_networking: false,
            enable_filesystem: false,
            enable_subprocesses: false,
            enable_clipboard: false,
        }
    }
}

/// Sandbox for secure extension execution
pub struct Sandbox {
    /// Configuration
    config: SandboxConfig,
    /// Active resources
    resources: ResourceTracker,
}

/// Resource tracker for active allocations
#[derive(Debug, Default)]
struct ResourceTracker {
    /// Current memory usage
    memory_used: usize,
    /// Open file handles
    open_files: HashSet<PathBuf>,
    /// Active network connections
    active_connections: usize,
}

impl Sandbox {
    /// Create a new sandbox with default config
    pub fn new() -> Self {
        Self {
            config: SandboxConfig::default(),
            resources: ResourceTracker::default(),
        }
    }

    /// Create a sandbox with custom config
    pub fn with_config(config: SandboxConfig) -> Self {
        Self {
            config,
            resources: ResourceTracker::default(),
        }
    }

    /// Create WASI context for sandboxed execution
    pub fn create_wasi_context(&self, manifest: &ExtensionManifest) -> Result<WasiCtx> {
        let mut builder = WasiCtxBuilder::new();

        // Inherit stdio (for logging)
        builder.inherit_stdio();

        // Add allowed directories
        // NOTE: Directory preopen API has changed in newer wasmtime_wasi versions
        // This feature is temporarily disabled

        // Add allowed environment variables
        for var in &self.config.allowed_env_vars {
            if let Ok(value) = std::env::var(var) {
                #[allow(unused)]
                let _result = builder.env(var, &value);
            }
        }

        // Set arguments  
        #[allow(unused)]
        let _result = builder.arg(&manifest.name);

        Ok(builder.build())
    }

    /// Validate file system access
    pub fn validate_fs_access(&self, path: &Path, write: bool) -> Result<()> {
        if !self.config.enable_filesystem && write {
            return Err(anyhow!("File system write access denied"));
        }

        // Check if path is allowed
        let canonical = path.canonicalize()?;
        
        for allowed in &self.config.allowed_paths {
            let allowed_canonical = allowed.canonicalize()?;
            if canonical.starts_with(&allowed_canonical) {
                // Check file size limit for writes
                if write && canonical.is_file() {
                    let metadata = std::fs::metadata(&canonical)?;
                    if metadata.len() > self.config.max_file_size as u64 {
                        return Err(anyhow!("File exceeds maximum size"));
                    }
                }
                return Ok(());
            }
        }

        Err(anyhow!("File system access denied: {}", path.display()))
    }

    /// Validate network access
    pub fn validate_network_access(&self, domain: &str, _port: Option<u16>) -> Result<()> {
        if !self.config.enable_networking {
            return Err(anyhow!("Network access disabled"));
        }

        // Check localhost
        if domain == "localhost" || domain == "127.0.0.1" {
            return Ok(());
        }

        // Check allowed domains
        for allowed in &self.config.allowed_domains {
            if domain.ends_with(allowed) {
                return Ok(());
            }
        }

        Err(anyhow!("Network access denied: {}", domain))
    }

    /// Validate subprocess creation
    pub fn validate_subprocess(&self, program: &str) -> Result<()> {
        if !self.config.enable_subprocesses {
            return Err(anyhow!("Subprocess creation disabled"));
        }

        // Only allow specific programs
        let allowed_programs = ["git", "node", "npm", "python", "rustc", "cargo"];
        if allowed_programs.contains(&program) {
            Ok(())
        } else {
            Err(anyhow!("Subprocess '{}' not allowed", program))
        }
    }

    /// Check memory limit
    pub fn check_memory(&mut self, additional: usize) -> Result<()> {
        if self.resources.memory_used + additional > self.config.max_memory {
            Err(anyhow!("Memory limit exceeded"))
        } else {
            self.resources.memory_used += additional;
            Ok(())
        }
    }

    /// Release memory
    pub fn release_memory(&mut self, amount: usize) {
        self.resources.memory_used = self.resources.memory_used.saturating_sub(amount);
    }

    /// Track open file
    pub fn track_open_file(&mut self, path: PathBuf) -> Result<()> {
        if self.resources.open_files.len() >= self.config.max_open_files {
            Err(anyhow!("Too many open files"))
        } else {
            self.resources.open_files.insert(path);
            Ok(())
        }
    }

    /// Track closed file
    pub fn track_closed_file(&mut self, path: &Path) {
        self.resources.open_files.remove(path);
    }

    /// Get current resource usage
    pub fn resource_usage(&self) -> ResourceUsage {
        ResourceUsage {
            memory_used: self.resources.memory_used,
            open_files: self.resources.open_files.len(),
            active_connections: self.resources.active_connections,
            memory_limit: self.config.max_memory,
            file_limit: self.config.max_open_files,
        }
    }

    /// Apply resource limits (Unix only)
    #[cfg(target_os = "linux")]
    pub fn apply_resource_limits(&self) -> Result<()> {
        use rlimit::{Resource, getrlimit, setrlimit};

        // Set memory limit
        let (soft, hard) = getrlimit(Resource::AS)?;
        setrlimit(Resource::AS, self.config.max_memory as u64, hard)?;

        // Set file size limit
        let (soft, hard) = getrlimit(Resource::FSIZE)?;
        setrlimit(Resource::FSIZE, self.config.max_file_size as u64, hard)?;

        // Set number of files limit
        let (soft, hard) = getrlimit(Resource::NOFILE)?;
        setrlimit(Resource::NOFILE, self.config.max_open_files as u64, hard)?;

        // Set CPU time limit
        let (soft, hard) = getrlimit(Resource::CPU)?;
        setrlimit(Resource::CPU, self.config.max_cpu_time_ms / 1000, hard)?;

        Ok(())
    }

    /// Apply seccomp filter (Linux only)
    #[cfg(target_os = "linux")]
    pub fn apply_seccomp_filter(&self) -> Result<()> {
        use seccompiler::{
            SeccompFilter, SeccompAction, SeccompRule, 
            TargetArch, BpfProgram, install
        };

        // Allow only necessary syscalls
        let filter = SeccompFilter::new(
            vec![
                // File system
                (libc::SYS_read, vec![]),
                (libc::SYS_write, vec![]),
                (libc::SYS_open, vec![]),
                (libc::SYS_close, vec![]),
                (libc::SYS_stat, vec![]),
                (libc::SYS_fstat, vec![]),
                (libc::SYS_lstat, vec![]),
                (libc::SYS_pread64, vec![]),
                (libc::SYS_pwrite64, vec![]),
                (libc::SYS_readv, vec![]),
                (libc::SYS_writev, vec![]),
                
                // Memory
                (libc::SYS_mmap, vec![]),
                (libc::SYS_munmap, vec![]),
                (libc::SYS_mprotect, vec![]),
                (libc::SYS_brk, vec![]),
                
                // Process
                (libc::SYS_exit, vec![]),
                (libc::SYS_exit_group, vec![]),
                (libc::SYS_gettid, vec![]),
                (libc::SYS_getpid, vec![]),
                (libc::SYS_getppid, vec![]),
                
                // Signals
                (libc::SYS_rt_sigaction, vec![]),
                (libc::SYS_rt_sigprocmask, vec![]),
                (libc::SYS_rt_sigreturn, vec![]),
                
                // Time
                (libc::SYS_clock_gettime, vec![]),
                (libc::SYS_gettimeofday, vec![]),
                (libc::SYS_nanosleep, vec![]),
                
                // Miscellaneous
                (libc::SYS_uname, vec![]),
                (libc::SYS_getuid, vec![]),
                (libc::SYS_geteuid, vec![]),
                (libc::SYS_getgid, vec![]),
                (libc::SYS_getegid, vec![]),
            ]
            .into_iter()
            .map(|(syscall, rules)| (syscall, SeccompRule::new(rules, SeccompAction::Allow)))
            .collect(),
            SeccompAction::Trap,
            TargetArch::native().unwrap(),
        )?;

        let bpf: BpfProgram = filter.try_into()?;
        install(bpf)?;

        Ok(())
    }
}

impl Default for Sandbox {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource usage statistics
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub memory_used: usize,
    pub open_files: usize,
    pub active_connections: usize,
    pub memory_limit: usize,
    pub file_limit: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_creation() {
        let sandbox = Sandbox::new();
        assert_eq!(sandbox.resource_usage().memory_used, 0);
    }

    #[test]
    fn test_memory_limits() {
        let mut sandbox = Sandbox::new();
        
        // Check memory within limit
        assert!(sandbox.check_memory(1024).is_ok());
        assert_eq!(sandbox.resource_usage().memory_used, 1024);
        
        // Release memory
        sandbox.release_memory(512);
        assert_eq!(sandbox.resource_usage().memory_used, 512);
    }

    #[test]
    fn test_memory_exceeded() {
        let mut sandbox = Sandbox::with_config(SandboxConfig {
            max_memory: 1024,
            ..Default::default()
        });
        
        assert!(sandbox.check_memory(1024).is_ok());
        assert!(sandbox.check_memory(1).is_err()); // Exceeded
    }

    #[test]
    fn test_file_tracking() {
        let mut sandbox = Sandbox::with_config(SandboxConfig {
            max_open_files: 2,
            ..Default::default()
        });
        
        let path1 = PathBuf::from("/tmp/test1.txt");
        let path2 = PathBuf::from("/tmp/test2.txt");
        let path3 = PathBuf::from("/tmp/test3.txt");
        
        assert!(sandbox.track_open_file(path1.clone()).is_ok());
        assert!(sandbox.track_open_file(path2.clone()).is_ok());
        assert!(sandbox.track_open_file(path3.clone()).is_err()); // Too many
        
        sandbox.track_closed_file(&path1);
        assert!(sandbox.track_open_file(path3).is_ok());
    }

    #[test]
    fn test_fs_access_validation() {
        let sandbox = Sandbox::with_config(SandboxConfig {
            enable_filesystem: true,
            allowed_paths: vec![PathBuf::from("/tmp")],
            ..Default::default()
        });
        
        // Allowed path
        let result = sandbox.validate_fs_access(Path::new("/tmp/test.txt"), false);
        assert!(result.is_ok());
        
        // Disallowed path
        let result = sandbox.validate_fs_access(Path::new("/etc/passwd"), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_network_validation() {
        let sandbox = Sandbox::with_config(SandboxConfig {
            enable_networking: true,
            allowed_domains: vec!["api.github.com".to_string()],
            ..Default::default()
        });
        
        // Allowed
        assert!(sandbox.validate_network_access("api.github.com", None).is_ok());
        
        // Localhost always allowed
        assert!(sandbox.validate_network_access("localhost", None).is_ok());
        
        // Disallowed
        assert!(sandbox.validate_network_access("evil.com", None).is_err());
    }

    #[test]
    fn test_subprocess_validation() {
        let sandbox = Sandbox::with_config(SandboxConfig {
            enable_subprocesses: true,
            ..Default::default()
        });
        
        assert!(sandbox.validate_subprocess("git").is_ok());
        assert!(sandbox.validate_subprocess("node").is_ok());
        assert!(sandbox.validate_subprocess("malicious").is_err());
    }

    #[test]
    fn test_wasi_context_creation() {
        let sandbox = Sandbox::new();
        let manifest = ExtensionManifest {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            publisher: "test".to_string(),
            description: None,
            entry: "test.wasm".to_string(),
            engines: Default::default(),
            categories: vec![],
            tags: vec![],
            repository: None,
            homepage: None,
            license: None,
            icon: None,
            activation_events: vec![],
            contributes: Default::default(),
            capabilities: vec![],
            permissions: vec![],
            dependencies: vec![],
            dev_dependencies: vec![],
        };
        
        let ctx = sandbox.create_wasi_context(&manifest);
        assert!(ctx.is_ok());
    }
}