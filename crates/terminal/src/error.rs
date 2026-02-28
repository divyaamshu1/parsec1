//! Terminal error types

use thiserror::Error;

/// Terminal error type
#[derive(Debug, Error)]
pub enum TerminalError {
    #[error("PTY initialization failed: {0}")]
    PtyInitFailed(String),

    #[error("PTY I/O error: {0}")]
    PtyIo(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session already exists: {0}")]
    SessionExists(String),

    #[error("Invalid terminal size: rows={rows}, cols={cols}")]
    InvalidSize { rows: u16, cols: u16 },

    #[error("Invalid UTF-8 sequence")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Shell not found: {0}")]
    ShellNotFound(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Split error: {0}")]
    SplitError(String),

    #[error("Theme error: {0}")]
    ThemeError(String),

    #[error("Search error: {0}")]
    SearchError(String),

    #[cfg(unix)]
    #[error("Nix error: {0}")]
    Nix(#[from] nix::Error),

    #[error("Channel send error")]
    SendError,

    #[error("Channel receive error")]
    RecvError,

    #[error("Timeout")]
    Timeout,

    #[error("Terminal is dead")]
    DeadTerminal,

    #[error("Not supported on this platform")]
    UnsupportedPlatform,

    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for TerminalError {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        TerminalError::SendError
    }
}

impl From<tokio::sync::watch::error::SendError<()>> for TerminalError {
    fn from(_: tokio::sync::watch::error::SendError<()>) -> Self {
        TerminalError::SendError
    }
}

impl From<tokio::sync::broadcast::error::SendError<Vec<u8>>> for TerminalError {
    fn from(_: tokio::sync::broadcast::error::SendError<Vec<u8>>) -> Self {
        TerminalError::SendError
    }
}

impl From<futures::channel::mpsc::SendError> for TerminalError {
    fn from(_: futures::channel::mpsc::SendError) -> Self {
        TerminalError::SendError
    }
}

impl From<std::num::TryFromIntError> for TerminalError {
    fn from(_: std::num::TryFromIntError) -> Self {
        TerminalError::ParseError("Integer conversion error".to_string())
    }
}

// If serde_json is available via features, allow converting its Error into TerminalError
#[cfg(feature = "serde")]
impl From<serde_json::Error> for TerminalError {
    fn from(err: serde_json::Error) -> Self {
        TerminalError::Serialization(err.to_string())
    }
}