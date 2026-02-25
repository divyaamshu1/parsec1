//! Git remote types
#[derive(Debug, Clone)]
pub struct Remote {
    pub name: String,
    pub url: String,
    pub push_url: Option<String>,
}

impl Remote {
    pub fn is_github(&self) -> bool {
        self.url.contains("github.com")
    }
    pub fn is_gitlab(&self) -> bool {
        self.url.contains("gitlab.com")
    }
}