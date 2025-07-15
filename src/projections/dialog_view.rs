//! DialogView projection - the primary read model for dialog state
//!
//! This projection maintains a denormalized view of dialog data optimized
//! for UI display and quick queries.

use super::{DialogProjection, DialogStatistics, ParticipantSummary, TopicSummary, ContextSummary};
use crate::aggregate::{DialogStatus, DialogType, ConversationContext};
use crate::events::*;
use crate::value_objects::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Complete view of a dialog with all relevant information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogView {
    // Core identification
    pub dialog_id: Uuid,
    pub dialog_type: DialogType,
    pub status: DialogStatus,
    
    // Timestamps
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub last_activity: DateTime<Utc>,
    pub paused_at: Option<DateTime<Utc>>,
    
    // Participants
    pub participants: HashMap<String, ParticipantSummary>,
    pub active_participants: HashSet<String>,
    
    // Content
    pub turns: Vec<Turn>,
    pub topics: HashMap<String, TopicSummary>,
    pub active_topics: HashSet<String>,
    
    // Context
    pub current_context: String,
    pub contexts: HashMap<String, ContextSummary>,
    pub context_variables: HashMap<String, HashMap<String, ContextVariable>>,
    
    // Metadata and metrics
    pub metadata: HashMap<String, serde_json::Value>,
    pub statistics: DialogStatistics,
    
    // Search/query optimization
    pub tags: HashSet<String>,
    pub keywords: HashSet<String>,
}

impl DialogView {
    /// Create a new dialog view from a started event
    pub fn new(event: &DialogStarted) -> Self {
        let mut participants = HashMap::new();
        let mut active_participants = HashSet::new();
        
        for participant in &event.participants {
            let participant_id = participant.id.clone();
            active_participants.insert(participant_id.clone());
            participants.insert(participant_id, ParticipantSummary {
                participant: participant.clone(),
                turn_count: 0,
                message_count: 0,
                first_turn_at: None,
                last_turn_at: None,
                topics_initiated: Vec::new(),
            });
        }
        
        let initial_context = ConversationContext {
            context_id: "default".to_string(),
            scope: ContextScope::Conversation,
            variables: HashMap::new(),
            parent_context: None,
        };
        
        let mut contexts = HashMap::new();
        contexts.insert("default".to_string(), ContextSummary {
            context_id: "default".to_string(),
            scope: ContextScope::Conversation,
            variable_count: 0,
            switches_to: 1,
            switches_from: 0,
            total_duration_seconds: 0,
        });
        
        Self {
            dialog_id: event.dialog_id,
            dialog_type: event.dialog_type.clone(),
            status: DialogStatus::Active,
            started_at: event.timestamp,
            ended_at: None,
            last_activity: event.timestamp,
            paused_at: None,
            participants,
            active_participants,
            turns: Vec::new(),
            topics: HashMap::new(),
            active_topics: HashSet::new(),
            current_context: "default".to_string(),
            contexts,
            context_variables: HashMap::new(),
            metadata: event.metadata.clone(),
            statistics: DialogStatistics {
                participant_count: event.participants.len(),
                ..Default::default()
            },
            tags: extract_tags(&event.metadata),
            keywords: HashSet::new(),
        }
    }
    
    /// Calculate engagement score based on dialog activity
    fn calculate_engagement_score(&self) -> f32 {
        let turn_frequency = if self.statistics.active_duration_seconds > 0 {
            self.statistics.total_turns as f32 / self.statistics.active_duration_seconds as f32
        } else {
            0.0
        };
        
        let participant_activity = self.participants.values()
            .map(|p| p.turn_count as f32 / self.statistics.total_turns.max(1) as f32)
            .sum::<f32>() / self.participants.len().max(1) as f32;
        
        let topic_completion = if self.statistics.topic_count > 0 {
            self.statistics.completed_topics as f32 / self.statistics.topic_count as f32
        } else {
            0.0
        };
        
        // Weighted average
        (turn_frequency * 0.3 + participant_activity * 0.4 + topic_completion * 0.3).min(1.0)
    }
}

impl DialogProjection for DialogView {
    fn apply_event(&mut self, event: &DialogDomainEvent) {
        match event {
            DialogDomainEvent::Started(e) => {
                // Already handled in new()
            }
            
            DialogDomainEvent::TurnAdded(e) => {
                self.turns.push(e.turn.clone());
                self.last_activity = e.timestamp;
                self.statistics.total_turns += 1;
                self.statistics.total_messages += e.turn.messages.len();
                
                // Update participant stats
                if let Some(participant) = self.participants.get_mut(&e.turn.participant_id) {
                    participant.turn_count += 1;
                    participant.message_count += e.turn.messages.len();
                    participant.last_turn_at = Some(e.timestamp);
                    if participant.first_turn_at.is_none() {
                        participant.first_turn_at = Some(e.timestamp);
                    }
                }
                
                // Update topic stats
                if let Some(topic_id) = &e.turn.topic_id {
                    if let Some(topic) = self.topics.get_mut(topic_id) {
                        topic.turn_count += 1;
                    }
                }
                
                // Extract keywords from messages
                for message in &e.turn.messages {
                    self.keywords.extend(extract_keywords(&message.content));
                }
                
                // Update average turn length
                let total_length: usize = self.turns.iter()
                    .map(|t| t.messages.iter().map(|m| m.content.len()).sum::<usize>())
                    .sum();
                self.statistics.average_turn_length = total_length as f32 / self.statistics.total_turns.max(1) as f32;
            }
            
            DialogDomainEvent::ParticipantAdded(e) => {
                self.active_participants.insert(e.participant.id.clone());
                self.participants.insert(e.participant.id.clone(), ParticipantSummary {
                    participant: e.participant.clone(),
                    turn_count: 0,
                    message_count: 0,
                    first_turn_at: None,
                    last_turn_at: None,
                    topics_initiated: Vec::new(),
                });
                self.statistics.participant_count += 1;
                self.last_activity = e.timestamp;
            }
            
            DialogDomainEvent::ParticipantRemoved(e) => {
                self.active_participants.remove(&e.participant_id);
                self.last_activity = e.timestamp;
            }
            
            DialogDomainEvent::TopicCompleted(e) => {
                if let Some(topic) = self.topics.get_mut(&e.topic_id) {
                    topic.completed_at = Some(e.timestamp);
                    self.active_topics.remove(&e.topic_id);
                    self.statistics.completed_topics += 1;
                }
                self.last_activity = e.timestamp;
            }
            
            DialogDomainEvent::ContextSwitched(e) => {
                // Update context duration
                if let Some(old_context) = self.contexts.get_mut(&self.current_context) {
                    old_context.switches_from += 1;
                }
                
                self.current_context = e.new_context.context_id.clone();
                
                self.contexts.entry(e.new_context.context_id.clone())
                    .or_insert_with(|| ContextSummary {
                        context_id: e.new_context.context_id.clone(),
                        scope: e.new_context.scope.clone(),
                        variable_count: e.new_context.variables.len(),
                        switches_to: 0,
                        switches_from: 0,
                        total_duration_seconds: 0,
                    })
                    .switches_to += 1;
                    
                self.last_activity = e.timestamp;
            }
            
            DialogDomainEvent::ContextVariableAdded(e) => {
                let context_vars = self.context_variables
                    .entry(e.context_id.clone())
                    .or_insert_with(HashMap::new);
                context_vars.insert(e.variable.name.clone(), e.variable.clone());
                
                if let Some(context) = self.contexts.get_mut(&e.context_id) {
                    context.variable_count += 1;
                }
                
                self.last_activity = e.timestamp;
            }
            
            DialogDomainEvent::MetadataSet(e) => {
                self.metadata = e.metadata.clone();
                self.tags = extract_tags(&e.metadata);
                self.last_activity = e.timestamp;
            }
            
            DialogDomainEvent::ContextUpdated(e) => {
                // Update context variables
                self.context_variables.insert(
                    e.context.context_id.clone(),
                    e.context.variables.clone()
                );
                
                if let Some(context) = self.contexts.get_mut(&e.context.context_id) {
                    context.variable_count = e.context.variables.len();
                }
                
                self.last_activity = e.timestamp;
            }
            
            DialogDomainEvent::Paused(e) => {
                self.status = DialogStatus::Paused;
                self.paused_at = Some(e.timestamp);
                self.last_activity = e.timestamp;
            }
            
            DialogDomainEvent::Resumed(e) => {
                self.status = DialogStatus::Active;
                if let Some(paused_at) = self.paused_at {
                    let pause_duration = e.timestamp.signed_duration_since(paused_at);
                    self.statistics.pause_duration_seconds += pause_duration.num_seconds().max(0) as u64;
                }
                self.paused_at = None;
                self.last_activity = e.timestamp;
            }
            
            DialogDomainEvent::Ended(e) => {
                self.status = DialogStatus::Completed;
                self.ended_at = Some(e.timestamp);
                self.last_activity = e.timestamp;
                
                // Calculate final statistics
                let total_duration = e.timestamp.signed_duration_since(self.started_at);
                self.statistics.active_duration_seconds = 
                    (total_duration.num_seconds().max(0) as u64) - self.statistics.pause_duration_seconds;
                self.statistics.engagement_score = self.calculate_engagement_score();
            }
        }
    }
    
    fn id(&self) -> &str {
        // Use dialog_id as string slice
        &self.dialog_id.to_string()
    }
}

/// Repository for managing dialog views
#[async_trait]
pub trait DialogViewRepository: Send + Sync {
    /// Save or update a dialog view
    async fn save(&self, view: DialogView) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Get a dialog view by ID
    async fn get(&self, dialog_id: &Uuid) -> Result<Option<DialogView>, Box<dyn std::error::Error>>;
    
    /// Get all active dialogs
    async fn get_active(&self) -> Result<Vec<DialogView>, Box<dyn std::error::Error>>;
    
    /// Get dialogs by participant
    async fn get_by_participant(&self, participant_id: &str) -> Result<Vec<DialogView>, Box<dyn std::error::Error>>;
    
    /// Search dialogs by metadata
    async fn search(&self, criteria: SearchCriteria) -> Result<Vec<DialogView>, Box<dyn std::error::Error>>;
}

/// In-memory implementation of DialogViewRepository
pub struct InMemoryDialogViewRepository {
    views: Arc<RwLock<HashMap<Uuid, DialogView>>>,
}

impl InMemoryDialogViewRepository {
    pub fn new() -> Self {
        Self {
            views: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl DialogViewRepository for InMemoryDialogViewRepository {
    async fn save(&self, view: DialogView) -> Result<(), Box<dyn std::error::Error>> {
        let mut views = self.views.write().await;
        views.insert(view.dialog_id, view);
        Ok(())
    }
    
    async fn get(&self, dialog_id: &Uuid) -> Result<Option<DialogView>, Box<dyn std::error::Error>> {
        let views = self.views.read().await;
        Ok(views.get(dialog_id).cloned())
    }
    
    async fn get_active(&self) -> Result<Vec<DialogView>, Box<dyn std::error::Error>> {
        let views = self.views.read().await;
        Ok(views.values()
            .filter(|v| v.status == DialogStatus::Active)
            .cloned()
            .collect())
    }
    
    async fn get_by_participant(&self, participant_id: &str) -> Result<Vec<DialogView>, Box<dyn std::error::Error>> {
        let views = self.views.read().await;
        Ok(views.values()
            .filter(|v| v.participants.contains_key(participant_id))
            .cloned()
            .collect())
    }
    
    async fn search(&self, criteria: SearchCriteria) -> Result<Vec<DialogView>, Box<dyn std::error::Error>> {
        let views = self.views.read().await;
        Ok(views.values()
            .filter(|v| criteria.matches(v))
            .cloned()
            .collect())
    }
}

/// Search criteria for dialogs
#[derive(Debug, Clone)]
pub struct SearchCriteria {
    pub status: Option<DialogStatus>,
    pub dialog_type: Option<DialogType>,
    pub participant_ids: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
    pub started_after: Option<DateTime<Utc>>,
    pub started_before: Option<DateTime<Utc>>,
}

impl SearchCriteria {
    fn matches(&self, view: &DialogView) -> bool {
        if let Some(status) = &self.status {
            if view.status != *status {
                return false;
            }
        }
        
        if let Some(dialog_type) = &self.dialog_type {
            if view.dialog_type != *dialog_type {
                return false;
            }
        }
        
        if let Some(participant_ids) = &self.participant_ids {
            if !participant_ids.iter().any(|id| view.participants.contains_key(id)) {
                return false;
            }
        }
        
        if let Some(tags) = &self.tags {
            if !tags.iter().all(|tag| view.tags.contains(tag)) {
                return false;
            }
        }
        
        if let Some(keywords) = &self.keywords {
            if !keywords.iter().any(|kw| view.keywords.contains(kw)) {
                return false;
            }
        }
        
        if let Some(after) = &self.started_after {
            if view.started_at < *after {
                return false;
            }
        }
        
        if let Some(before) = &self.started_before {
            if view.started_at > *before {
                return false;
            }
        }
        
        true
    }
}

// Helper functions
fn extract_tags(metadata: &HashMap<String, serde_json::Value>) -> HashSet<String> {
    let mut tags = HashSet::new();
    
    if let Some(tags_value) = metadata.get("tags") {
        if let Some(tags_array) = tags_value.as_array() {
            for tag in tags_array {
                if let Some(tag_str) = tag.as_str() {
                    tags.insert(tag_str.to_string());
                }
            }
        }
    }
    
    tags
}

fn extract_keywords(content: &MessageContent) -> HashSet<String> {
    // Simple keyword extraction - in production, use NLP
    match content {
        MessageContent::Text(text) => {
            text.split_whitespace()
                .filter(|w| w.len() > 3)
                .map(|w| w.to_lowercase())
                .collect()
        }
        _ => HashSet::new(),
    }
}