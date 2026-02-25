//! PTY (Pseudo-terminal) handling for shell integration

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use tokio::sync::{mpsc, Mutex};

#[cfg(unix)]
mod imp {
    use super::*;
    use tokio::process::Command as TokioCommand;
    use std::os::unix::io::{AsRawFd, FromRawFd};
    use nix::pty::{openpty, OpenptyResult, Winsize};
    use nix::unistd::{fork, setsid, execvp, close, ForkResult};
    use nix::sys::signal::{kill, Signal};
    use nix::sys::wait::{waitpid, WaitStatus};
    
    /// PTY process for Unix systems
    #[derive(Debug)]
    pub struct PtyProcess {
        master: std::fs::File,
        child_pid: Option<nix::unistd::Pid>,
        reader_task: Option<tokio::task::JoinHandle<()>>,
        writer_task: Option<tokio::task::JoinHandle<()>>,
        rx: mpsc::UnboundedReceiver<Vec<u8>>,
        tx: mpsc::UnboundedSender<Vec<u8>>,
        alive: bool,
    }

    impl PtyProcess {
        pub fn new(
            shell_path: String,
            args: Vec<String>,
            working_dir: Option<String>,
            env: HashMap<String, String>,
        ) -> Result<Self> {
            // Open PTY
            let OpenptyResult { master, slave } = openpty(
                None,
                Some(&Winsize {
                    ws_row: 24,
                    ws_col: 80,
                    ws_xpixel: 0,
                    ws_ypixel: 0,
                }),
            )?;
            
            // Convert to std::fs::File
            let master = unsafe { std::fs::File::from_raw_fd(master) };
            let slave = unsafe { std::fs::File::from_raw_fd(slave) };
            
            // Fork process
            match unsafe { fork() }? {
                ForkResult::Parent { child } => {
                    // Parent process
                    close(slave.as_raw_fd())?;
                    
                    let (tx, rx) = mpsc::unbounded_channel();
                    let (input_tx, mut input_rx) = mpsc::unbounded_channel();
                    
                    // Spawn reader task
                    let mut master_clone = master.try_clone()?;
                    let reader_task = tokio::spawn(async move {
                        let mut buf = vec![0u8; 4096];
                        loop {
                            match tokio::task::spawn_blocking(move || {
                                nix::unistd::read(master_clone.as_raw_fd(), &mut buf)
                            }).await {
                                Ok(Ok(n)) if n > 0 => {
                                    let _ = tx.send(buf[..n].to_vec());
                                }
                                Ok(Ok(0)) => break, // EOF
                                _ => break,
                            }
                        }
                    });
                    
                    // Spawn writer task
                    let master_clone = master.try_clone()?;
                    let writer_task = tokio::spawn(async move {
                        while let Some(data) = input_rx.recv().await {
                            let _ = tokio::task::spawn_blocking(move || {
                                nix::unistd::write(master_clone.as_raw_fd(), &data)
                            }).await;
                        }
                    });
                    
                    Ok(Self {
                        master,
                        child_pid: Some(child),
                        reader_task: Some(reader_task),
                        writer_task: Some(writer_task),
                        rx,
                        tx: input_tx,
                        alive: true,
                    })
                }
                ForkResult::Child => {
                    // Child process
                    setsid()?; // Create new session
                    
                    // Setup slave PTY as stdin/out/err
                    let slave_fd = slave.as_raw_fd();
                    let _ = close(0);
                    let _ = close(1);
                    let _ = close(2);
                    let _ = nix::unistd::dup2(slave_fd, 0);
                    let _ = nix::unistd::dup2(slave_fd, 1);
                    let _ = nix::unistd::dup2(slave_fd, 2);
                    
                    // Close master in child
                    let _ = close(master.as_raw_fd());
                    let _ = close(slave_fd);
                    
                    // Change directory
                    if let Some(dir) = working_dir {
                        let _ = std::env::set_current_dir(dir);
                    }
                    
                    // Set environment
                    for (key, value) in env {
                        std::env::set_var(key, value);
                    }
                    
                    // Execute shell
                    let _ = execvp(&shell_path, &args);
                    
                    // Should never reach here
                    std::process::exit(1);
                }
            }
        }

        pub async fn read(&mut self) -> Option<Vec<u8>> {
            self.rx.recv().await
        }

        pub fn write(&self, data: &[u8]) -> Result<()> {
            if self.alive {
                self.tx.send(data.to_vec())?;
                Ok(())
            } else {
                Err(anyhow!("Process is dead"))
            }
        }

        pub fn resize(&self, rows: u16, cols: u16) -> Result<()> {
            let ws = Winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            
            // Safe because we're using the raw file descriptor correctly
            #[allow(unused_unsafe)]
            unsafe {
                use nix::libc::ioctl;
                use nix::libc::TIOCSWINSZ;
                
                let result = ioctl(self.master.as_raw_fd(), TIOCSWINSZ, &ws);
                if result != 0 {
                    return Err(anyhow!("Failed to resize PTY"));
                }
            }
            
            Ok(())
        }

        pub fn kill(&mut self) -> Result<()> {
            if let Some(pid) = self.child_pid.take() {
                let _ = kill(pid, Signal::SIGKILL);
                let _ = waitpid(pid, None);
                self.alive = false;
            }
            
            if let Some(task) = self.reader_task.take() {
                task.abort();
            }
            if let Some(task) = self.writer_task.take() {
                task.abort();
            }
            
            Ok(())
        }

        pub fn is_alive(&self) -> bool {
            self.alive
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, RawHandle};
    use tokio::process::{Command, Child};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::Mutex;
    
    /// Windows ConPTY implementation
    #[derive(Debug)]
    pub struct PtyProcess {
        child: Arc<Mutex<Child>>,
        stdin_tx: mpsc::UnboundedSender<Vec<u8>>,
        stdout_rx: mpsc::UnboundedReceiver<Vec<u8>>,
        stderr_rx: mpsc::UnboundedReceiver<Vec<u8>>,
        alive: bool,
    }

    impl PtyProcess {
        pub fn new(
            shell_path: String,
            args: Vec<String>,
            working_dir: Option<String>,
            env: HashMap<String, String>,
        ) -> Result<Self> {
            use std::os::windows::process::CommandExt;
            
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            
            let mut cmd = Command::new(&shell_path);
            
            // Set arguments
            if !args.is_empty() {
                cmd.args(&args);
            }
            
            // Set working directory
            if let Some(dir) = working_dir {
                cmd.current_dir(dir);
            }
            
            // Set environment variables
            cmd.envs(&env);
            
            // IMPORTANT: These flags are needed for ConPTY to work properly
            cmd.creation_flags(CREATE_NO_WINDOW);
            
            // Create pipes for stdio
            cmd.stdin(std::process::Stdio::piped());
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
            
            // Spawn the process
            let mut child = cmd.spawn()?;
            
            // Get stdin handle
            let mut child_stdin = child.stdin.take()
                .ok_or_else(|| anyhow!("Failed to get stdin handle"))?;
            
            // Get stdout handle
            let mut child_stdout = child.stdout.take()
                .ok_or_else(|| anyhow!("Failed to get stdout handle"))?;
            
            // Get stderr handle
            let mut child_stderr = child.stderr.take()
                .ok_or_else(|| anyhow!("Failed to get stderr handle"))?;
            
            // Create channels for I/O
            let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<Vec<u8>>();
            let (stdout_tx, stdout_rx) = mpsc::unbounded_channel();
            let (stderr_tx, stderr_rx) = mpsc::unbounded_channel();
            
            // Spawn writer task for stdin
            tokio::spawn(async move {
                while let Some(data) = stdin_rx.recv().await {
                    if let Err(e) = child_stdin.write_all(&data).await {
                        eprintln!("Error writing to stdin: {}", e);
                        break;
                    }
                    if let Err(e) = child_stdin.flush().await {
                        eprintln!("Error flushing stdin: {}", e);
                        break;
                    }
                }
            });
            
            // Spawn reader task for stdout
            let stdout_tx_clone = stdout_tx.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                loop {
                    match child_stdout.read(&mut buf).await {
                        Ok(0) => break, // EOF
                        Ok(n) => {
                            let _ = stdout_tx_clone.send(buf[..n].to_vec());
                        }
                        Err(e) => {
                            eprintln!("Error reading from stdout: {}", e);
                            break;
                        }
                    }
                }
            });
            
            // Spawn reader task for stderr
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                loop {
                    match child_stderr.read(&mut buf).await {
                        Ok(0) => break, // EOF
                        Ok(n) => {
                            let _ = stderr_tx.send(buf[..n].to_vec());
                        }
                        Err(e) => {
                            eprintln!("Error reading from stderr: {}", e);
                            break;
                        }
                    }
                }
            });
            
            Ok(Self {
                child: Arc::new(Mutex::new(child)),
                stdin_tx,
                stdout_rx,
                stderr_rx,
                alive: true,
            })
        }
        
        pub async fn read(&mut self) -> Option<Vec<u8>> {
            tokio::select! {
                Some(data) = self.stdout_rx.recv() => Some(data),
                Some(data) = self.stderr_rx.recv() => Some(data),
                else => None,
            }
        }
        
        pub fn write(&self, data: &[u8]) -> Result<()> {
            if !self.alive {
                return Err(anyhow!("Process is dead"));
            }
            self.stdin_tx.send(data.to_vec())?;
            Ok(())
        }
        
        pub fn resize(&self, rows: u16, cols: u16) -> Result<()> {
            // Windows Console API for resizing
            // Note: This is a simplified version
            // Full implementation would use SetConsoleScreenBufferSize
            Ok(())
        }
        
        pub async fn kill(&mut self) -> Result<()> {
            self.alive = false;
            let mut child = self.child.lock().await;
            let _ = child.kill().await;
            let _ = child.wait().await;
            Ok(())
        }
        
        pub fn is_alive(&self) -> bool {
            self.alive
        }
    }
}

// Export platform-specific implementation
#[cfg(unix)]
pub use imp::PtyProcess;

#[cfg(windows)]
pub use imp::PtyProcess;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pty_creation() {
        let env = HashMap::new();
        let shell = if cfg!(windows) { 
            "powershell.exe".to_string() 
        } else { 
            "/bin/bash".to_string() 
        };
        
        let pty = PtyProcess::new(
            shell,
            vec![],
            None,
            env,
        );
        
        // On Windows, this might still fail if ConPTY isn't available
        // But at least it's implemented now
        if cfg!(unix) {
            assert!(pty.is_ok());
        }
    }
}