//! Dialog domain queries for search and retrieval
//!
//! This module provides query capabilities for the Dialog domain,
//! enabling efficient search and retrieval of dialog data.

use crate::aggregate::{DialogStatus, DialogType};
use crate::projections::{SimpleDialogView, SimpleProjectionUpdater};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Query types for dialog domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogQuery {
    /// Get a specific dialog by ID
    GetDialogById { dialog_id: Uuid },
    
    /// Get all active dialogs
    GetActiveDialogs,
    
    /// Get dialogs by participant
    GetDialogsByParticipant { participant_id: String },
    
    /// Get dialogs by type
    GetDialogsByType { dialog_type: DialogType },
    
    /// Get dialogs by status
    GetDialogsByStatus { status: DialogStatus },
    
    /// Get dialogs in date range
    GetDialogsInDateRange {
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    },
    
    /// Search dialogs by text in messages
    SearchDialogsByText { search_text: String },
    
    /// Get dialog statistics
    GetDialogStatistics,
}

/// Query result for dialog queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogQueryResult {
    /// Single dialog result
    Dialog(Option<SimpleDialogView>),
    
    /// Multiple dialogs result
    Dialogs(Vec<SimpleDialogView>),
    
    /// Statistics result
    Statistics(DialogStatistics),
    
    /// Error result
    Error(String),
}

/// Dialog statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogStatistics {
    pub total_dialogs: usize,
    pub active_dialogs: usize,
    pub completed_dialogs: usize,
    pub paused_dialogs: usize,
    pub dialogs_by_type: Vec<(DialogType, usize)>,
    pub average_turn_count: f64,
    pub total_participants: usize,
}

/// Dialog query handler
pub struct DialogQueryHandler {
    projection_updater: Arc<RwLock<SimpleProjectionUpdater>>,
}

impl DialogQueryHandler {
    /// Create a new query handler
    pub fn new(projection_updater: Arc<RwLock<SimpleProjectionUpdater>>) -> Self {
        Self { projection_updater }
    }
    
    /// Execute a query
    pub async fn execute(&self, query: DialogQuery) -> DialogQueryResult {
        match query {
            DialogQuery::GetDialogById { dialog_id } => {
                self.get_dialog_by_id(dialog_id).await
            }
            DialogQuery::GetActiveDialogs => {
                self.get_active_dialogs().await
            }
            DialogQuery::GetDialogsByParticipant { participant_id } => {
                self.get_dialogs_by_participant(&participant_id).await
            }
            DialogQuery::GetDialogsByType { dialog_type } => {
                self.get_dialogs_by_type(dialog_type).await
            }
            DialogQuery::GetDialogsByStatus { status } => {
                self.get_dialogs_by_status(status).await
            }
            DialogQuery::GetDialogsInDateRange { start_date, end_date } => {
                self.get_dialogs_in_date_range(start_date, end_date).await
            }
            DialogQuery::SearchDialogsByText { search_text } => {
                self.search_dialogs_by_text(&search_text).await
            }
            DialogQuery::GetDialogStatistics => {
                self.get_dialog_statistics().await
            }
        }
    }
    
    async fn get_dialog_by_id(&self, dialog_id: Uuid) -> DialogQueryResult {
        let updater = self.projection_updater.read().await;
        let dialog = updater.get_view(&dialog_id).cloned();
        DialogQueryResult::Dialog(dialog)
    }
    
    async fn get_active_dialogs(&self) -> DialogQueryResult {
        let updater = self.projection_updater.read().await;
        let dialogs = updater.get_active_dialogs()
            .into_iter()
            .cloned()
            .collect();
        DialogQueryResult::Dialogs(dialogs)
    }
    
    async fn get_dialogs_by_participant(&self, participant_id: &str) -> DialogQueryResult {
        let updater = self.projection_updater.read().await;
        let dialogs = updater.get_all_dialogs()
            .into_iter()
            .filter(|d| d.participants.contains_key(participant_id))
            .cloned()
            .collect();
        DialogQueryResult::Dialogs(dialogs)
    }
    
    async fn get_dialogs_by_type(&self, dialog_type: DialogType) -> DialogQueryResult {
        let updater = self.projection_updater.read().await;
        let dialogs = updater.get_all_dialogs()
            .into_iter()
            .filter(|d| d.dialog_type == dialog_type)
            .cloned()
            .collect();
        DialogQueryResult::Dialogs(dialogs)
    }
    
    async fn get_dialogs_by_status(&self, status: DialogStatus) -> DialogQueryResult {
        let updater = self.projection_updater.read().await;
        let dialogs = updater.get_all_dialogs()
            .into_iter()
            .filter(|d| d.status == status)
            .cloned()
            .collect();
        DialogQueryResult::Dialogs(dialogs)
    }
    
    async fn get_dialogs_in_date_range(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> DialogQueryResult {
        let updater = self.projection_updater.read().await;
        let dialogs = updater.get_all_dialogs()
            .into_iter()
            .filter(|d| d.started_at >= start_date && d.started_at <= end_date)
            .cloned()
            .collect();
        DialogQueryResult::Dialogs(dialogs)
    }
    
    async fn search_dialogs_by_text(&self, search_text: &str) -> DialogQueryResult {
        let search_lower = search_text.to_lowercase();
        let updater = self.projection_updater.read().await;
        
        let dialogs = updater.get_all_dialogs()
            .into_iter()
            .filter(|d| {
                // Search in turn messages
                d.turns.iter().any(|turn| {
                    match &turn.message.content {
                        crate::value_objects::MessageContent::Text(text) => 
                            text.to_lowercase().contains(&search_lower),
                        crate::value_objects::MessageContent::Structured(value) => 
                            value.to_string().to_lowercase().contains(&search_lower),
                        crate::value_objects::MessageContent::Multimodal { text, .. } => 
                            text.as_ref().map_or(false, |t| t.to_lowercase().contains(&search_lower)),
                    }
                })
            })
            .cloned()
            .collect();
            
        DialogQueryResult::Dialogs(dialogs)
    }
    
    async fn get_dialog_statistics(&self) -> DialogQueryResult {
        let updater = self.projection_updater.read().await;
        let all_dialogs = updater.get_all_dialogs();
        
        let total_dialogs = all_dialogs.len();
        let active_dialogs = all_dialogs.iter()
            .filter(|d| d.status == DialogStatus::Active)
            .count();
        let completed_dialogs = all_dialogs.iter()
            .filter(|d| d.status == DialogStatus::Ended)
            .count();
        let paused_dialogs = all_dialogs.iter()
            .filter(|d| d.status == DialogStatus::Paused)
            .count();
            
        // Count by type
        let mut type_counts = std::collections::HashMap::new();
        for dialog in &all_dialogs {
            *type_counts.entry(dialog.dialog_type.clone()).or_insert(0) += 1;
        }
        let dialogs_by_type: Vec<(DialogType, usize)> = type_counts.into_iter().collect();
        
        // Calculate average turn count
        let total_turns: usize = all_dialogs.iter().map(|d| d.turns.len()).sum();
        let average_turn_count = if total_dialogs > 0 {
            total_turns as f64 / total_dialogs as f64
        } else {
            0.0
        };
        
        // Count unique participants
        let mut unique_participants = std::collections::HashSet::new();
        for dialog in &all_dialogs {
            for participant_id in dialog.participants.keys() {
                unique_participants.insert(participant_id.clone());
            }
        }
        let total_participants = unique_participants.len();
        
        DialogQueryResult::Statistics(DialogStatistics {
            total_dialogs,
            active_dialogs,
            completed_dialogs,
            paused_dialogs,
            dialogs_by_type,
            average_turn_count,
            total_participants,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{DialogDomainEvent, DialogStarted};
    use crate::value_objects::{Participant, ParticipantType, ParticipantRole};
    
    #[tokio::test]
    async fn test_query_handler() {
        // Create projection updater
        let mut updater = SimpleProjectionUpdater::new();
        
        // Create a test dialog
        let dialog_id = Uuid::new_v4();
        let event = DialogDomainEvent::DialogStarted(DialogStarted {
            dialog_id,
            dialog_type: DialogType::Support,
            primary_participant: Participant {
                id: Uuid::new_v4(),
                participant_type: ParticipantType::Human,
                role: ParticipantRole::Primary,
                name: "Test User".to_string(),
                metadata: std::collections::HashMap::new(),
            },
            started_at: Utc::now(),
        });
        
        // Handle the event
        updater.handle_event(event).await.unwrap();
        
        // Create query handler
        let updater_arc = Arc::new(RwLock::new(updater));
        let handler = DialogQueryHandler::new(updater_arc);
        
        // Test get by ID
        let result = handler.execute(DialogQuery::GetDialogById { dialog_id }).await;
        match result {
            DialogQueryResult::Dialog(Some(dialog)) => {
                assert_eq!(dialog.dialog_id, dialog_id);
            }
            _ => panic!("Expected dialog result"),
        }
        
        // Test get active dialogs
        let result = handler.execute(DialogQuery::GetActiveDialogs).await;
        match result {
            DialogQueryResult::Dialogs(dialogs) => {
                assert_eq!(dialogs.len(), 1);
            }
            _ => panic!("Expected dialogs result"),
        }
        
        // Test statistics
        let result = handler.execute(DialogQuery::GetDialogStatistics).await;
        match result {
            DialogQueryResult::Statistics(stats) => {
                assert_eq!(stats.total_dialogs, 1);
                assert_eq!(stats.active_dialogs, 1);
            }
            _ => panic!("Expected statistics result"),
        }
    }
}