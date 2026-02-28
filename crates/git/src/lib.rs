//! Parsec git integration - small helper functions used by the workspace.

use anyhow::{Context, Result};
use git2::Repository;

/// Open a repository at `path`.
pub fn open_repo(path: &str) -> Result<Repository> {
    Repository::open(path).context("opening repository")
}

/// Return the current branch name for the repository at `path`.
pub fn current_branch(path: &str) -> Result<String> {
    let repo = open_repo(path)?;
    let head = repo.head().context("getting HEAD")?;
    let name = head.shorthand().map(|s| s.to_string()).unwrap_or_else(|| "HEAD".into());
    Ok(name)
}

/// Get the repository's remote URL for `origin` if present.
pub fn origin_url(path: &str) -> Result<Option<String>> {
    let repo = open_repo(path)?;
    match repo.find_remote("origin") {
        Ok(r) => Ok(r.url().map(|u| u.to_string())),
        Err(_) => Ok(None),
    }
}
