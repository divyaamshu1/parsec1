//! PTY process management

use crate::Result;
use std::io::ErrorKind;

/// PTY process
#[derive(Debug, Clone)]
pub struct PtyProcess {
    pid: u32,
}

/// PTY size (rows, columns)
#[derive(Debug, Clone)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
}

/// Events coming from the PTY stream
#[derive(Debug, Clone)]
pub enum PtyEvent {
    Data(Vec<u8>),
    Resize(u16, u16),
    Exit,
}

impl PtyProcess {
    /// Create new PTY process
    ///
    /// Arguments are currently ignored in the stub implementation.
    pub fn new(
        _command: impl Into<String>,
        _args: Vec<String>,
        _cwd: Option<String>,
        _env: Vec<(String, String)>,
    ) -> crate::Result<Self> {
        // stub: pretend we spawned a process with pid 0
        Ok(Self { pid: 0 })
    }

    /// Read from PTY
    pub async fn read(&mut self) -> Option<Vec<u8>> {
        // Stub: return None for now
        None
    }

    /// Write to PTY
    pub fn write(&mut self, _data: &[u8]) -> Result<()> {
        Ok(())
    }

    /// Resize PTY
    pub fn resize(&mut self, _rows: u16, _cols: u16) -> Result<()> {
        Ok(())
    }

    /// Kill PTY
    pub async fn kill(&mut self) -> Result<()> {
        Ok(())
    }
}
