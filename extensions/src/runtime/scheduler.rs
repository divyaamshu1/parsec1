//! Task scheduler for extensions
//! 
//! Manages concurrent execution of extension tasks.

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Priority levels for scheduled tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// A scheduled task
pub struct ScheduledTask {
    pub id: u64,
    pub priority: TaskPriority,
    pub task: Box<dyn FnOnce() + Send + 'static>,
}

/// Scheduler for managing concurrent extension tasks
pub struct Scheduler {
    tasks: Arc<Mutex<VecDeque<ScheduledTask>>>,
    next_id: Arc<Mutex<u64>>,
    max_concurrent: usize,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(VecDeque::new())),
            next_id: Arc::new(Mutex::new(1)),
            max_concurrent,
        }
    }

    /// Schedule a task
    pub async fn schedule<F>(&self, priority: TaskPriority, task: F) -> u64
    where
        F: FnOnce() + Send + 'static,
    {
        let mut id_lock = self.next_id.lock().await;
        let id = *id_lock;
        *id_lock += 1;

        let scheduled = ScheduledTask {
            id,
            priority,
            task: Box::new(task),
        };

        let mut tasks = self.tasks.lock().await;
        tasks.push_back(scheduled);
        
        // Sort by priority (would need proper implementation)
        id
    }

    /// Get the next task to execute
    pub async fn next_task(&self) -> Option<ScheduledTask> {
        let mut tasks = self.tasks.lock().await;
        tasks.pop_front()
    }

    /// Cancel a task
    pub async fn cancel(&self, id: u64) -> bool {
        let mut tasks = self.tasks.lock().await;
        let len_before = tasks.len();
        tasks.retain(|t| t.id != id);
        tasks.len() != len_before
    }

    /// Get task count
    pub async fn task_count(&self) -> usize {
        self.tasks.lock().await.len()
    }
}