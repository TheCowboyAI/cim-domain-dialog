//! Dialog projections for read models
//!
//! This module provides optimized read models for dialog data, supporting
//! efficient queries and real-time updates through event sourcing.

use crate::events::*;
use crate::value_objects::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod simple_projection;
// pub mod dialog_view;
// pub mod conversation_history;
// pub mod active_dialogs;
// pub mod projection_updater;

pub use simple_projection::{SimpleDialogView, SimpleProjectionUpdater};
// pub use dialog_view::{DialogView, DialogViewRepository};
// pub use conversation_history::{ConversationHistory, ConversationHistoryRepository};
// pub use active_dialogs::{ActiveDialogs, ActiveDialogsRepository};
// pub use projection_updater::DialogProjectionUpdater;

/// Common trait for dialog projections
pub trait DialogProjection: Send + Sync {
    /// Update the projection based on an event
    fn apply_event(&mut self, event: &DialogDomainEvent);
    
    /// Get the projection ID
    fn id(&self) -> &str;
}

/// Summary statistics for a dialog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogStatistics {
    pub total_turns: usize,
    pub total_messages: usize,
    pub participant_count: usize,
    pub topic_count: usize,
    pub completed_topics: usize,
    pub active_duration_seconds: u64,
    pub pause_duration_seconds: u64,
    pub average_turn_length: f32,
    pub engagement_score: f32,
}

impl Default for DialogStatistics {
    fn default() -> Self {
        Self {
            total_turns: 0,
            total_messages: 0,
            participant_count: 0,
            topic_count: 0,
            completed_topics: 0,
            active_duration_seconds: 0,
            pause_duration_seconds: 0,
            average_turn_length: 0.0,
            engagement_score: 0.0,
        }
    }
}

/// Participant summary in a dialog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantSummary {
    pub participant: Participant,
    pub turn_count: usize,
    pub message_count: usize,
    pub first_turn_at: Option<DateTime<Utc>>,
    pub last_turn_at: Option<DateTime<Utc>>,
    pub topics_initiated: Vec<String>,
}

/// Topic summary in a dialog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicSummary {
    pub topic: Topic,
    pub turn_count: usize,
    pub participant_count: usize,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub relevance_scores: Vec<f32>,
}

/// Context state summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSummary {
    pub context_id: String,
    pub scope: ContextScope,
    pub variable_count: usize,
    pub switches_to: usize,
    pub switches_from: usize,
    pub total_duration_seconds: u64,
}