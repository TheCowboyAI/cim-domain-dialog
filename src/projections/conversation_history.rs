//! ConversationHistory projection - optimized for message retrieval and search
//!
//! This projection maintains a searchable history of all conversation messages
//! with efficient pagination and filtering capabilities.

use super::DialogProjection;
use crate::events::*;
use crate::value_objects::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// A single message entry in the conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub entry_id: Uuid,
    pub dialog_id: Uuid,
    pub turn_id: Uuid,
    pub message_index: usize,
    pub participant_id: String,
    pub participant_name: String,
    pub participant_type: ParticipantType,
    pub message: Message,
    pub timestamp: DateTime<Utc>,
    pub context_id: String,
    pub topic_id: Option<String>,
    pub topic_name: Option<String>,
    pub metadata: TurnMetadata,
    pub sequence_number: u64,
}

/// Conversation history projection
#[derive(Debug, Clone)]
pub struct ConversationHistory {
    pub dialog_id: Uuid,
    pub entries: Vec<HistoryEntry>,
    pub participant_index: HashMap<String, Vec<usize>>,
    pub topic_index: HashMap<String, Vec<usize>>,
    pub context_index: HashMap<String, Vec<usize>>,
    pub total_messages: u64,
    pub last_sequence: u64,
}

impl ConversationHistory {
    pub fn new(dialog_id: Uuid) -> Self {
        Self {
            dialog_id,
            entries: Vec::new(),
            participant_index: HashMap::new(),
            topic_index: HashMap::new(),
            context_index: HashMap::new(),
            total_messages: 0,
            last_sequence: 0,
        }
    }
    
    /// Get messages for a specific participant
    pub fn get_by_participant(&self, participant_id: &str) -> Vec<&HistoryEntry> {
        self.participant_index.get(participant_id)
            .map(|indices| {
                indices.iter()
                    .filter_map(|&idx| self.entries.get(idx))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get messages for a specific topic
    pub fn get_by_topic(&self, topic_id: &str) -> Vec<&HistoryEntry> {
        self.topic_index.get(topic_id)
            .map(|indices| {
                indices.iter()
                    .filter_map(|&idx| self.entries.get(idx))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get messages in a time range
    pub fn get_by_time_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&HistoryEntry> {
        self.entries.iter()
            .filter(|entry| entry.timestamp >= start && entry.timestamp <= end)
            .collect()
    }
    
    /// Get paginated messages
    pub fn get_page(&self, offset: usize, limit: usize) -> &[HistoryEntry] {
        let start = offset.min(self.entries.len());
        let end = (start + limit).min(self.entries.len());
        &self.entries[start..end]
    }
    
    /// Search messages by content
    pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
        let query_lower = query.to_lowercase();
        self.entries.iter()
            .filter(|entry| {
                match &entry.message.content {
                    MessageContent::Text(text) => text.to_lowercase().contains(&query_lower),
                    MessageContent::Code { code, .. } => code.to_lowercase().contains(&query_lower),
                    MessageContent::Structured { template, .. } => template.to_lowercase().contains(&query_lower),
                    _ => false,
                }
            })
            .collect()
    }
}

impl DialogProjection for ConversationHistory {
    fn apply_event(&mut self, event: &DialogDomainEvent) {
        match event {
            DialogDomainEvent::TurnAdded(e) => {
                let turn = &e.turn;
                let current_context = e.current_context.as_deref().unwrap_or("default");
                
                // Get participant info (would be enriched from participant store in production)
                let participant_name = format!("Participant {}", &turn.participant_id);
                let participant_type = ParticipantType::User; // Default, would be looked up
                
                // Get topic info if available
                let (topic_id, topic_name) = if let Some(tid) = &turn.topic_id {
                    (Some(tid.clone()), Some(format!("Topic {}", tid)))
                } else {
                    (None, None)
                };
                
                // Add each message as a history entry
                for (idx, message) in turn.messages.iter().enumerate() {
                    self.last_sequence += 1;
                    
                    let entry = HistoryEntry {
                        entry_id: Uuid::new_v4(),
                        dialog_id: e.dialog_id,
                        turn_id: turn.turn_id,
                        message_index: idx,
                        participant_id: turn.participant_id.clone(),
                        participant_name: participant_name.clone(),
                        participant_type: participant_type.clone(),
                        message: message.clone(),
                        timestamp: e.timestamp,
                        context_id: current_context.to_string(),
                        topic_id: topic_id.clone(),
                        topic_name: topic_name.clone(),
                        metadata: turn.metadata.clone(),
                        sequence_number: self.last_sequence,
                    };
                    
                    let entry_index = self.entries.len();
                    
                    // Update indices
                    self.participant_index
                        .entry(turn.participant_id.clone())
                        .or_insert_with(Vec::new)
                        .push(entry_index);
                    
                    if let Some(tid) = &topic_id {
                        self.topic_index
                            .entry(tid.clone())
                            .or_insert_with(Vec::new)
                            .push(entry_index);
                    }
                    
                    self.context_index
                        .entry(current_context.to_string())
                        .or_insert_with(Vec::new)
                        .push(entry_index);
                    
                    self.entries.push(entry);
                    self.total_messages += 1;
                }
            }
            _ => {} // Other events don't affect history
        }
    }
    
    fn id(&self) -> &str {
        &self.dialog_id.to_string()
    }
}

/// Repository for conversation history
#[async_trait]
pub trait ConversationHistoryRepository: Send + Sync {
    /// Save or update conversation history
    async fn save(&self, history: ConversationHistory) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Get conversation history by dialog ID
    async fn get(&self, dialog_id: &Uuid) -> Result<Option<ConversationHistory>, Box<dyn std::error::Error>>;
    
    /// Get history entries across all dialogs for a participant
    async fn get_participant_history(
        &self, 
        participant_id: &str,
        limit: usize,
    ) -> Result<Vec<HistoryEntry>, Box<dyn std::error::Error>>;
    
    /// Search across all conversation histories
    async fn search_all(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<HistoryEntry>, Box<dyn std::error::Error>>;
}

/// In-memory implementation
pub struct InMemoryConversationHistoryRepository {
    histories: Arc<RwLock<HashMap<Uuid, ConversationHistory>>>,
}

impl InMemoryConversationHistoryRepository {
    pub fn new() -> Self {
        Self {
            histories: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ConversationHistoryRepository for InMemoryConversationHistoryRepository {
    async fn save(&self, history: ConversationHistory) -> Result<(), Box<dyn std::error::Error>> {
        let mut histories = self.histories.write().await;
        histories.insert(history.dialog_id, history);
        Ok(())
    }
    
    async fn get(&self, dialog_id: &Uuid) -> Result<Option<ConversationHistory>, Box<dyn std::error::Error>> {
        let histories = self.histories.read().await;
        Ok(histories.get(dialog_id).cloned())
    }
    
    async fn get_participant_history(
        &self, 
        participant_id: &str,
        limit: usize,
    ) -> Result<Vec<HistoryEntry>, Box<dyn std::error::Error>> {
        let histories = self.histories.read().await;
        let mut all_entries: Vec<HistoryEntry> = histories.values()
            .flat_map(|h| h.get_by_participant(participant_id))
            .cloned()
            .collect();
        
        // Sort by timestamp descending
        all_entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all_entries.truncate(limit);
        
        Ok(all_entries)
    }
    
    async fn search_all(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<HistoryEntry>, Box<dyn std::error::Error>> {
        let histories = self.histories.read().await;
        let mut all_results: Vec<HistoryEntry> = histories.values()
            .flat_map(|h| h.search(query))
            .cloned()
            .collect();
        
        // Sort by timestamp descending
        all_results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all_results.truncate(limit);
        
        Ok(all_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_conversation_history() {
        let dialog_id = Uuid::new_v4();
        let mut history = ConversationHistory::new(dialog_id);
        
        // Create a turn added event
        let turn = Turn {
            turn_id: Uuid::new_v4(),
            turn_type: TurnType::Message,
            participant_id: "user1".to_string(),
            messages: vec![
                Message {
                    content: MessageContent::Text("Hello world".to_string()),
                    intent: MessageIntent::Statement,
                    confidence: 1.0,
                    metadata: HashMap::new(),
                }
            ],
            topic_id: Some("topic1".to_string()),
            metadata: TurnMetadata {
                duration_ms: Some(100),
                tokens_used: Some(10),
                model_used: None,
                error: None,
            },
            timestamp: Utc::now(),
        };
        
        let event = DialogDomainEvent::TurnAdded(TurnAdded {
            dialog_id,
            turn: turn.clone(),
            turn_number: 1,
            current_context: Some("default".to_string()),
            timestamp: Utc::now(),
        });
        
        history.apply_event(&event);
        
        assert_eq!(history.total_messages, 1);
        assert_eq!(history.entries.len(), 1);
        assert_eq!(history.get_by_participant("user1").len(), 1);
        assert_eq!(history.get_by_topic("topic1").len(), 1);
        
        let search_results = history.search("hello");
        assert_eq!(search_results.len(), 1);
    }
}