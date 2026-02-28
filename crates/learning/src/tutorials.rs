//! Interactive tutorials system

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::fs;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Datelike};
use tera::{Tera, Context as TeraContext};
use markdown::to_html;
use tracing::{info, warn, debug};

use crate::{ContentId, Difficulty, Result, LearningError, LearnerId};

/// Tutorial step type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepType {
    Explanation,
    Code,
    Exercise,
    Quiz,
    Challenge,
    Video,
}

/// Tutorial step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TutorialStep {
    pub id: String,
    pub title: String,
    pub step_type: StepType,
    pub content: String,
    pub code_template: Option<String>,
    pub solution: Option<String>,
    pub hints: Vec<String>,
    pub expected_output: Option<String>,
    pub test_file: Option<String>,
    pub dependencies: Vec<String>,
    pub estimated_minutes: Option<u32>,
}

/// Tutorial
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tutorial {
    pub id: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub language: String,
    pub difficulty: Difficulty,
    pub tags: Vec<String>,
    pub prerequisites: Vec<String>,
    pub required_skill: Option<String>,
    pub min_skill_level: Option<SkillLevel>,
    pub steps: Vec<TutorialStep>,
    pub estimated_minutes: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub cover_image: Option<String>,
    pub readme: Option<String>,
    pub repository: Option<String>,
}

/// Tutorial progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TutorialProgress {
    pub tutorial_id: String,
    pub learner_id: LearnerId,
    pub current_step: usize,
    pub completed_steps: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub score: Option<f32>,
    pub answers: HashMap<String, String>,
    pub code_attempts: HashMap<String, Vec<String>>,
}

/// Skill level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SkillLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

/// Tutorial manager
pub struct TutorialManager {
    tutorials: Arc<RwLock<HashMap<String, Tutorial>>>,
    progress: Arc<RwLock<HashMap<String, TutorialProgress>>>,
    content_dir: PathBuf,
    template_engine: Option<Tera>,
}

impl TutorialManager {
    /// Create new tutorial manager
    pub fn new(content_dir: PathBuf) -> Self {
        let mut template_engine = Tera::default();
        // templates may not exist in workspace; provide empty placeholders
        let _ = template_engine.add_raw_templates(vec![
            ("base", ""),
            ("tutorial", ""),
            ("step", ""),
        ]);

        Self {
            tutorials: Arc::new(RwLock::new(HashMap::new())),
            progress: Arc::new(RwLock::new(HashMap::new())),
            content_dir,
            template_engine: Some(template_engine),
        }
    }

    /// Load tutorial from directory
    pub async fn load_tutorial(&self, path: &Path) -> Result<String> {
        let manifest_path = path.join("tutorial.toml");
        if !manifest_path.exists() {
            return Err(LearningError::TutorialNotFound(format!("No manifest at {}", path.display())));
        }

        let content = fs::read_to_string(manifest_path).await.map_err(LearningError::Io)?;
        let mut tutorial: Tutorial = toml::from_str(&content).map_err(LearningError::Toml)?;

        // Load step content
        for step in &mut tutorial.steps {
            let step_path = path.join(format!("steps/{}.md", step.id));
            if step_path.exists() {
                let markdown = fs::read_to_string(step_path).await?;
                step.content = to_html(&markdown);
            }
        }

        // Load readme if exists
        let readme_path = path.join("README.md");
        if readme_path.exists() {
            let readme = fs::read_to_string(readme_path).await?;
            tutorial.readme = Some(to_html(&readme));
        }

        self.tutorials.write().await.insert(tutorial.id.clone(), tutorial.clone());
        info!("Loaded tutorial: {}", tutorial.title);

        Ok(tutorial.id)
    }

    /// Load all tutorials from content directory
    pub async fn load_all(&self) -> Result<Vec<String>> {
        let mut loaded = Vec::new();
        let mut read_dir = fs::read_dir(&self.content_dir).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(id) = self.load_tutorial(&path).await {
                    loaded.push(id);
                }
            }
        }

        Ok(loaded)
    }

    /// Get tutorial by ID
    pub async fn get_tutorial(&self, id: &str) -> Option<Tutorial> {
        self.tutorials.read().await.get(id).cloned()
    }

    /// List all tutorials
    pub async fn list(&self) -> Result<Vec<Tutorial>> {
        Ok(self.tutorials.read().await.values().cloned().collect())
    }

    /// Get tutorials by language
    pub async fn get_by_language(&self, language: &str) -> Vec<Tutorial> {
        self.tutorials.read().await
            .values()
            .filter(|t| t.language == language)
            .cloned()
            .collect()
    }

    /// Get tutorials by difficulty
    pub async fn get_by_difficulty(&self, difficulty: Difficulty) -> Vec<Tutorial> {
        self.tutorials.read().await
            .values()
            .filter(|t| t.difficulty == difficulty)
            .cloned()
            .collect()
    }

    /// Start tutorial for learner
    pub async fn start_tutorial(&self, tutorial_id: &str, learner_id: LearnerId) -> Result<TutorialProgress> {
        let _tutorial = self.get_tutorial(tutorial_id).await
            .ok_or_else(|| LearningError::TutorialNotFound(tutorial_id.to_string()))?;

        let progress = TutorialProgress {
            tutorial_id: tutorial_id.to_string(),
            learner_id,
            current_step: 0,
            completed_steps: Vec::new(),
            started_at: Utc::now(),
            last_activity: Utc::now(),
            completed_at: None,
            score: None,
            answers: HashMap::new(),
            code_attempts: HashMap::new(),
        };

        let key = format!("{}-{}", tutorial_id, progress.learner_id.0);
        self.progress.write().await.insert(key.clone(), progress.clone());

        Ok(progress)
    }

    /// Advance to next step
    pub async fn next_step(&self, tutorial_id: &str, learner_id: &LearnerId) -> Result<Option<TutorialStep>> {
        let key = format!("{}-{}", tutorial_id, learner_id.0);
        let mut progress = self.progress.write().await;
        
        if let Some(progress) = progress.get_mut(&key) {
            let tutorial = self.get_tutorial(tutorial_id).await
                .ok_or_else(|| LearningError::TutorialNotFound(tutorial_id.to_string()))?;

            if progress.current_step + 1 < tutorial.steps.len() {
                progress.current_step += 1;
                progress.last_activity = Utc::now();
                return Ok(Some(tutorial.steps[progress.current_step].clone()));
            }
        }

        Ok(None)
    }

    /// Complete current step
    pub async fn complete_step(
        &self,
        tutorial_id: &str,
        learner_id: &LearnerId,
        answer: Option<String>,
        code_attempt: Option<String>,
    ) -> Result<bool> {
        let key = format!("{}-{}", tutorial_id, learner_id.0);
        let mut progress = self.progress.write().await;
        
        if let Some(progress) = progress.get_mut(&key) {
            let tutorial = self.get_tutorial(tutorial_id).await
                .ok_or_else(|| LearningError::TutorialNotFound(tutorial_id.to_string()))?;

            let current_step = &tutorial.steps[progress.current_step];
            
            // Validate answer if it's a quiz
            if let Some(answer) = answer {
                progress.answers.insert(current_step.id.clone(), answer);
            }

            // Save code attempt
            if let Some(code) = code_attempt {
                let attempts = progress.code_attempts.entry(current_step.id.clone()).or_insert_with(Vec::new);
                attempts.push(code);
            }

            // Mark step as completed
            if !progress.completed_steps.contains(&current_step.id) {
                progress.completed_steps.push(current_step.id.clone());
            }

            progress.last_activity = Utc::now();

            // Check if tutorial is complete
            if progress.completed_steps.len() == tutorial.steps.len() {
                progress.completed_at = Some(Utc::now());
                
                // Calculate score
                let mut correct = 0;
                for (step_id, answer) in &progress.answers {
                    if let Some(step) = tutorial.steps.iter().find(|s| &s.id == step_id) {
                        if step.solution.as_ref() == Some(answer) {
                            correct += 1;
                        }
                    }
                }
                progress.score = Some(correct as f32 / fmax(tutorial.steps.len() as f32, 1.0));
                
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get tutorial progress for learner
    pub async fn get_progress(&self, tutorial_id: &str, learner_id: &LearnerId) -> Option<TutorialProgress> {
        let key = format!("{}-{}", tutorial_id, learner_id.0);
        self.progress.read().await.get(&key).cloned()
    }

    /// Search tutorials
    pub async fn search(&self, query: &str) -> Result<Vec<Tutorial>> {
        let query_lower = query.to_lowercase();
        let tutorials = self.tutorials.read().await;
        
        let results = tutorials
            .values()
            .filter(|t| {
                t.title.to_lowercase().contains(&query_lower) ||
                t.description.to_lowercase().contains(&query_lower) ||
                t.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect();

        Ok(results)
    }

    /// Render tutorial as HTML
    pub async fn render_tutorial(&self, tutorial: &Tutorial, progress: Option<&TutorialProgress>) -> Result<String> {
        let template = self.template_engine.as_ref()
            .ok_or_else(|| LearningError::ContentError("Template engine not available".to_string()))?;

        let mut context = TeraContext::new();
        context.insert("tutorial", tutorial);
        context.insert("progress", &progress);
        context.insert("current_year", &Utc::now().year());

        let html = template.render("tutorial", &context)
            .map_err(|e| LearningError::ContentError(format!("Template error: {}", e)))?;

        Ok(html)
    }

    /// Render step as HTML
    pub async fn render_step(&self, step: &TutorialStep, progress: Option<&TutorialProgress>) -> Result<String> {
        let template = self.template_engine.as_ref()
            .ok_or_else(|| LearningError::ContentError("Template engine not available".to_string()))?;

        let mut context = TeraContext::new();
        context.insert("step", step);
        context.insert("progress", &progress);

        let html = template.render("step", &context)
            .map_err(|e| LearningError::ContentError(format!("Template error: {}", e)))?;

        Ok(html)
    }
}

impl Default for TutorialManager {
    fn default() -> Self {
        Self::new(PathBuf::from("tutorials"))
    }
}

fn fmax(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}