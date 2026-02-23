use crate::{MemoryEntry, MemoryError, MemoryQuery, MemoryStore};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Turn — a single message in a conversation
// ---------------------------------------------------------------------------

/// Role of a conversation participant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A single turn in a multi-turn conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    /// Who sent this message.
    pub role: Role,
    /// The message content.
    pub content: String,
    /// Optional tool call id (for tool role).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Arbitrary metadata (e.g. token count, model used).
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    /// When this turn was recorded.
    pub timestamp: chrono::DateTime<Utc>,
}

impl Turn {
    /// Create a new turn.
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            tool_call_id: None,
            metadata: std::collections::HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }

    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        let mut t = Self::new(Role::Tool, content);
        t.tool_call_id = Some(tool_call_id.into());
        t
    }
}

// ---------------------------------------------------------------------------
// ConversationMemory — session-scoped multi-turn history
// ---------------------------------------------------------------------------

/// Manages multi-turn conversation history on top of a [`MemoryStore`].
///
/// Each session is stored as a single [`MemoryEntry`] containing a
/// JSON array of [`Turn`]s. This keeps the store schema simple while
/// enabling efficient retrieval of full conversation context.
pub struct ConversationMemory {
    store: Arc<dyn MemoryStore>,
    /// Maximum turns to keep per session (oldest are dropped).
    max_turns: usize,
}

impl ConversationMemory {
    /// Create a new conversation memory manager.
    pub fn new(store: Arc<dyn MemoryStore>) -> Self {
        Self {
            store,
            max_turns: 100,
        }
    }

    /// Set the maximum number of turns to retain per session.
    pub fn with_max_turns(mut self, max: usize) -> Self {
        self.max_turns = max;
        self
    }

    fn session_key(session_id: &str) -> String {
        format!("conversation:{session_id}")
    }

    /// Add a turn to a session.
    pub async fn add_turn(&self, session_id: &str, turn: Turn) -> Result<(), MemoryError> {
        let key = Self::session_key(session_id);
        let mut turns = self.get_turns(session_id).await?;
        turns.push(turn);

        // Trim to max_turns (keep most recent).
        if turns.len() > self.max_turns {
            let skip = turns.len() - self.max_turns;
            turns = turns.into_iter().skip(skip).collect();
        }

        let value = serde_json::to_value(&turns)
            .map_err(|e| MemoryError::serialization(e.to_string()))?;

        let entry = MemoryEntry::new(key, value).with_namespace("conversations");
        self.store.store(entry).await
    }

    /// Get all turns for a session.
    pub async fn get_turns(&self, session_id: &str) -> Result<Vec<Turn>, MemoryError> {
        let key = Self::session_key(session_id);
        match self.store.get(&key).await? {
            Some(entry) => {
                let turns: Vec<Turn> = serde_json::from_value(entry.value)
                    .map_err(|e| MemoryError::serialization(e.to_string()))?;
                Ok(turns)
            }
            None => Ok(Vec::new()),
        }
    }

    /// Get the last N turns for a session.
    pub async fn get_recent_turns(
        &self,
        session_id: &str,
        n: usize,
    ) -> Result<Vec<Turn>, MemoryError> {
        let turns = self.get_turns(session_id).await?;
        let start = turns.len().saturating_sub(n);
        Ok(turns[start..].to_vec())
    }

    /// Clear a session's history.
    pub async fn clear_session(&self, session_id: &str) -> Result<(), MemoryError> {
        let key = Self::session_key(session_id);
        self.store.delete(&key).await?;
        Ok(())
    }

    /// List all conversation session ids.
    pub async fn list_sessions(&self) -> Result<Vec<String>, MemoryError> {
        let query = MemoryQuery::new()
            .with_namespace("conversations")
            .with_key_prefix("conversation:");
        let entries = self.store.list(&query).await?;
        Ok(entries
            .into_iter()
            .map(|e| e.key.strip_prefix("conversation:").unwrap_or(&e.key).to_string())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::InMemoryStore;

    #[tokio::test]
    async fn test_conversation_memory_basic() {
        let store = Arc::new(InMemoryStore::new());
        let conv = ConversationMemory::new(store);

        conv.add_turn("s1", Turn::user("Hello")).await.unwrap();
        conv.add_turn("s1", Turn::assistant("Hi there!"))
            .await
            .unwrap();

        let turns = conv.get_turns("s1").await.unwrap();
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].role, Role::User);
        assert_eq!(turns[1].role, Role::Assistant);
    }

    #[tokio::test]
    async fn test_conversation_memory_max_turns() {
        let store = Arc::new(InMemoryStore::new());
        let conv = ConversationMemory::new(store).with_max_turns(3);

        for i in 0..5 {
            conv.add_turn("s2", Turn::user(format!("msg {i}")))
                .await
                .unwrap();
        }

        let turns = conv.get_turns("s2").await.unwrap();
        assert_eq!(turns.len(), 3);
        assert_eq!(turns[0].content, "msg 2");
    }

    #[tokio::test]
    async fn test_conversation_memory_clear() {
        let store = Arc::new(InMemoryStore::new());
        let conv = ConversationMemory::new(store);

        conv.add_turn("s3", Turn::user("test")).await.unwrap();
        conv.clear_session("s3").await.unwrap();

        let turns = conv.get_turns("s3").await.unwrap();
        assert!(turns.is_empty());
    }

    #[tokio::test]
    async fn test_conversation_recent_turns() {
        let store = Arc::new(InMemoryStore::new());
        let conv = ConversationMemory::new(store);

        for i in 0..10 {
            conv.add_turn("s4", Turn::user(format!("msg {i}")))
                .await
                .unwrap();
        }

        let recent = conv.get_recent_turns("s4", 3).await.unwrap();
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].content, "msg 7");
    }
}
