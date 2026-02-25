//! Git branch types

use super::Commit;

/// Git branch information
#[derive(Debug, Clone)]
pub struct Branch {
    pub name: String,
    pub full_name: String,
    pub is_remote: bool,
    pub is_current: bool,
    pub upstream: Option<String>,
    pub commit: Commit,
}

impl Branch {
    /// Get display name (with remote prefix if remote)
    pub fn display_name(&self) -> String {
        if self.is_remote {
            format!("remotes/{}", self.name)
        } else {
            self.name.clone()
        }
    }

    /// Get short name (without remote prefix)
    pub fn short_name(&self) -> String {
        if self.is_remote {
            self.name.split('/').last().unwrap_or(&self.name).to_string()
        } else {
            self.name.clone()
        }
    }

    /// Check if this is a main branch (main/master)
    pub fn is_main(&self) -> bool {
        self.name == "main" || self.name == "master"
    }

    /// Get ahead/behind counts relative to upstream
    pub fn ahead_behind(&self, _upstream_commit: Option<&Commit>) -> (usize, usize) {
        // This would need repository access to calculate
        (0, 0)
    }

    /// Format branch for display
    pub fn format(&self) -> String {
        let current_marker = if self.is_current { "* " } else { "  " };
        let remote_info = if let Some(up) = &self.upstream {
            format!(" -> {}", up)
        } else {
            String::new()
        };
        
        format!("{}{}{}", current_marker, self.name, remote_info)
    }
}

/// Remote branch information
#[derive(Debug, Clone)]
pub struct RemoteBranch {
    pub name: String,
    pub remote: String,
    pub commit: Commit,
}

impl RemoteBranch {
    /// Convert to regular branch
    pub fn to_branch(&self) -> Branch {
        Branch {
            name: self.name.clone(),
            full_name: format!("refs/remotes/{}/{}", self.remote, self.name),
            is_remote: true,
            is_current: false,
            upstream: None,
            commit: self.commit.clone(),
        }
    }
}

/// Branch comparison
#[derive(Debug, Clone)]
pub struct BranchComparison {
    pub branch: Branch,
    pub ahead: usize,
    pub behind: usize,
}

impl BranchComparison {
    /// Check if branch is up to date
    pub fn is_up_to_date(&self) -> bool {
        self.ahead == 0 && self.behind == 0
    }

    /// Check if branch is ahead
    pub fn is_ahead(&self) -> bool {
        self.ahead > 0
    }

    /// Check if branch is behind
    pub fn is_behind(&self) -> bool {
        self.behind > 0
    }

    /// Check if branch has diverged
    pub fn has_diverged(&self) -> bool {
        self.ahead > 0 && self.behind > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn mock_commit() -> Commit {
        Commit {
            id: "abc123".to_string(),
            short_id: "abc123".to_string(),
            message: "Test commit".to_string(),
            summary: "Test commit".to_string(),
            body: None,
            author_name: "Test".to_string(),
            author_email: "test@example.com".to_string(),
            author_time: Utc::now(),
            committer_name: "Test".to_string(),
            committer_email: "test@example.com".to_string(),
            committer_time: Utc::now(),
            parents: 0,
            parent_ids: vec![],
            tree_id: "tree123".to_string(),
        }
    }

    #[test]
    fn test_branch_creation() {
        let commit = mock_commit();
        let branch = Branch {
            name: "main".to_string(),
            full_name: "refs/heads/main".to_string(),
            is_remote: false,
            is_current: true,
            upstream: Some("origin/main".to_string()),
            commit,
        };
        
        assert_eq!(branch.display_name(), "main");
        assert_eq!(branch.short_name(), "main");
        assert!(branch.is_main());
    }

    #[test]
    fn test_remote_branch() {
        let commit = mock_commit();
        let branch = Branch {
            name: "origin/feature".to_string(),
            full_name: "refs/remotes/origin/feature".to_string(),
            is_remote: true,
            is_current: false,
            upstream: None,
            commit,
        };
        
        assert_eq!(branch.display_name(), "remotes/origin/feature");
        assert_eq!(branch.short_name(), "feature");
    }
}