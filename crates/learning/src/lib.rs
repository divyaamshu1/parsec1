//! Parsec Learning Tools
//!
//! Interactive learning features including:
//! - Interactive tutorials
//! - Code snippets library
//! - Cheat sheets
//! - Interactive playgrounds
//! - AI-powered learning assistant
//! - Progress tracking
//! - Skill assessments

#![allow(dead_code, unused_imports)]

pub mod tutorials;
pub mod snippets;
pub mod cheat_sheets;
pub mod playground;

// Inline stubs for previously external modules (no new files created)
pub mod ai_assistant {
    use super::{LearningConfig, Result};
    #[derive(Debug, Clone)]
    pub struct LearningAssistant;
    #[derive(Debug, Clone)]
    pub struct AssistantMessage;
    #[derive(Debug, Clone)]
    pub enum AssistantRole { User, System, Assistant }
    #[derive(Debug, Clone)]
    pub struct LearningContext;

    impl LearningAssistant {
        pub async fn new(_config: LearningConfig) -> Result<Self> {
            Ok(LearningAssistant)
        }
    }
}

pub mod progress {
    use super::{Result, assessment::SkillLevel, LearnerId};
    use std::collections::HashMap;
    use std::path::PathBuf;
    #[derive(Debug, Clone)]
    pub struct ProgressTracker;
    #[derive(Debug, Clone)]
    pub struct UserProgress {
        pub completed_tutorials: Vec<String>,
        pub started_tutorials: Vec<String>,
        pub skills: HashMap<String, SkillLevel>,
    }
    #[derive(Debug, Clone)]
    pub enum Skill { Beginner, Intermediate, Advanced, Expert }
    #[derive(Debug, Clone)]
    pub struct Achievement;
    #[derive(Debug, Clone)]
    pub struct Badge;

    impl UserProgress {
        pub fn new() -> Self {
            Self {
                completed_tutorials: Vec::new(),
                started_tutorials: Vec::new(),
                skills: HashMap::new(),
            }
        }
    }

    impl ProgressTracker {
        pub fn new(_user_data_dir: PathBuf, _learner: LearnerId) -> Self {
            ProgressTracker
        }
        pub async fn get_progress(&self) -> Result<UserProgress> {
            Ok(UserProgress::new())
        }
    }
}

pub mod assessment {
    // use the same SkillLevel definition as tutorials so types line up
    pub use crate::tutorials::SkillLevel;

    #[derive(Debug, Clone)]
    pub struct Assessment;
    #[derive(Debug, Clone)]
    pub struct Question;
    #[derive(Debug, Clone)]
    pub struct Answer;
    #[derive(Debug, Clone)]
    pub struct AssessmentResult;
}

pub mod content {
    // placeholder
}

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast, mpsc};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Datelike};

// Re-exports
pub use tutorials::{Tutorial, TutorialManager, TutorialStep, StepType, TutorialProgress};
pub use snippets::{Snippet, SnippetManager, SnippetLanguage, SnippetTag};
pub use cheat_sheets::{CheatSheet, CheatSheetManager, CheatCategory, CheatEntry};
pub use playground::{PlaygroundManager, PlaygroundLanguage, PlaygroundConfig, ExecutionResult};
pub use ai_assistant::{LearningAssistant, AssistantMessage, AssistantRole, LearningContext};
pub use progress::{ProgressTracker, UserProgress, Skill as ProgressSkill, Achievement, Badge};
pub use assessment::{Assessment, Question, Answer, AssessmentResult, SkillLevel};

/// Result type for learning operations
pub type Result<T> = std::result::Result<T, LearningError>;

/// Learning error
#[derive(Debug, thiserror::Error)]
pub enum LearningError {
    #[error("Tutorial not found: {0}")]
    TutorialNotFound(String),

    #[error("Step not found: {0}")]
    StepNotFound(String),

    #[error("Snippet not found: {0}")]
    SnippetNotFound(String),

    #[error("Cheat sheet not found: {0}")]
    CheatSheetNotFound(String),

    #[error("Playground error: {0}")]
    PlaygroundError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Assessment failed: {0}")]
    AssessmentFailed(String),

    #[error("AI assistant error: {0}")]
    AIAssistantError(String),

    #[error("Content error: {0}")]
    ContentError(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
}

/// User identifier for learning
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LearnerId(pub String);

impl LearnerId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

/// Content identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentId(pub String);

impl ContentId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

/// Learning content type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    Tutorial,
    Snippet,
    CheatSheet,
    Article,
    Video,
    Exercise,
    Project,
    Assessment,
}

/// Difficulty level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

impl std::fmt::Display for Difficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Difficulty::Beginner => write!(f, "Beginner"),
            Difficulty::Intermediate => write!(f, "Intermediate"),
            Difficulty::Advanced => write!(f, "Advanced"),
            Difficulty::Expert => write!(f, "Expert"),
        }
    }
}

/// Learning event
#[derive(Debug, Clone)]
pub enum LearningEvent {
    TutorialStarted(ContentId, String),
    TutorialCompleted(ContentId, String),
    StepCompleted(ContentId, String, String),
    SnippetSaved(Snippet),
    CheatSheetViewed(ContentId, String),
    PlaygroundStarted(PlaygroundLanguage),
    CodeExecuted(String, bool),
    AssessmentCompleted(ContentId, f32),
    AchievementUnlocked(String),
    BadgeEarned(String),
    SkillImproved(String, SkillLevel),
    ProgressUpdated(UserProgress),
}

/// Learning configuration
#[derive(Debug, Clone)]
pub struct LearningConfig {
    /// Content directory
    pub content_dir: PathBuf,
    /// User data directory
    pub user_data_dir: PathBuf,
    /// Enable AI assistant
    pub enable_ai: bool,
    /// AI provider (openai, anthropic, local)
    pub ai_provider: String,
    /// AI model
    pub ai_model: String,
    /// AI API key
    pub ai_api_key: Option<String>,
    /// Enable playground
    pub enable_playground: bool,
    /// Playground timeout (seconds)
    pub playground_timeout: u64,
    /// Enable metrics
    pub enable_metrics: bool,
    /// Enable offline mode
    pub offline_mode: bool,
}

impl Default for LearningConfig {
    fn default() -> Self {
        let data_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("parsec");
        
        Self {
            content_dir: data_dir.join("learning"),
            user_data_dir: data_dir.join("user").join("learning"),
            enable_ai: true,
            ai_provider: "openai".to_string(),
            ai_model: "gpt-4".to_string(),
            ai_api_key: None,
            enable_playground: true,
            playground_timeout: 30,
            enable_metrics: true,
            offline_mode: false,
        }
    }
}

/// Main learning engine
pub struct LearningEngine {
    /// Configuration
    config: LearningConfig,
    /// Tutorial manager
    tutorials: Arc<TutorialManager>,
    /// Snippet manager
    snippets: Arc<SnippetManager>,
    /// Cheat sheet manager
    cheat_sheets: Arc<CheatSheetManager>,
    /// Playground manager
    playground: Arc<PlaygroundManager>,
    /// AI assistant
    ai_assistant: Arc<LearningAssistant>,
    /// Progress tracker
    progress: Arc<ProgressTracker>,
    /// Event broadcaster
    event_tx: broadcast::Sender<LearningEvent>,
    /// Event receiver
    event_rx: broadcast::Receiver<LearningEvent>,
    /// Current learner
    current_learner: LearnerId,
}

impl LearningEngine {
    /// Create new learning engine
    pub async fn new(config: LearningConfig, learner_id: Option<LearnerId>) -> Result<Self> {
        let (event_tx, event_rx) = broadcast::channel(100);
        
        let learner = learner_id.unwrap_or_else(LearnerId::new);
        
        // Create directories
        tokio::fs::create_dir_all(&config.content_dir).await?;
        tokio::fs::create_dir_all(&config.user_data_dir).await?;

        Ok(Self {
            tutorials: Arc::new(TutorialManager::new(config.content_dir.join("tutorials"))),
            snippets: Arc::new(SnippetManager::new(config.user_data_dir.join("snippets"))),
            cheat_sheets: Arc::new(CheatSheetManager::new(config.content_dir.join("cheatsheets"))),
            playground: Arc::new(PlaygroundManager::new(config.clone()).await?),
            ai_assistant: Arc::new(LearningAssistant::new(config.clone()).await?),
            progress: Arc::new(ProgressTracker::new(config.user_data_dir.clone(), learner.clone())),
            config,
            event_tx,
            event_rx,
            current_learner: learner,
        })
    }

    /// Get tutorial manager
    pub fn tutorials(&self) -> Arc<TutorialManager> {
        self.tutorials.clone()
    }

    /// Get snippet manager
    pub fn snippets(&self) -> Arc<SnippetManager> {
        self.snippets.clone()
    }

    /// Get cheat sheet manager
    pub fn cheat_sheets(&self) -> Arc<CheatSheetManager> {
        self.cheat_sheets.clone()
    }

    /// Get playground manager
    pub fn playground(&self) -> Arc<PlaygroundManager> {
        self.playground.clone()
    }

    /// Get AI assistant
    pub fn ai_assistant(&self) -> Arc<LearningAssistant> {
        self.ai_assistant.clone()
    }

    /// Get progress tracker
    pub fn progress(&self) -> Arc<ProgressTracker> {
        self.progress.clone()
    }

    /// Subscribe to learning events
    pub fn subscribe(&self) -> broadcast::Receiver<LearningEvent> {
        self.event_tx.subscribe()
    }

    /// Get recommended content for user
    pub async fn get_recommendations(&self, count: usize) -> Result<Vec<RecommendedContent>> {
        let progress = self.progress.get_progress().await?;
        let mut recommendations = Vec::new();

        // Get tutorials based on skill level
        let tutorials = self.tutorials.list().await?;
        for tutorial in tutorials {
            if let Some(skill) = &tutorial.required_skill {
                if let Some(level) = progress.skills.get(skill) {
                    if *level >= tutorial.min_skill_level.unwrap_or(SkillLevel::Beginner) {
                        let score = self.calculate_relevance(&tutorial, &progress);
                        recommendations.push(RecommendedContent {
                            id: ContentId(tutorial.id.clone()),
                            title: tutorial.title.clone(),
                            content_type: ContentType::Tutorial,
                            difficulty: tutorial.difficulty,
                            relevance_score: score,
                            estimated_minutes: tutorial.estimated_minutes,
                        });
                    }
                }
            }
        }

        // Sort by relevance and limit
        recommendations.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        recommendations.truncate(count);

        Ok(recommendations)
    }

    /// Calculate content relevance
    fn calculate_relevance(&self, tutorial: &Tutorial, progress: &UserProgress) -> f32 {
        let mut score = 1.0;

        // Factor in completed tutorials
        if progress.completed_tutorials.contains(&tutorial.id) {
            score *= 0.1; // Already completed
        }

        // Factor in started tutorials
        if progress.started_tutorials.contains(&tutorial.id) {
            score *= 0.5; // Already started
        }

        // Factor in skill level
        if let Some(skill) = &tutorial.required_skill {
            if let Some(level) = progress.skills.get(skill) {
                let level_score = match level {
                    SkillLevel::Beginner => 0.3,
                    SkillLevel::Intermediate => 0.6,
                    SkillLevel::Advanced => 0.9,
                    SkillLevel::Expert => 1.0,
                };
                score *= level_score;
            }
        }

        score
    }

    /// Search all learning content
    pub async fn search(&self, query: &str, content_types: Option<Vec<ContentType>>) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();

        // Search tutorials
        if content_types.as_ref().map_or(true, |types| types.contains(&ContentType::Tutorial)) {
            let tutorials = self.tutorials.search(query).await?;
            for tutorial in &tutorials {
                results.push(SearchResult {
                    id: ContentId(tutorial.id.clone()),
                    title: tutorial.title.clone(),
                    description: tutorial.description.clone(),
                    content_type: ContentType::Tutorial,
                    url: format!("/learn/tutorials/{}", tutorial.id),
                });
            }
        }

        // Search snippets
        if content_types.as_ref().map_or(true, |types| types.contains(&ContentType::Snippet)) {
            let snippets = self.snippets.search(query).await?;
            for snippet in &snippets {
                results.push(SearchResult {
                    id: ContentId(snippet.id.clone()),
                    title: snippet.title.clone(),
                    description: snippet.description.clone().unwrap_or_else(|| snippet.title.clone()),
                    content_type: ContentType::Snippet,
                    url: format!("/learn/snippets/{}", snippet.id),
                });
            }
        }

        // Search cheat sheets
        if content_types.as_ref().map_or(true, |types| types.contains(&ContentType::CheatSheet)) {
            let sheets = self.cheat_sheets.search(query).await?;
            for sheet in &sheets {
                results.push(SearchResult {
                    id: ContentId(sheet.id.clone()),
                    title: sheet.title.clone(),
                    description: sheet.description.clone(),
                    content_type: ContentType::CheatSheet,
                    url: format!("/learn/cheatsheets/{}", sheet.id),
                });
            }
        }

        Ok(results)
    }
}

/// Recommended content item
#[derive(Debug, Clone)]
pub struct RecommendedContent {
    pub id: ContentId,
    pub title: String,
    pub content_type: ContentType,
    pub difficulty: Difficulty,
    pub relevance_score: f32,
    pub estimated_minutes: Option<u32>,
}

/// Search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: ContentId,
    pub title: String,
    pub description: String,
    pub content_type: ContentType,
    pub url: String,
}