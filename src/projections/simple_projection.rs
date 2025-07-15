//! Simple projection implementation for Dialog domain
//!
//! This provides a working projection system that matches the actual event structure

use crate::events::*;
use crate::aggregate::{DialogStatus, DialogType};
use crate::value_objects::{Participant, Turn, ConversationMetrics};
use cim_domain::DomainEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Simple dialog view projection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleDialogView {
    pub dialog_id: Uuid,
    pub dialog_type: DialogType,
    pub status: DialogStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub primary_participant: Participant,
    pub participants: HashMap<String, Participant>,
    pub turns: Vec<Turn>,
    pub metrics: Option<ConversationMetrics>,
}

impl SimpleDialogView {
    /// Create from a DialogStarted event
    pub fn from_started(event: &DialogStarted) -> Self {
        let mut participants = HashMap::new();
        participants.insert(
            event.primary_participant.id.to_string(),
            event.primary_participant.clone(),
        );

        Self {
            dialog_id: event.dialog_id,
            dialog_type: event.dialog_type.clone(),
            status: DialogStatus::Active,
            started_at: event.started_at,
            ended_at: None,
            primary_participant: event.primary_participant.clone(),
            participants,
            turns: Vec::new(),
            metrics: None,
        }
    }

    /// Apply an event to update the view
    pub fn apply_event(&mut self, event: &DialogDomainEvent) {
        match event {
            DialogDomainEvent::DialogStarted(_) => {
                // Already handled in from_started
            }
            DialogDomainEvent::DialogEnded(e) => {
                self.status = DialogStatus::Ended;
                self.ended_at = Some(e.ended_at);
                self.metrics = Some(e.final_metrics.clone());
            }
            DialogDomainEvent::DialogPaused(_) => {
                self.status = DialogStatus::Paused;
            }
            DialogDomainEvent::DialogResumed(_) => {
                self.status = DialogStatus::Active;
            }
            DialogDomainEvent::TurnAdded(e) => {
                self.turns.push(e.turn.clone());
            }
            DialogDomainEvent::ParticipantAdded(e) => {
                self.participants.insert(
                    e.participant.id.to_string(),
                    e.participant.clone(),
                );
            }
            DialogDomainEvent::ParticipantRemoved(e) => {
                self.participants.remove(&e.participant_id.to_string());
            }
            DialogDomainEvent::TopicCompleted(_) => {
                // Topic tracking could be added here
            }
            _ => {
                // Handle other events as needed
            }
        }
    }
}

/// Simple projection updater
pub struct SimpleProjectionUpdater {
    views: HashMap<Uuid, SimpleDialogView>,
}

impl SimpleProjectionUpdater {
    pub fn new() -> Self {
        Self {
            views: HashMap::new(),
        }
    }

    /// Handle a domain event
    pub async fn handle_event(&mut self, event: DialogDomainEvent) -> Result<(), Box<dyn std::error::Error>> {
        let dialog_id = event.aggregate_id();

        match &event {
            DialogDomainEvent::DialogStarted(e) => {
                let view = SimpleDialogView::from_started(e);
                self.views.insert(dialog_id, view);
            }
            _ => {
                if let Some(view) = self.views.get_mut(&dialog_id) {
                    view.apply_event(&event);
                }
            }
        }

        Ok(())
    }

    /// Get a dialog view
    pub fn get_view(&self, dialog_id: &Uuid) -> Option<&SimpleDialogView> {
        self.views.get(dialog_id)
    }

    /// Get all active dialogs
    pub fn get_active_dialogs(&self) -> Vec<&SimpleDialogView> {
        self.views
            .values()
            .filter(|v| v.status == DialogStatus::Active)
            .collect()
    }
    
    /// Get all dialogs
    pub fn get_all_dialogs(&self) -> Vec<&SimpleDialogView> {
        self.views.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_projection() {
        let mut updater = SimpleProjectionUpdater::new();

        // Create a dialog started event
        let dialog_id = Uuid::new_v4();
        let event = DialogDomainEvent::DialogStarted(DialogStarted {
            dialog_id,
            dialog_type: DialogType::Support,
            primary_participant: Participant {
                id: Uuid::new_v4(),
                participant_type: ParticipantType::Human,
                role: ParticipantRole::Primary,
                name: "User 1".to_string(),
                metadata: HashMap::new(),
            },
            started_at: Utc::now(),
        });

        // Handle the event
        updater.handle_event(event).await.unwrap();

        // Check the view was created
        let view = updater.get_view(&dialog_id).unwrap();
        assert_eq!(view.dialog_id, dialog_id);
        assert_eq!(view.status, DialogStatus::Active);
        assert_eq!(view.participants.len(), 1);
    }
}