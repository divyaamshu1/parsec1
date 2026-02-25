//! Git file status types

use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use git2::Status as Git2Status;

/// File status in Git
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatus {
    pub path: PathBuf,
    pub staged: bool,
    pub unstaged: bool,
    pub untracked: bool,
    pub deleted: bool,
    pub modified: bool,
    pub added: bool,
    pub renamed: bool,
    pub conflicted: bool,
    pub entry: StatusEntry,
}

/// Detailed status entry (like git status --short)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEntry {
    pub index_status: String,
    pub worktree_status: String,
}

impl FileStatus {
    /// Create from git2 status
    pub fn from_git_status(path: PathBuf, status: Git2Status) -> Self {
        Self {
            path,
            staged: Self::is_staged(status),
            unstaged: Self::is_unstaged(status),
            untracked: status.contains(Git2Status::WT_NEW),
            deleted: status.contains(Git2Status::WT_DELETED) || 
                     status.contains(Git2Status::INDEX_DELETED),
            modified: status.contains(Git2Status::WT_MODIFIED) ||
                      status.contains(Git2Status::INDEX_MODIFIED),
            added: status.contains(Git2Status::INDEX_NEW),
            renamed: status.contains(Git2Status::INDEX_RENAMED) ||
                     status.contains(Git2Status::WT_RENAMED),
            conflicted: status.is_conflicted(),
            entry: StatusEntry {
                index_status: Self::status_to_string(status, true),
                worktree_status: Self::status_to_string(status, false),
            },
        }
    }

    /// Check if status is staged
    fn is_staged(status: Git2Status) -> bool {
        status.intersects(
            Git2Status::INDEX_NEW |
            Git2Status::INDEX_MODIFIED |
            Git2Status::INDEX_DELETED |
            Git2Status::INDEX_RENAMED |
            Git2Status::INDEX_TYPECHANGE,
        )
    }

    /// Check if status is unstaged
    fn is_unstaged(status: Git2Status) -> bool {
        status.intersects(
            Git2Status::WT_MODIFIED |
            Git2Status::WT_DELETED |
            Git2Status::WT_RENAMED |
            Git2Status::WT_TYPECHANGE |
            Git2Status::WT_NEW,
        )
    }

    /// Convert status to string (like git status --short)
    fn status_to_string(status: Git2Status, index: bool) -> String {
        let s = if index {
            status & (Git2Status::INDEX_NEW |
                     Git2Status::INDEX_MODIFIED |
                     Git2Status::INDEX_DELETED |
                     Git2Status::INDEX_RENAMED |
                     Git2Status::INDEX_TYPECHANGE)
        } else {
            status & (Git2Status::WT_NEW |
                     Git2Status::WT_MODIFIED |
                     Git2Status::WT_DELETED |
                     Git2Status::WT_RENAMED |
                     Git2Status::WT_TYPECHANGE)
        };

        if s.contains(Git2Status::INDEX_NEW) || s.contains(Git2Status::WT_NEW) {
            "A".to_string()
        } else if s.contains(Git2Status::INDEX_MODIFIED) || s.contains(Git2Status::WT_MODIFIED) {
            "M".to_string()
        } else if s.contains(Git2Status::INDEX_DELETED) || s.contains(Git2Status::WT_DELETED) {
            "D".to_string()
        } else if s.contains(Git2Status::INDEX_RENAMED) || s.contains(Git2Status::WT_RENAMED) {
            "R".to_string()
        } else if s.contains(Git2Status::INDEX_TYPECHANGE) || s.contains(Git2Status::WT_TYPECHANGE) {
            "T".to_string()
        } else {
            " ".to_string()
        }
    }

    /// Get short status string (like ' M' or 'A ')
    pub fn short_status(&self) -> String {
        format!("{}{}", self.entry.index_status, self.entry.worktree_status)
    }

    /// Get color for status (for UI)
    pub fn status_color(&self) -> &'static str {
        if self.conflicted {
            "red"
        } else if self.staged {
            "green"
        } else if self.unstaged {
            "yellow"
        } else if self.untracked {
            "cyan"
        } else {
            "white"
        }
    }

    /// Get icon for status (for UI)
    pub fn status_icon(&self) -> &'static str {
        if self.conflicted {
            "⚠"
        } else if self.staged {
            "✓"
        } else if self.unstaged {
            "✗"
        } else if self.untracked {
            "?"
        } else {
            " "
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Status;

    #[test]
    fn test_status_creation() {
        let status = FileStatus::from_git_status(
            PathBuf::from("test.txt"),
            Status::WT_MODIFIED,
        );
        
        assert!(!status.staged);
        assert!(status.unstaged);
        assert_eq!(status.short_status(), " M");
    }
}