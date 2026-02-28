//! Code review workflows

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::{UserId, DocumentId, ReviewId, Result, Range};

/// Review status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewStatus {
    Draft,
    Pending,
    InProgress,
    Approved,
    ChangesRequested,
    Rejected,
    Merged,
    Closed,
}

/// Review priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Change request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRequest {
    pub id: String,
    pub reviewer: UserId,
    pub description: String,
    pub range: Option<Range>,
    pub created_at: DateTime<Utc>,
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    pub id: String,
    pub reviewer: UserId,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub inline_comments: Vec<InlineComment>,
}

/// Inline comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineComment {
    pub id: String,
    pub range: Range,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub resolved: bool,
}

/// Review
#[derive(Debug, Clone)]
pub struct Review {
    pub id: ReviewId,
    pub document_id: DocumentId,
    pub title: String,
    pub description: String,
    pub author: UserId,
    pub reviewers: Vec<UserId>,
    pub status: ReviewStatus,
    pub priority: ReviewPriority,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub change_requests: Vec<ChangeRequest>,
    pub feedback: Vec<Feedback>,
    pub labels: Vec<String>,
    pub branch: Option<String>,
    pub commit_hash: Option<String>,
    pub base_commit: Option<String>,
}

/// Review manager
pub struct ReviewManager {
    reviews: Arc<RwLock<HashMap<ReviewId, Review>>>,
    document_reviews: Arc<RwLock<HashMap<DocumentId, Vec<ReviewId>>>>,
    user_reviews: Arc<RwLock<HashMap<UserId, Vec<ReviewId>>>>,
}

impl ReviewManager {
    /// Create new review manager
    pub fn new() -> Self {
        Self {
            reviews: Arc::new(RwLock::new(HashMap::new())),
            document_reviews: Arc::new(RwLock::new(HashMap::new())),
            user_reviews: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create new review
    pub async fn create_review(
        &self,
        document_id: DocumentId,
        title: String,
        description: String,
        author: UserId,
        reviewers: Vec<UserId>,
        priority: ReviewPriority,
    ) -> Result<Review> {
        let review_id = ReviewId::new();

        let review = Review {
            id: review_id.clone(),
            document_id: document_id.clone(),
            title,
            description,
            author: author.clone(),
            reviewers,
            status: ReviewStatus::Draft,
            priority,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            change_requests: Vec::new(),
            feedback: Vec::new(),
            labels: Vec::new(),
            branch: None,
            commit_hash: None,
            base_commit: None,
        };

        self.reviews.write().await.insert(review_id.clone(), review.clone());
        
        // Index by document
        self.document_reviews.write().await
            .entry(document_id)
            .or_insert_with(Vec::new)
            .push(review_id.clone());

        // Index by author and reviewers
        let mut user_reviews = self.user_reviews.write().await;
        user_reviews.entry(author).or_insert_with(Vec::new).push(review_id.clone());
        for reviewer in &review.reviewers {
            user_reviews.entry(reviewer.clone()).or_insert_with(Vec::new).push(review_id.clone());
        }

        Ok(review)
    }

    /// Update review status
    pub async fn update_status(&self, review_id: &ReviewId, status: ReviewStatus) -> Result<()> {
        let mut reviews = self.reviews.write().await;
        if let Some(review) = reviews.get_mut(review_id) {
            review.status = status;
            review.updated_at = Utc::now();
            
            if matches!(status, ReviewStatus::Approved | ReviewStatus::Rejected | ReviewStatus::Merged) {
                review.completed_at = Some(Utc::now());
            }
        }
        Ok(())
    }

    /// Add reviewer
    pub async fn add_reviewer(&self, review_id: &ReviewId, reviewer: UserId) -> Result<()> {
        let mut reviews = self.reviews.write().await;
        if let Some(review) = reviews.get_mut(review_id) {
            if !review.reviewers.contains(&reviewer) {
                review.reviewers.push(reviewer.clone());
                review.updated_at = Utc::now();
                
                self.user_reviews.write().await
                    .entry(reviewer)
                    .or_insert_with(Vec::new)
                    .push(review_id.clone());
            }
        }
        Ok(())
    }

    /// Remove reviewer
    pub async fn remove_reviewer(&self, review_id: &ReviewId, reviewer: &UserId) -> Result<()> {
        let mut reviews = self.reviews.write().await;
        if let Some(review) = reviews.get_mut(review_id) {
            review.reviewers.retain(|r| r != reviewer);
            review.updated_at = Utc::now();
            
            if let Some(user_reviews) = self.user_reviews.write().await.get_mut(reviewer) {
                user_reviews.retain(|id| id != review_id);
            }
        }
        Ok(())
    }

    /// Add change request
    pub async fn add_change_request(
        &self,
        review_id: &ReviewId,
        reviewer: UserId,
        description: String,
        range: Option<Range>,
    ) -> Result<ChangeRequest> {
        let mut reviews = self.reviews.write().await;
        if let Some(review) = reviews.get_mut(review_id) {
            let change_request = ChangeRequest {
                id: uuid::Uuid::new_v4().to_string(),
                reviewer,
                description,
                range,
                created_at: Utc::now(),
                resolved: false,
                resolved_at: None,
            };
            
            review.change_requests.push(change_request.clone());
            review.updated_at = Utc::now();
            
            // Update status
            review.status = ReviewStatus::ChangesRequested;
            
            Ok(change_request)
        } else {
            Err(crate::CollaborationError::SessionNotFound(review_id.to_string()))
        }
    }

    /// Resolve change request
    pub async fn resolve_change_request(
        &self,
        review_id: &ReviewId,
        request_id: &str,
    ) -> Result<()> {
        let mut reviews = self.reviews.write().await;
        if let Some(review) = reviews.get_mut(review_id) {
            if let Some(request) = review.change_requests.iter_mut().find(|cr| cr.id == request_id) {
                request.resolved = true;
                request.resolved_at = Some(Utc::now());
                review.updated_at = Utc::now();
            }
        }
        Ok(())
    }

    /// Add feedback
    pub async fn add_feedback(
        &self,
        review_id: &ReviewId,
        reviewer: UserId,
        content: String,
    ) -> Result<Feedback> {
        let mut reviews = self.reviews.write().await;
        if let Some(review) = reviews.get_mut(review_id) {
            let feedback = Feedback {
                id: uuid::Uuid::new_v4().to_string(),
                reviewer,
                content,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                inline_comments: Vec::new(),
            };
            
            review.feedback.push(feedback.clone());
            review.updated_at = Utc::now();
            
            Ok(feedback)
        } else {
            Err(crate::CollaborationError::SessionNotFound(review_id.to_string()))
        }
    }

    /// Add inline comment to feedback
    pub async fn add_inline_comment(
        &self,
        review_id: &ReviewId,
        feedback_id: &str,
        range: Range,
        content: String,
    ) -> Result<InlineComment> {
        let mut reviews = self.reviews.write().await;
        if let Some(review) = reviews.get_mut(review_id) {
            if let Some(feedback) = review.feedback.iter_mut().find(|f| f.id == feedback_id) {
                let comment = InlineComment {
                    id: uuid::Uuid::new_v4().to_string(),
                    range,
                    content,
                    created_at: Utc::now(),
                    resolved: false,
                };
                
                feedback.inline_comments.push(comment.clone());
                feedback.updated_at = Utc::now();
                review.updated_at = Utc::now();
                
                Ok(comment)
            } else {
                Err(crate::CollaborationError::SessionNotFound(feedback_id.to_string()))
            }
        } else {
            Err(crate::CollaborationError::SessionNotFound(review_id.to_string()))
        }
    }

    /// Get review
    pub async fn get_review(&self, review_id: &ReviewId) -> Option<Review> {
        self.reviews.read().await.get(review_id).cloned()
    }

    /// Get reviews for document
    pub async fn get_document_reviews(&self, document_id: &DocumentId) -> Vec<Review> {
        let reviews = self.reviews.read().await;
        let doc_reviews = self.document_reviews.read().await;
        
        if let Some(ids) = doc_reviews.get(document_id) {
            ids.iter()
                .filter_map(|id| reviews.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get reviews for user
    pub async fn get_user_reviews(&self, user_id: &UserId) -> Vec<Review> {
        let reviews = self.reviews.read().await;
        let user_reviews = self.user_reviews.read().await;
        
        if let Some(ids) = user_reviews.get(user_id) {
            ids.iter()
                .filter_map(|id| reviews.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get pending reviews for user
    pub async fn get_pending_reviews(&self, user_id: &UserId) -> Vec<Review> {
        let reviews = self.reviews.read().await;
        let user_reviews = self.user_reviews.read().await;
        
        if let Some(ids) = user_reviews.get(user_id) {
            ids.iter()
                .filter_map(|id| {
                    let review = reviews.get(id)?;
                    if matches!(review.status, ReviewStatus::Pending | ReviewStatus::InProgress) {
                        Some(review.clone())
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Delete review
    pub async fn delete_review(&self, review_id: &ReviewId) -> Result<()> {
        if let Some(review) = self.reviews.write().await.remove(review_id) {
            // Remove from document index
            if let Some(doc_reviews) = self.document_reviews.write().await.get_mut(&review.document_id) {
                doc_reviews.retain(|id| id != review_id);
            }
            
            // Remove from user indexes
            let mut user_reviews = self.user_reviews.write().await;
            if let Some(reviews) = user_reviews.get_mut(&review.author) {
                reviews.retain(|id| id != review_id);
            }
            for reviewer in &review.reviewers {
                if let Some(reviews) = user_reviews.get_mut(reviewer) {
                    reviews.retain(|id| id != review_id);
                }
            }
        }
        Ok(())
    }
}