//! Stage 0: Mem0 Recall — external memory layer that survives truncation.
//!
//! Before the context pipeline runs (tool budget → trim → microcompact →
//! autocompact), this stage queries a Mem0 instance for memories relevant
//! to the current user message and injects them as a system prompt section.
//!
//! After each turn, important facts are extracted and stored back into Mem0
//! so they survive across sessions and context truncation.
//!
//! ## Why Mem0 (not a custom solution)
//!
//! Mem0 (github.com/mem0ai/mem0, 25k+ stars) provides:
//! - **Single-pass ADD-only extraction** — one LLM call per add, no UPDATE/DELETE
//! - **Entity linking** — entities extracted, embedded, linked across memories
//! - **Multi-signal retrieval** — semantic + BM25 keyword + entity matching fused
//! - **Temporal reasoning** — time-aware retrieval ranks the right dated instance
//!
//! ## Architecture
//!
//! ```text
//! User message arrives
//!     ↓
//! [0] mem0_recall()  ← THIS MODULE — query mem0, inject results into prompt
//!     ↓
//! [1] tool_result_budget
//! [2] trim
//! [3] microcompact
//! [4] autocompact
//! [5] session_memory
//!     ↓
//! Turn complete
//!     ↓
//! [6] mem0_store()   ← extract facts from this turn, store to mem0
//! ```

use crate::openhuman::inference::provider::ConversationMessage;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing;

/// Configuration for the Mem0 recall/store integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mem0Config {
    /// Whether mem0 recall is enabled. When false, all operations are no-ops.
    pub enabled: bool,
    /// HTTP endpoint for the mem0 server. Defaults to localhost:8080.
    /// Can also be a Unix socket path prefixed with "unix:".
    pub endpoint: String,
    /// API key for mem0 (if using cloud). Empty for local self-hosted.
    pub api_key: String,
    /// User ID to scope memory queries. Typically the harness user's identifier.
    pub user_id: String,
    /// Maximum number of memories to recall per query.
    pub max_results: usize,
    /// Maximum characters of recalled memory text to inject into the prompt.
    /// Keeps the recall payload bounded so it doesn't eat into the context budget.
    pub max_injection_chars: usize,
    /// Minimum score threshold for recalled memories. Memories below this
    /// relevance score are discarded.
    pub min_score: f64,
}

impl Default for Mem0Config {
    fn default() -> Self {
        Self {
            enabled: false, // Opt-in — must be explicitly enabled in config
            endpoint: "http://127.0.0.1:8080".to_string(),
            api_key: String::new(),
            user_id: "default".to_string(),
            max_results: 10,
            max_injection_chars: 2048,
            min_score: 0.3,
        }
    }
}

/// A single recalled memory from Mem0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecalledMemory {
    pub id: String,
    pub memory: String,
    pub score: f64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Result of a mem0 recall operation.
#[derive(Debug, Clone)]
pub struct RecallResult {
    pub memories: Vec<RecalledMemory>,
    pub query: String,
    pub total_chars: usize,
    pub truncated: bool,
}

/// Result of a mem0 store operation.
#[derive(Debug, Clone)]
pub struct StoreResult {
    pub facts_stored: usize,
    pub memory_ids: Vec<String>,
}

/// The Mem0 recall/store client. Wraps HTTP calls to a running mem0 server.
///
/// In production, this talks to the mem0 self-hosted server (via Docker).
/// The server handles LLM-based fact extraction, embedding, vector storage,
/// and multi-signal retrieval internally — we just send add/search requests.
pub struct Mem0Client {
    config: Mem0Config,
    http: reqwest::Client,
}

impl Mem0Client {
    pub fn new(config: Mem0Config) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        Self { config, http }
    }

    /// Search for memories relevant to a query string.
    /// Uses mem0's multi-signal retrieval (semantic + BM25 + entity matching).
    pub async fn recall(&self, query: &str) -> Result<RecallResult> {
        if !self.config.enabled {
            return Ok(RecallResult {
                memories: vec![],
                query: query.to_string(),
                total_chars: 0,
                truncated: false,
            });
        }

        tracing::info!(query = %query, "[mem0_recall] searching memories");

        let response = self
            .http
            .post(format!("{}/v1/memories/search/", self.config.endpoint.trim_end_matches('/')))
            .header("Content-Type", "application/json")
            .bearer_auth(&self.config.api_key)
            .json(&serde_json::json!({
                "query": query,
                "user_id": self.config.user_id,
                "limit": self.config.max_results,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::warn!(status = %status, body = %body, "[mem0_recall] search failed");
            anyhow::bail!("mem0 search returned {}: {}", status, body);
        }

        let results: serde_json::Value = response.json().await?;
        let mut memories = Vec::new();
        let mut total_chars = 0;

        if let Some(results_arr) = results["results"].as_array() {
            for item in results_arr {
                let score = item["score"].as_f64().unwrap_or(0.0);
                if score < self.config.min_score {
                    continue;
                }
                let memory = RecalledMemory {
                    id: item["id"].as_str().unwrap_or("").to_string(),
                    memory: item["memory"].as_str().unwrap_or("").to_string(),
                    score,
                    created_at: item["created_at"].as_str().map(|s| s.to_string()),
                    updated_at: item["updated_at"].as_str().map(|s| s.to_string()),
                    metadata: item["metadata"].as_object().cloned().map(|v| v.into()),
                };
                total_chars += memory.memory.len();
                memories.push(memory);
            }
        }

        let truncated = total_chars > self.config.max_injection_chars;
        // Truncate if over budget — keep highest-scoring memories first
        if truncated {
            memories.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            let mut chars = 0;
            memories.retain(|m| {
                chars += m.memory.len();
                chars <= self.config.max_injection_chars
            });
            total_chars = chars;
        }

        tracing::info!(
            count = memories.len(),
            total_chars,
            truncated,
            "[mem0_recall] recalled memories"
        );

        Ok(RecallResult {
            memories,
            query: query.to_string(),
            total_chars,
            truncated,
        })
    }

    /// Store conversation content to mem0. Mem0 handles fact extraction
    /// internally via its single-pass ADD-only algorithm.
    pub async fn store(&self, messages: &str) -> Result<StoreResult> {
        if !self.config.enabled {
            return Ok(StoreResult {
                facts_stored: 0,
                memory_ids: vec![],
            });
        }

        tracing::info!(msg_len = messages.len(), "[mem0_store] adding to memory");

        let response = self
            .http
            .post(format!("{}/v1/memories/", self.config.endpoint.trim_end_matches('/')))
            .header("Content-Type", "application/json")
            .bearer_auth(&self.config.api_key)
            .json(&serde_json::json!({
                "messages": [{"role": "user", "content": messages}],
                "user_id": self.config.user_id,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::warn!(status = %status, body = %body, "[mem0_store] store failed");
            anyhow::bail!("mem0 add returned {}: {}", status, body);
        }

        let result: serde_json::Value = response.json().await?;
        let memory_ids = result["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| r["id"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        tracing::info!(
            facts_stored = memory_ids.len(),
            "[mem0_store] stored facts"
        );

        Ok(StoreResult {
            facts_stored: memory_ids.len(),
            memory_ids,
        })
    }
}

/// Format recalled memories into a prompt section that can be injected
/// into the system prompt before the LLM call.
///
/// This follows mem0's recommended pattern: inject as a "## Relevant Memories"
/// section so the model knows this is persistent context that survived truncation.
pub fn format_recall_for_prompt(recall: &RecallResult) -> String {
    if recall.memories.is_empty() {
        return String::new();
    }

    let mut parts = vec!["## Relevant Memories (from persistent storage)".to_string()];
    parts.push("These memories were recalled from your long-term memory store. They persist across sessions and survive context truncation.".to_string());
    parts.push(String::new());

    for (i, mem) in recall.memories.iter().enumerate() {
        let timestamp = mem.created_at
            .as_ref()
            .map(|t| format!(" [{}]", t))
            .unwrap_or_default();
        parts.push(format!("{}. {}{}", i + 1, mem.memory, timestamp));
    }

    if recall.truncated {
        parts.push(String::new());
        parts.push("_(Some lower-relevance memories were truncated to fit the context budget)_".to_string());
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_empty_recall() {
        let recall = RecallResult {
            memories: vec![],
            query: "test".to_string(),
            total_chars: 0,
            truncated: false,
        };
        assert!(format_recall_for_prompt(&recall).is_empty());
    }

    #[test]
    fn format_recall_with_memories() {
        let recall = RecallResult {
            memories: vec![
                RecalledMemory {
                    id: "1".to_string(),
                    memory: "User prefers dark mode".to_string(),
                    score: 0.95,
                    created_at: Some("2026-05-22".to_string()),
                    updated_at: None,
                    metadata: None,
                },
                RecalledMemory {
                    id: "2".to_string(),
                    memory: "Working on hermes-harness project".to_string(),
                    score: 0.88,
                    created_at: None,
                    updated_at: None,
                    metadata: None,
                },
            ],
            query: "current work".to_string(),
            total_chars: 50,
            truncated: false,
        };
        let formatted = format_recall_for_prompt(&recall);
        assert!(formatted.contains("User prefers dark mode"));
        assert!(formatted.contains("hermes-harness"));
        assert!(formatted.contains("[2026-05-22]"));
    }

    #[test]
    fn default_config_disabled() {
        let config = Mem0Config::default();
        assert!(!config.enabled);
    }
}
