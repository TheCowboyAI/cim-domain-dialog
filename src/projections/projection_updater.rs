//! Projection updater for Dialog domain
//!
//! This module handles updating all dialog projections in response to domain events,
//! ensuring consistency across all read models.

use super::{
    DialogProjection, DialogView, ConversationHistory,
    DialogViewRepository, ConversationHistoryRepository, ActiveDialogsRepository,
};
use crate::events::DialogDomainEvent;
// Removed DialogEventHandler import - it's a struct, not a trait
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, error};
use uuid::Uuid;

/// Handles updating all dialog projections
pub struct DialogProjectionUpdater {
    dialog_view_repo: Arc<dyn DialogViewRepository>,
    conversation_history_repo: Arc<dyn ConversationHistoryRepository>,
    active_dialogs_repo: Arc<dyn ActiveDialogsRepository>,
}

impl DialogProjectionUpdater {
    pub fn new(
        dialog_view_repo: Arc<dyn DialogViewRepository>,
        conversation_history_repo: Arc<dyn ConversationHistoryRepository>,
        active_dialogs_repo: Arc<dyn ActiveDialogsRepository>,
    ) -> Self {
        Self {
            dialog_view_repo,
            conversation_history_repo,
            active_dialogs_repo,
        }
    }
    
    /// Update all projections for a dialog event
    async fn update_projections(&self, event: &DialogDomainEvent) -> Result<(), Box<dyn std::error::Error>> {
        let dialog_id = match event {
            DialogDomainEvent::Started(e) => e.dialog_id,
            DialogDomainEvent::TurnAdded(e) => e.dialog_id,
            DialogDomainEvent::ParticipantAdded(e) => e.dialog_id,
            DialogDomainEvent::ParticipantRemoved(e) => e.dialog_id,
            DialogDomainEvent::TopicCompleted(e) => e.dialog_id,
            DialogDomainEvent::ContextSwitched(e) => e.dialog_id,
            DialogDomainEvent::ContextVariableAdded(e) => e.dialog_id,
            DialogDomainEvent::MetadataSet(e) => e.dialog_id,
            DialogDomainEvent::ContextUpdated(e) => e.dialog_id,
            DialogDomainEvent::Paused(e) => e.dialog_id,
            DialogDomainEvent::Resumed(e) => e.dialog_id,
            DialogDomainEvent::Ended(e) => e.dialog_id,
        };
        
        // Update DialogView
        let view_result = self.update_dialog_view(&dialog_id, event).await;
        if let Err(e) = view_result {
            error!("Failed to update dialog view for {}: {}", dialog_id, e);
        }
        
        // Update ConversationHistory
        let history_result = self.update_conversation_history(&dialog_id, event).await;
        if let Err(e) = history_result {
            error!("Failed to update conversation history for {}: {}", dialog_id, e);
        }
        
        // Update ActiveDialogs
        let active_result = self.update_active_dialogs(event).await;
        if let Err(e) = active_result {
            error!("Failed to update active dialogs: {}", e);
        }
        
        Ok(())
    }
    
    async fn update_dialog_view(
        &self,
        dialog_id: &Uuid,
        event: &DialogDomainEvent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut view = if let DialogDomainEvent::Started(e) = event {
            DialogView::new(e)
        } else {
            match self.dialog_view_repo.get(dialog_id).await? {
                Some(v) => v,
                None => {
                    error!("Dialog view not found for {}", dialog_id);
                    return Ok(());
                }
            }
        };
        
        view.apply_event(event);
        self.dialog_view_repo.save(view).await?;
        
        Ok(())
    }
    
    async fn update_conversation_history(
        &self,
        dialog_id: &Uuid,
        event: &DialogDomainEvent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut history = match self.conversation_history_repo.get(dialog_id).await? {
            Some(h) => h,
            None => ConversationHistory::new(*dialog_id),
        };
        
        history.apply_event(event);
        self.conversation_history_repo.save(history).await?;
        
        Ok(())
    }
    
    async fn update_active_dialogs(
        &self,
        event: &DialogDomainEvent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut active = self.active_dialogs_repo.get().await?;
        active.apply_event(event);
        self.active_dialogs_repo.save(active).await?;
        
        Ok(())
    }
}

impl DialogProjectionUpdater {
    /// Handle a domain event by updating all projections
    pub async fn handle_event(&self, event: DialogDomainEvent) -> Result<(), Box<dyn std::error::Error>> {
        info!("Updating projections for event: {:?}", event);
        self.update_projections(&event).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projections::{
        InMemoryDialogViewRepository,
        InMemoryConversationHistoryRepository,
        InMemoryActiveDialogsRepository,
    };
    use crate::aggregate::DialogType;
    use crate::events::DialogStarted;
    use crate::value_objects::{Participant, ParticipantType};
    use chrono::Utc;
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_projection_updater() {
        // Create repositories
        let dialog_view_repo = Arc::new(InMemoryDialogViewRepository::new());
        let conversation_history_repo = Arc::new(InMemoryConversationHistoryRepository::new());
        let active_dialogs_repo = Arc::new(InMemoryActiveDialogsRepository::new());
        
        // Create updater
        let updater = DialogProjectionUpdater::new(
            dialog_view_repo.clone(),
            conversation_history_repo.clone(),
            active_dialogs_repo.clone(),
        );
        
        // Create a dialog started event
        let dialog_id = Uuid::new_v4();
        let event = DialogDomainEvent::Started(DialogStarted {
            dialog_id,
            dialog_type: DialogType::Support,
            participants: vec![
                Participant {
                    id: "user1".to_string(),
                    participant_type: ParticipantType::User,
                    name: Some("User 1".to_string()),
                    metadata: HashMap::new(),
                }
            ],
            initial_context: None,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        });
        
        // Update projections
        updater.handle_event(event).await.unwrap();
        
        // Verify dialog view was created
        let view = dialog_view_repo.get(&dialog_id).await.unwrap();
        assert!(view.is_some());
        assert_eq!(view.unwrap().dialog_id, dialog_id);
        
        // Verify conversation history was created
        let history = conversation_history_repo.get(&dialog_id).await.unwrap();
        assert!(history.is_some());
        
        // Verify active dialogs was updated
        let active = active_dialogs_repo.get().await.unwrap();
        assert!(active.dialogs.contains_key(&dialog_id));
    }
}