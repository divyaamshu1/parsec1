//! Git repository management

use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow};
use git2::{
    Repository as Git2Repo, Branch as Git2Branch, BranchType,
    Commit as Git2Commit, StatusOptions,
    Signature, Oid, PushOptions, FetchOptions, Cred,
    RemoteCallbacks, AutotagOption
};
use chrono::{DateTime, Utc};
use super::{Branch, Commit, FileStatus, Remote, Stash};

/// Git repository wrapper
pub struct Repository {
    /// Path to repository
    path: PathBuf,
    /// Git2 repository
    repo: Git2Repo,
    /// Cache for performance
    cache: RepoCache,
}

/// Repository cache
struct RepoCache {
    branches: Option<Vec<Branch>>,
    commits: Vec<Commit>,
    last_fetch: Option<DateTime<Utc>>,
    last_push: Option<DateTime<Utc>>,
}

impl Repository {
    /// Open an existing repository
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let repo = Git2Repo::discover(path.as_ref())?;
        let path = repo.path().parent()
            .ok_or_else(|| anyhow!("Invalid repository path"))?
            .to_path_buf();
        
        Ok(Self {
            path,
            repo,
            cache: RepoCache {
                branches: None,
                commits: Vec::new(),
                last_fetch: None,
                last_push: None,
            },
        })
    }

    /// Initialize a new repository
    pub fn init<P: AsRef<Path>>(path: P, bare: bool) -> Result<Self> {
        let path = path.as_ref();
        let repo = if bare {
            Git2Repo::init_bare(path)?
        } else {
            Git2Repo::init(path)?
        };
        
        Ok(Self {
            path: path.to_path_buf(),
            repo,
            cache: RepoCache {
                branches: None,
                commits: Vec::new(),
                last_fetch: None,
                last_push: None,
            },
        })
    }

    /// Get repository path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get repository status
    pub fn status(&self) -> Result<Vec<FileStatus>> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .renames_head_to_index(true)
            .renames_index_to_workdir(true);
        
        let statuses = self.repo.statuses(Some(&mut opts))?;
        let mut files = Vec::new();
        
        for entry in statuses.iter() {
            let path = entry.path()
                .map(|p| self.path.join(p))
                .unwrap_or_else(|| self.path.clone());
            
            let status = entry.status();
            files.push(FileStatus::from_git_status(path, status));
        }
        
        Ok(files)
    }

    /// Stage files
    pub fn stage(&mut self, paths: &[PathBuf]) -> Result<()> {
        let mut index = self.repo.index()?;
        
        for path in paths {
            let rel_path = path.strip_prefix(&self.path)?;
            index.add_path(rel_path)?;
        }
        
        index.write()?;
        
        Ok(())
    }

    /// Unstage files
    pub fn unstage(&mut self, paths: &[PathBuf]) -> Result<()> {
        let mut index = self.repo.index()?;

        for path in paths {
            let rel_path = path.strip_prefix(&self.path)?;
            index.remove_path(rel_path)?;
            
            // Restore from HEAD if it exists
            if let Ok(head) = self.repo.head() {
                if let Ok(head_commit) = head.peel_to_commit() {
                    if let Ok(head_tree) = head_commit.tree() {
                        let _entry = head_tree.get_path(rel_path);
                        index.add_path(rel_path)?;
                    }
                }
            }
        }
        
        index.write()?;
        
        Ok(())
    }

    /// Commit changes
    pub fn commit(
        &mut self,
        message: &str,
        amend: bool,
        user_name: Option<&str>,
        user_email: Option<&str>,
    ) -> Result<Commit> {
        let mut index = self.repo.index()?;
        let tree_oid = index.write_tree()?;
        let tree = self.repo.find_tree(tree_oid)?;
        
        let parent_commits = if amend {
            let head = self.repo.head()?;
            let head_commit = head.peel_to_commit()?;
            vec![head_commit]
        } else {
            let head = self.repo.head()?;
            if head.name().is_some() {
                let head_commit = head.peel_to_commit()?;
                vec![head_commit]
            } else {
                vec![]
            }
        };
        
        let name = user_name.unwrap_or("Unknown");
        let email = user_email.unwrap_or("unknown@example.com");
        let signature = Signature::now(name, email)?;
        
        let commit_oid = if amend {
            let head = self.repo.head()?;
            let head_commit = head.peel_to_commit()?;
            self.repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &[&head_commit],
            )?
        } else {
            let parents: Vec<&Git2Commit> = parent_commits.iter().collect();
            self.repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &parents,
            )?
        };
        
        let commit = self.repo.find_commit(commit_oid)?;
        self.commit_from_git2(&commit)
    }

    /// Get current branch
    pub fn current_branch(&self) -> Option<String> {
        self.repo.head().ok()
            .and_then(|head| head.shorthand().map(|s| s.to_string()))
    }

    /// Get all branches
    pub fn branches(&self) -> Result<Vec<Branch>> {
        let mut branches = Vec::new();
        
        // Local branches
        for branch_result in self.repo.branches(Some(BranchType::Local))? {
            let (branch, _) = branch_result?;
            if let Some(b) = self.branch_from_git2(&branch) {
                branches.push(b);
            }
        }
        
        // Remote branches
        for branch_result in self.repo.branches(Some(BranchType::Remote))? {
            let (branch, _) = branch_result?;
            if let Some(b) = self.branch_from_git2(&branch) {
                branches.push(b);
            }
        }
        
        Ok(branches)
    }

    /// Create branch
    pub fn create_branch(&mut self, name: &str, checkout: bool) -> Result<Branch> {
        let head = self.repo.head()?;
        let head_commit = head.peel_to_commit()?;
        
        let branch = self.repo.branch(name, &head_commit, false)?;
        
        if checkout {
            self.repo.set_head(&format!("refs/heads/{}", name))?;
            self.repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;
        }
        
        self.branch_from_git2(&branch)
            .ok_or_else(|| anyhow!("Failed to create branch"))
    }

    /// Checkout branch
    pub fn checkout_branch(&mut self, name: &str) -> Result<()> {
        let treeish = self.repo.revparse_single(&format!("refs/heads/{}", name))?;
        let object = treeish.peel(git2::ObjectType::Commit)?;
        self.repo.set_head(&format!("refs/heads/{}", name))?;
        self.repo.checkout_tree(&object, Some(git2::build::CheckoutBuilder::new().safe()))?;
        
        Ok(())
    }

    /// Delete branch
    pub fn delete_branch(&mut self, name: &str, force: bool) -> Result<()> {
        let mut branch = self.repo.find_branch(name, BranchType::Local)?;
        
        if force {
            branch.delete()?;
        } else {
            let head = self.repo.head()?;
            let head_commit = head.peel_to_commit()?;
            let branch_commit = branch.get().peel_to_commit()?;
            
            if self.repo.graph_descendant_of(head_commit.id(), branch_commit.id())? {
                branch.delete()?;
            } else {
                return Err(anyhow!("Branch not fully merged"));
            }
        }
        
        Ok(())
    }

    /// Get commit history
    pub fn commits(&self, limit: usize) -> Result<Vec<Commit>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;
        
        let mut commits = Vec::new();
        for oid in revwalk.take(limit) {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            commits.push(self.commit_from_git2(&commit)?);
        }
        
        Ok(commits)
    }

    /// Get specific commit
    pub fn get_commit(&self, id: &str) -> Result<Commit> {
        let oid = Oid::from_str(id)?;
        let commit = self.repo.find_commit(oid)?;
        self.commit_from_git2(&commit)
    }

    /// Convert git2 commit to our Commit type
    fn commit_from_git2(&self, commit: &Git2Commit) -> Result<Commit> {
        let author = commit.author();
        let committer = commit.committer();
        
        Ok(Commit {
            id: commit.id().to_string(),
            short_id: commit.id().to_string()[..7].to_string(),
            message: commit.message().unwrap_or("").to_string(),
            summary: commit.summary().unwrap_or("").to_string(),
            body: commit.body().map(|s| s.to_string()),
            author_name: author.name().unwrap_or("").to_string(),
            author_email: author.email().unwrap_or("").to_string(),
            author_time: DateTime::from_timestamp(author.when().seconds(), 0).unwrap_or(Utc::now()),
            committer_name: committer.name().unwrap_or("").to_string(),
            committer_email: committer.email().unwrap_or("").to_string(),
            committer_time: DateTime::from_timestamp(committer.when().seconds(), 0).unwrap_or(Utc::now()),
            parents: commit.parent_count(),
            parent_ids: commit.parent_ids().map(|p| p.to_string()).collect(),
            tree_id: commit.tree_id().to_string(),
        })
    }

    /// Convert git2 branch to our Branch type
    fn branch_from_git2(&self, branch: &Git2Branch) -> Option<Branch> {
        let name = branch.name().ok().flatten()?;
        let upstream = branch.upstream().ok()
            .and_then(|b| b.name().ok().flatten().map(|s| s.to_string()));
        
        let commit = branch.get().peel_to_commit().ok()?;
        let commit_info = self.commit_from_git2(&commit).ok()?;
        
        let is_current = if let Ok(head) = self.repo.head() {
            if let Some(head_name) = head.shorthand() {
                head_name == name
            } else {
                false
            }
        } else {
            false
        };
        
        Some(Branch {
            name: name.to_string(),
            full_name: format!("refs/heads/{}", name),
            is_remote: false,
            is_current,
            upstream,
            commit: commit_info,
        })
    }

    /// Push to remote
    pub fn push(&self, remote_name: &str, branch: &str, force: bool) -> Result<()> {
        let mut remote = self.repo.find_remote(remote_name)?;
        
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });
        
        let mut push_opts = PushOptions::new();
        push_opts.remote_callbacks(callbacks);
        
        let refspec = if force {
            format!("+refs/heads/{}:refs/heads/{}", branch, branch)
        } else {
            format!("refs/heads/{}:refs/heads/{}", branch, branch)
        };
        
        remote.push(&[&refspec], Some(&mut push_opts))?;
        
        Ok(())
    }

    /// Pull from remote
    pub fn pull(&mut self, remote_name: &str, branch: &str, rebase: bool) -> Result<()> {
        // Fetch first
        self.fetch(remote_name, true)?;
        
        let remote_branch = self.repo.find_branch(
            &format!("{}/{}", remote_name, branch),
            BranchType::Remote,
        )?;
        
        let remote_commit = remote_branch.get().peel_to_commit()?;
        
        if rebase {
            let head = self.repo.head()?;
            let head_commit = head.peel_to_commit()?;
            
            let mut rebase_opts = git2::RebaseOptions::new();
            rebase_opts.inmemory(false);
            
            // Create annotated commits using lookup with OIDs
            let head_annotated = self.repo.find_annotated_commit(head_commit.id())?;
            let remote_annotated = self.repo.find_annotated_commit(remote_commit.id())?;
            
            let mut rebase = self.repo.rebase(
                Some(&head_annotated),
                Some(&head_annotated),
                Some(&remote_annotated),
                Some(&mut rebase_opts),
            )?;
            
            while let Some(op) = rebase.next() {
                match op {
                    Ok(_) => {
                        if rebase.inmemory_index().is_ok() {
                            return Err(anyhow!("Merge conflict during rebase"));
                        }
                        rebase.commit(None, &git2::Signature::now("Parsec", "parsec@local")?, None)?;
                    }
                    Err(e) => return Err(anyhow!("Rebase failed: {}", e)),
                }
            }
            
            rebase.finish(None)?;
        } else {
            let head = self.repo.head()?;
            let head_commit = head.peel_to_commit()?;
            
            let _merge_base = self.repo.merge_base(head_commit.id(), remote_commit.id())?;
            
            // Create annotated commit for remote
            let remote_annotated = self.repo.find_annotated_commit(remote_commit.id())?;
            
            self.repo.merge(&[&remote_annotated], None, None)?;
            
            if self.repo.index()?.has_conflicts() {
                return Err(anyhow!("Merge conflicts detected"));
            }
            
            let tree_id = self.repo.index()?.write_tree_to(&self.repo)?;
            let tree = self.repo.find_tree(tree_id)?;
            
            let signature = git2::Signature::now("Parsec", "parsec@local")?;
            
            self.repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                &format!("Merge branch '{}' of {}", branch, remote_name),
                &tree,
                &[&head_commit, &remote_commit],
            )?;
        }
        
        Ok(())
    }

    /// Fetch from remote
    pub fn fetch(&self, remote_name: &str, prune: bool) -> Result<usize> {
        let mut remote = self.repo.find_remote(remote_name)?;
        
        let mut fetch_opts = FetchOptions::new();
        fetch_opts.download_tags(AutotagOption::All);
        
        if prune {
            fetch_opts.prune(git2::FetchPrune::On);
        }
        
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });
        fetch_opts.remote_callbacks(callbacks);
        
        remote.fetch(&["refs/heads/*:refs/heads/*"], Some(&mut fetch_opts), None)?;
        
        Ok(remote.stats().received_objects())
    }

    /// Get remotes
    pub fn remotes(&self) -> Result<Vec<Remote>> {
        let mut remotes = Vec::new();
        
        for name in self.repo.remotes()?.iter() {
            if let Some(name) = name {
                if let Ok(remote) = self.repo.find_remote(name) {
                    remotes.push(Remote {
                        name: name.to_string(),
                        url: remote.url().unwrap_or("").to_string(),
                        push_url: remote.pushurl().map(|s| s.to_string()),
                    });
                }
            }
        }
        
        Ok(remotes)
    }

    /// Add remote
    pub fn add_remote(&self, name: &str, url: &str) -> Result<()> {
        self.repo.remote(name, url)?;
        Ok(())
    }

    /// Remove remote
    pub fn remove_remote(&self, name: &str) -> Result<()> {
        self.repo.remote_delete(name)?;
        Ok(())
    }

    /// Create stash
    pub fn stash_create(&mut self, message: Option<&str>) -> Result<Stash> {
        let message = message.unwrap_or("");
        let signature = git2::Signature::now("Parsec", "parsec@local")?;
        
        let stash_id = self.repo.stash_save2(&signature, Some(message), None)?;
        
        Ok(Stash {
            id: stash_id.to_string(),
            index: 0,
            message: message.to_string(),
            branch: self.current_branch().unwrap_or_default(),
            commit: self.get_commit(&stash_id.to_string())?,
        })
    }

    /// Apply stash
    pub fn stash_apply(&mut self, index: usize) -> Result<()> {
        self.repo.stash_apply(index as usize, None)?;
        Ok(())
    }

    /// Drop stash
    pub fn stash_drop(&mut self, index: usize) -> Result<()> {
        self.repo.stash_drop(index as usize)?;
        Ok(())
    }

    /// Get stashes - now takes &mut self because stash_foreach requires mutable access
    pub fn stashes(&mut self) -> Result<Vec<Stash>> {
        let mut stashes = Vec::new();
        let mut stash_entries = Vec::new();
        
        // Collect stash information first
        self.repo.stash_foreach(|idx, message, oid| {
            stash_entries.push((idx, message.to_string(), *oid));
            true
        })?;
        
        // Then process each stash
        for (idx, message, oid) in stash_entries {
            if let Ok(commit) = self.repo.find_commit(oid) {
                if let Ok(commit_info) = self.commit_from_git2(&commit) {
                    stashes.push(Stash {
                        id: oid.to_string(),
                        index: idx as usize,
                        message,
                        branch: "unknown".to_string(),
                        commit: commit_info,
                    });
                }
            }
        }
        
        Ok(stashes)
    }

    /// Get commits ahead of upstream
    pub fn commits_ahead(&self) -> usize {
        if let (Ok(head), Ok(upstream)) = (self.repo.head(), self.upstream_branch()) {
            if let (Ok(head_commit), Ok(upstream_commit)) = (
                head.peel_to_commit(),
                upstream.get().peel_to_commit(),
            ) {
                if let Ok((ahead, _)) = self.repo.graph_ahead_behind(
                    head_commit.id(),
                    upstream_commit.id(),
                ) {
                    return ahead as usize;
                }
            }
        }
        0
    }

    /// Get commits behind upstream
    pub fn commits_behind(&self) -> usize {
        if let (Ok(head), Ok(upstream)) = (self.repo.head(), self.upstream_branch()) {
            if let (Ok(head_commit), Ok(upstream_commit)) = (
                head.peel_to_commit(),
                upstream.get().peel_to_commit(),
            ) {
                if let Ok((_, behind)) = self.repo.graph_ahead_behind(
                    head_commit.id(),
                    upstream_commit.id(),
                ) {
                    return behind as usize;
                }
            }
        }
        0
    }

    /// Get upstream branch for current branch
    fn upstream_branch(&self) -> Result<git2::Branch> {
        let head = self.repo.head()?;
        if let Some(branch_name) = head.shorthand() {
            if let Ok(branch) = self.repo.find_branch(branch_name, BranchType::Local) {
                if let Ok(upstream) = branch.upstream() {
                    return Ok(upstream);
                }
            }
        }
        Err(anyhow!("No upstream branch"))
    }
}

// Repository wraps non-Sync types from libgit2; for the purposes of the
// application we mark it as Send+Sync so it can be stored behind async
// locks. This is safe here because access to the underlying libgit2
// repository is serialized by the API usage elsewhere in the project.
unsafe impl Send for Repository {}
unsafe impl Sync for Repository {}