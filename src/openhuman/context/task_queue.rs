//! Task Queue — prevents task loss across message interruptions.
//!
//! When a user sends multiple messages in quick succession, the agent's
//! current turn may be interrupted (new message preempts the old one).
//! This module provides a persistent task queue that:
//!
//! 1. **Tracks pending tasks** — every user request is recorded
//! 2. **Survives interruptions** — tasks are stored in mem0, not just context
//! 3. **Forces completion** — the system prompt includes all pending tasks
//!    with explicit instructions to address each one
//!
//! ## Architecture
//!
//! ```text
//! New user message arrives
//!     ↓
//! [TaskQueue::enqueue()] — record the task as "pending"
//!     ↓
//! [TaskQueue::recall_pending()] — fetch all pending tasks from mem0
//!     ↓
//! [format_tasks_for_prompt()] — inject into system prompt
//!     ↓
//! Agent processes ALL pending tasks (not just the latest message)
//!     ↓
//! [TaskQueue::mark_completed()] — mark tasks done after handling
//! ```
//!
//! ## Why this works
//!
//! The core problem: OpenClaw's message scheduler interrupts the current
//! turn when a new message arrives. This means:
//! - Message A arrives → agent starts working on A
//! - Message B arrives → agent is interrupted, starts working on B
//! - Task A is lost
//!
//! Solution: store tasks externally (in mem0), recall them at the start
//! of every turn, and inject them into the system prompt. Even if the
//! agent is interrupted, the next turn will see ALL pending tasks.

use super::mem0_recall::{Mem0Client, Mem0Config};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Status of a tracked task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task has been recorded but not yet addressed.
    Pending,
    /// Agent is currently working on this task.
    InProgress,
    /// Task has been completed.
    Completed,
    /// Task could not be completed (blocked by external factor).
    Blocked,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "⏳"),
            TaskStatus::InProgress => write!(f, "🔄"),
            TaskStatus::Completed => write!(f, "✅"),
            TaskStatus::Blocked => write!(f, "🚫"),
        }
    }
}

/// A single tracked task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedTask {
    /// Unique identifier (uuid or sequential).
    pub id: String,
    /// Original user message that created this task.
    pub source_message: String,
    /// Brief description of what needs to be done.
    pub description: String,
    /// Current status.
    pub status: TaskStatus,
    /// When the task was created (ISO 8601).
    pub created_at: String,
    /// When the task was last updated.
    pub updated_at: String,
    /// Optional reason if blocked.
    pub block_reason: Option<String>,
}

/// Configuration for the task queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskQueueConfig {
    /// Whether task tracking is enabled.
    pub enabled: bool,
    /// Maximum number of pending tasks to show in prompt.
    pub max_display_tasks: usize,
    /// Maximum characters for the task injection section.
    pub max_injection_chars: usize,
    /// Mem0 metadata key used to tag task memories.
    pub memory_tag: String,
}

impl Default for TaskQueueConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_display_tasks: 20,
            max_injection_chars: 3000,
            memory_tag: "task_queue".to_string(),
        }
    }
}

/// The task queue client. Uses mem0 for persistent storage.
pub struct TaskQueue {
    config: TaskQueueConfig,
    mem0: Mem0Client,
}

impl TaskQueue {
    pub fn new(mem0_config: Mem0Config, queue_config: TaskQueueConfig) -> Self {
        let mem0 = Mem0Client::new(mem0_config);
        Self {
            config: queue_config,
            mem0,
        }
    }

    /// Record a new task from a user message.
    pub async fn enqueue(
        &self,
        message: &str,
        description: &str,
    ) -> anyhow::Result<TrackedTask> {
        if !self.config.enabled {
            return Ok(TrackedTask {
                id: String::new(),
                source_message: message.to_string(),
                description: description.to_string(),
                status: TaskStatus::Pending,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
                block_reason: None,
            });
        }

        let now = chrono::Utc::now().to_rfc3339();
        let task = TrackedTask {
            id: format!("task_{}", chrono::Utc::now().timestamp_millis()),
            source_message: message.to_string(),
            description: description.to_string(),
            status: TaskStatus::Pending,
            created_at: now.clone(),
            updated_at: now,
            block_reason: None,
        };

        // Store as a mem0 memory with task metadata
        let task_json = serde_json::to_string(&task)?;
        self.mem0.store(&task_json).await?;

        tracing::info!(task_id = %task.id, desc = %task.description, "[task_queue] enqueued task");

        Ok(task)
    }

    /// Recall all pending tasks from mem0.
    pub async fn recall_pending(&self) -> anyhow::Result<Vec<TrackedTask>> {
        if !self.config.enabled {
            return Ok(vec![]);
        }

        let recall = self
            .mem0
            .recall(&format!("pending tasks {}", self.config.memory_tag))
            .await?;

        let mut tasks = Vec::new();
        for mem in &recall.memories {
            // Try to parse as a tracked task
            if let Ok(task) = serde_json::from_str::<TrackedTask>(&mem.memory) {
                if task.status != TaskStatus::Completed {
                    tasks.push(task);
                }
            }
        }

        // Sort by creation time (oldest first)
        tasks.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        tasks.truncate(self.config.max_display_tasks);

        Ok(tasks)
    }

    /// Mark a task as completed.
    pub async fn mark_completed(&self, task_id: &str) -> anyhow::Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Store completion marker
        let completion = format!(
            "TASK COMPLETED: {} at {}",
            task_id,
            chrono::Utc::now().to_rfc3339()
        );
        self.mem0.store(&completion).await?;

        tracing::info!(task_id = %task_id, "[task_queue] marked completed");
        Ok(())
    }

    /// Mark a task as blocked.
    pub async fn mark_blocked(
        &self,
        task_id: &str,
        reason: &str,
    ) -> anyhow::Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let blockage = format!(
            "TASK BLOCKED: {} — reason: {}",
            task_id, reason
        );
        self.mem0.store(&blockage).await?;

        tracing::info!(task_id = %task_id, reason = %reason, "[task_queue] marked blocked");
        Ok(())
    }
}

/// Format pending tasks into a system prompt section.
///
/// This is the key prompt engineering: it forces the agent to acknowledge
/// and address every pending task, preventing the "only answer the last
/// message" behavior.
pub fn format_tasks_for_prompt(tasks: &[TrackedTask]) -> String {
    if tasks.is_empty() {
        return String::new();
    }

    let mut parts = vec![
        "## ⚠️ Pending Tasks (MUST address ALL of these)".to_string(),
        "You have unfinished tasks from previous messages. You MUST address each one.".to_string(),
        "Do NOT skip any task. If a task is blocked, say so explicitly.".to_string(),
        String::new(),
    ];

    for (i, task) in tasks.iter().enumerate() {
        let status_icon = task.status;
        let blocked = task
            .block_reason
            .as_ref()
            .map(|r| format!(" (blocked: {})", r))
            .unwrap_or_default();
        parts.push(format!(
            "{}. {} **[{}]** {}{}",
            i + 1,
            status_icon,
            task.id,
            task.description,
            blocked
        ));
    }

    parts.push(String::new());
    parts.push("Pattern: address each task in order, mark ✅ when done.".to_string());

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_empty_tasks() {
        assert!(format_tasks_for_prompt(&[]).is_empty());
    }

    #[test]
    fn format_pending_tasks() {
        let tasks = vec![
            TrackedTask {
                id: "task_1".to_string(),
                source_message: "push to github".to_string(),
                description: "Push commit 938686ac to GitHub".to_string(),
                status: TaskStatus::Pending,
                created_at: "2026-05-23T06:00:00Z".to_string(),
                updated_at: "2026-05-23T06:00:00Z".to_string(),
                block_reason: None,
            },
            TrackedTask {
                id: "task_2".to_string(),
                source_message: "solve task loss".to_string(),
                description: "Implement task queue in harness".to_string(),
                status: TaskStatus::InProgress,
                created_at: "2026-05-23T06:30:00Z".to_string(),
                updated_at: "2026-05-23T06:30:00Z".to_string(),
                block_reason: None,
            },
            TrackedTask {
                id: "task_3".to_string(),
                source_message: "check API key".to_string(),
                description: "Verify new API key activation".to_string(),
                status: TaskStatus::Blocked,
                created_at: "2026-05-23T07:00:00Z".to_string(),
                updated_at: "2026-05-23T07:00:00Z".to_string(),
                block_reason: Some("User needs to activate key".to_string()),
            },
        ];
        let formatted = format_tasks_for_prompt(&tasks);
        assert!(formatted.contains("Pending Tasks"));
        assert!(formatted.contains("task_1"));
        assert!(formatted.contains("Push commit"));
        assert!(formatted.contains("blocked: User needs to activate"));
        assert!(formatted.contains("address each task"));
    }

    #[test]
    fn default_config_enabled() {
        let config = TaskQueueConfig::default();
        assert!(config.enabled);
    }
}
