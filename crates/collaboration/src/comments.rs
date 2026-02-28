//! Comments and discussions

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::{UserId, DocumentId, CommentId, Range, Result};

/// Comment severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommentSeverity {
    Info,
    Question,
    Suggestion,
    Warning,
    Error,
    Critical,
}

/// Comment status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommentStatus {
    Open,
    Resolved,
    Closed,
    Reopened,
}

/// Comment reply
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentReply {
    pub id: CommentId,
    pub content: String,
    pub author: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub mentions: Vec<UserId>,
}

/// Comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: CommentId,
    pub document_id: DocumentId,
    pub thread_id: CommentId,
    pub parent_id: Option<CommentId>,
    pub content: String,
    pub author: UserId,
    pub range: Option<Range>,
    pub severity: CommentSeverity,
    pub status: CommentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<UserId>,
    pub mentions: Vec<UserId>,
    pub labels: Vec<String>,
    pub attachments: Vec<String>,
    pub replies: Vec<CommentReply>,
}

/// Comment thread
#[derive(Debug, Clone)]
pub struct CommentThread {
    pub id: CommentId,
    pub document_id: DocumentId,
    pub title: String,
    pub comments: Vec<Comment>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub participants: Vec<UserId>,
}

/// Comment manager
pub struct CommentManager {
    threads: Arc<RwLock<HashMap<CommentId, CommentThread>>>,
    document_threads: Arc<RwLock<HashMap<DocumentId, Vec<CommentId>>>>,
}

impl CommentManager {
    /// Create new comment manager
    pub fn new() -> Self {
        Self {
            threads: Arc::new(RwLock::new(HashMap::new())),
            document_threads: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create new comment thread
    pub async fn create_thread(
        &self,
        document_id: DocumentId,
        title: String,
        first_comment: String,
        author: UserId,
        range: Option<Range>,
    ) -> Result<CommentThread> {
        let thread_id = CommentId::new();
        let comment_id = CommentId::new();

        let comment = Comment {
            id: comment_id.clone(),
            document_id: document_id.clone(),
            thread_id: thread_id.clone(),
            parent_id: None,
            content: first_comment,
            author: author.clone(),
            range,
            severity: CommentSeverity::Info,
            status: CommentStatus::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            resolved_at: None,
            resolved_by: None,
            mentions: vec![],
            labels: vec![],
            attachments: vec![],
            replies: vec![],
        };

        let thread = CommentThread {
            id: thread_id.clone(),
            document_id: document_id.clone(),
            title,
            comments: vec![comment],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            participants: vec![author],
        };

        self.threads.write().await.insert(thread_id.clone(), thread.clone());
        
        let mut doc_threads = self.document_threads.write().await;
        doc_threads.entry(document_id).or_insert_with(Vec::new).push(thread_id);

        Ok(thread)
    }

    /// Add comment to thread
    pub async fn add_comment(
        &self,
        thread_id: &CommentId,
        content: String,
        author: UserId,
        parent_id: Option<CommentId>,
    ) -> Result<Comment> {
        let mut threads = self.threads.write().await;
        let thread = threads.get_mut(thread_id)
            .ok_or_else(|| crate::CollaborationError::SessionNotFound(thread_id.to_string()))?;

        let comment_id = CommentId::new();
        let comment = Comment {
            id: comment_id.clone(),
            document_id: thread.document_id.clone(),
            thread_id: thread_id.clone(),
            parent_id,
            content,
            author: author.clone(),
            range: None,
            severity: CommentSeverity::Info,
            status: CommentStatus::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            resolved_at: None,
            resolved_by: None,
            mentions: vec![],
            labels: vec![],
            attachments: vec![],
            replies: vec![],
        };

        thread.comments.push(comment.clone());
        thread.updated_at = Utc::now();
        if !thread.participants.contains(&author) {
            thread.participants.push(author);
        }

        Ok(comment)
    }

    /// Add reply to comment
    pub async fn add_reply(
        &self,
        thread_id: &CommentId,
        comment_id: &CommentId,
        content: String,
        author: UserId,
    ) -> Result<CommentReply> {
        let mut threads = self.threads.write().await;
        let thread = threads.get_mut(thread_id)
            .ok_or_else(|| crate::CollaborationError::SessionNotFound(thread_id.to_string()))?;

        let comment = thread.comments.iter_mut()
            .find(|c| c.id == *comment_id)
            .ok_or_else(|| crate::CollaborationError::SessionNotFound(comment_id.to_string()))?;

        let reply = CommentReply {
            id: CommentId::new(),
            content,
            author,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            mentions: vec![],
        };

        comment.replies.push(reply.clone());
        comment.updated_at = Utc::now();
        thread.updated_at = Utc::now();

        Ok(reply)
    }

    /// Resolve comment
    pub async fn resolve_comment(
        &self,
        thread_id: &CommentId,
        comment_id: &CommentId,
        resolved_by: UserId,
    ) -> Result<()> {
        let mut threads = self.threads.write().await;
        let thread = threads.get_mut(thread_id)
            .ok_or_else(|| crate::CollaborationError::SessionNotFound(thread_id.to_string()))?;

        let comment = thread.comments.iter_mut()
            .find(|c| c.id == *comment_id)
            .ok_or_else(|| crate::CollaborationError::SessionNotFound(comment_id.to_string()))?;

        comment.status = CommentStatus::Resolved;
        comment.resolved_at = Some(Utc::now());
        comment.resolved_by = Some(resolved_by);
        comment.updated_at = Utc::now();
        thread.updated_at = Utc::now();

        Ok(())
    }

    /// Get thread
    pub async fn get_thread(&self, thread_id: &CommentId) -> Option<CommentThread> {
        self.threads.read().await.get(thread_id).cloned()
    }

    /// Get threads for document
    pub async fn get_document_threads(&self, document_id: &DocumentId) -> Vec<CommentThread> {
        let threads = self.threads.read().await;
        let doc_threads = self.document_threads.read().await;
        
        if let Some(ids) = doc_threads.get(document_id) {
            ids.iter()
                .filter_map(|id| threads.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all threads
    pub async fn get_all_threads(&self) -> Vec<CommentThread> {
        self.threads.read().await.values().cloned().collect()
    }

    /// Delete thread
    pub async fn delete_thread(&self, thread_id: &CommentId) -> Result<()> {
        if let Some(thread) = self.threads.write().await.remove(thread_id) {
            if let Some(doc_threads) = self.document_threads.write().await.get_mut(&thread.document_id) {
                doc_threads.retain(|id| id != thread_id);
            }
        }
        Ok(())
    }

    /// Update comment
    pub async fn update_comment(
        &self,
        thread_id: &CommentId,
        comment_id: &CommentId,
        content: String,
    ) -> Result<()> {
        let mut threads = self.threads.write().await;
        let thread = threads.get_mut(thread_id)
            .ok_or_else(|| crate::CollaborationError::SessionNotFound(thread_id.to_string()))?;

        let comment = thread.comments.iter_mut()
            .find(|c| c.id == *comment_id)
            .ok_or_else(|| crate::CollaborationError::SessionNotFound(comment_id.to_string()))?;

        comment.content = content;
        comment.updated_at = Utc::now();
        thread.updated_at = Utc::now();

        Ok(())
    }
}