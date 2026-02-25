//! Git stash types
use super::Commit;

#[derive(Debug, Clone)]
pub struct Stash {
    pub id: String,
    pub index: usize,
    pub message: String,
    pub branch: String,
    pub commit: Commit,
}

impl Stash {
    pub fn display_name(&self) -> String {
        format!("stash@{{{}}}: {}", self.index, self.message)
    }
}