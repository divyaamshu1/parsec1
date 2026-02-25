//! Git commit types
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Commit {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub summary: String,
    pub body: Option<String>,
    pub author_name: String,
    pub author_email: String,
    pub author_time: DateTime<Utc>,
    pub committer_name: String,
    pub committer_email: String,
    pub committer_time: DateTime<Utc>,
    pub parents: usize,
    pub parent_ids: Vec<String>,
    pub tree_id: String,
}

impl Commit {
    pub fn format_short(&self) -> String {
        format!("{} {}", self.short_id, self.summary)
    }
}