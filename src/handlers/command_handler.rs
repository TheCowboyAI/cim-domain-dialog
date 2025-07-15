//! Dialog command handler implementation

use cim_domain::{
    DomainError, DomainResult, EntityId, AggregateRepository,
};
use std::sync::Arc;
use chrono::Utc;

use crate::{
    aggregate::{Dialog, DialogMarker},
    commands::*,
    events::*,
    value_objects::ConversationMetrics,
};

/// Handler for dialog commands
pub struct DialogCommandHandler<R> 
where
    R: AggregateRepository<Dialog> + Send + Sync,
{
    repository: Arc<R>,
}

impl<R> DialogCommandHandler<R>
where
    R: AggregateRepository<Dialog> + Send + Sync,
{
    /// Create a new dialog command handler
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            repository,
        }
    }

    /// Handle StartDialog command
    pub fn handle_start_dialog(&self, cmd: StartDialog) -> DomainResult<Vec<DialogDomainEvent>> {
        // Create new dialog aggregate
        let mut dialog = Dialog::new(
            cmd.id,
            cmd.dialog_type,
            cmd.primary_participant.clone(),
        );

        let mut domain_events = vec![
            DialogDomainEvent::DialogStarted(DialogStarted {
                dialog_id: cmd.id,
                dialog_type: cmd.dialog_type,
                primary_participant: cmd.primary_participant,
                started_at: Utc::now(),
            })
        ];
        
        // Set metadata if provided
        if let Some(metadata) = cmd.metadata {
            for (key, value) in metadata {
                let _events = dialog.set_metadata(key.clone(), value.clone())
                    .map_err(|e| DomainError::ValidationError(e.to_string()))?;
                    
                // For now, we'll create the event manually since we can't downcast
                domain_events.push(DialogDomainEvent::DialogMetadataSet(DialogMetadataSet {
                    dialog_id: cmd.id,
                    key,
                    value,
                    set_at: Utc::now(),
                }));
            }
        }
        
        // Save aggregate
        self.repository.save(&dialog)
            .map_err(|e| DomainError::Generic(e))?;

        Ok(domain_events)
    }

    /// Handle EndDialog command
    pub fn handle_end_dialog(&self, cmd: EndDialog) -> DomainResult<Vec<DialogDomainEvent>> {
        // Load dialog aggregate
        let entity_id = EntityId::<DialogMarker>::from_uuid(cmd.id);
        let mut dialog = self.repository.load(entity_id)
            .map_err(|e| DomainError::Generic(e))?
            .ok_or_else(|| DomainError::EntityNotFound { 
                entity_type: "Dialog".to_string(),
                id: cmd.id.to_string(),
            })?;

        // End the dialog
        let _events = dialog.end(cmd.reason.clone())
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        // Save aggregate
        self.repository.save(&dialog)
            .map_err(|e| DomainError::Generic(e))?;
        
        // Create event manually
        let domain_events = vec![
            DialogDomainEvent::DialogEnded(DialogEnded {
                dialog_id: cmd.id,
                ended_at: Utc::now(),
                reason: cmd.reason,
                final_metrics: ConversationMetrics {
                    turn_count: dialog.turn_count() as u32,
                    avg_response_time_ms: 0.0,
                    topic_switches: 0,
                    clarification_count: 0,
                    sentiment_trend: 0.0,
                    coherence_score: 1.0,
                },
            })
        ];

        Ok(domain_events)
    }

    /// Handle AddTurn command
    pub fn handle_add_turn(&self, cmd: AddTurn) -> DomainResult<Vec<DialogDomainEvent>> {
        // Load dialog aggregate
        let entity_id = EntityId::<DialogMarker>::from_uuid(cmd.dialog_id);
        let mut dialog = self.repository.load(entity_id)
            .map_err(|e| DomainError::Generic(e))?
            .ok_or_else(|| DomainError::EntityNotFound { 
                entity_type: "Dialog".to_string(),
                id: cmd.dialog_id.to_string(),
            })?;

        // Get current turn count before adding
        let turn_number = (dialog.turn_count() + 1) as u32;
        
        // Add the turn
        let _events = dialog.add_turn(cmd.turn.clone())
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        // Save aggregate
        self.repository.save(&dialog)
            .map_err(|e| DomainError::Generic(e))?;
        
        // Create event manually
        let domain_events = vec![
            DialogDomainEvent::TurnAdded(TurnAdded {
                dialog_id: cmd.dialog_id,
                turn: cmd.turn,
                turn_number,
            })
        ];

        Ok(domain_events)
    }

    /// Handle SwitchContext command
    pub fn handle_switch_context(&self, cmd: SwitchContext) -> DomainResult<Vec<DialogDomainEvent>> {
        // Load dialog aggregate
        let entity_id = EntityId::<DialogMarker>::from_uuid(cmd.dialog_id);
        let mut dialog = self.repository.load(entity_id)
            .map_err(|e| DomainError::Generic(e))?
            .ok_or_else(|| DomainError::EntityNotFound { 
                entity_type: "Dialog".to_string(),
                id: cmd.dialog_id.to_string(),
            })?;

        // Get current topic before switching
        let previous_topic = dialog.current_topic().map(|t| t.id);
        
        // Switch topic (context)
        let _events = dialog.switch_topic(cmd.topic.clone())
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        // Save aggregate
        self.repository.save(&dialog)
            .map_err(|e| DomainError::Generic(e))?;
        
        // Create event manually
        let domain_events = vec![
            DialogDomainEvent::ContextSwitched(ContextSwitched {
                dialog_id: cmd.dialog_id,
                previous_topic,
                new_topic: cmd.topic,
                switched_at: Utc::now(),
            })
        ];

        Ok(domain_events)
    }

    /// Handle UpdateContext command
    pub fn handle_update_context(&self, cmd: UpdateContext) -> DomainResult<Vec<DialogDomainEvent>> {
        // Load dialog aggregate
        let entity_id = EntityId::<DialogMarker>::from_uuid(cmd.dialog_id);
        let mut dialog = self.repository.load(entity_id)
            .map_err(|e| DomainError::Generic(e))?
            .ok_or_else(|| DomainError::EntityNotFound { 
                entity_type: "Dialog".to_string(),
                id: cmd.dialog_id.to_string(),
            })?;

        // Update context variables
        let _events = dialog.update_context(cmd.variables.clone())
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        // Save aggregate
        self.repository.save(&dialog)
            .map_err(|e| DomainError::Generic(e))?;
        
        // Create event manually
        let domain_events = vec![
            DialogDomainEvent::ContextUpdated(ContextUpdated {
                dialog_id: cmd.dialog_id,
                updated_variables: cmd.variables,
                updated_at: Utc::now(),
            })
        ];

        Ok(domain_events)
    }

    /// Handle PauseDialog command
    pub fn handle_pause_dialog(&self, cmd: PauseDialog) -> DomainResult<Vec<DialogDomainEvent>> {
        // Load dialog aggregate
        let entity_id = EntityId::<DialogMarker>::from_uuid(cmd.id);
        let mut dialog = self.repository.load(entity_id)
            .map_err(|e| DomainError::Generic(e))?
            .ok_or_else(|| DomainError::EntityNotFound { 
                entity_type: "Dialog".to_string(),
                id: cmd.id.to_string(),
            })?;

        // Get current context snapshot
        let context_snapshot = dialog.context().variables.clone();
        
        // Pause the dialog
        let _events = dialog.pause()
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        // Save aggregate
        self.repository.save(&dialog)
            .map_err(|e| DomainError::Generic(e))?;
        
        // Create event manually
        let domain_events = vec![
            DialogDomainEvent::DialogPaused(DialogPaused {
                dialog_id: cmd.id,
                paused_at: Utc::now(),
                context_snapshot,
            })
        ];

        Ok(domain_events)
    }

    /// Handle ResumeDialog command
    pub fn handle_resume_dialog(&self, cmd: ResumeDialog) -> DomainResult<Vec<DialogDomainEvent>> {
        // Load dialog aggregate
        let entity_id = EntityId::<DialogMarker>::from_uuid(cmd.id);
        let mut dialog = self.repository.load(entity_id)
            .map_err(|e| DomainError::Generic(e))?
            .ok_or_else(|| DomainError::EntityNotFound { 
                entity_type: "Dialog".to_string(),
                id: cmd.id.to_string(),
            })?;

        // Resume the dialog
        let _events = dialog.resume()
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        // Save aggregate
        self.repository.save(&dialog)
            .map_err(|e| DomainError::Generic(e))?;
        
        // Create event manually
        let domain_events = vec![
            DialogDomainEvent::DialogResumed(DialogResumed {
                dialog_id: cmd.id,
                resumed_at: Utc::now(),
            })
        ];

        Ok(domain_events)
    }

    /// Handle SetDialogMetadata command
    pub fn handle_set_metadata(&self, cmd: SetDialogMetadata) -> DomainResult<Vec<DialogDomainEvent>> {
        // Load dialog aggregate
        let entity_id = EntityId::<DialogMarker>::from_uuid(cmd.dialog_id);
        let mut dialog = self.repository.load(entity_id)
            .map_err(|e| DomainError::Generic(e))?
            .ok_or_else(|| DomainError::EntityNotFound { 
                entity_type: "Dialog".to_string(),
                id: cmd.dialog_id.to_string(),
            })?;

        // Set metadata
        let _events = dialog.set_metadata(cmd.key.clone(), cmd.value.clone())
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        // Save aggregate
        self.repository.save(&dialog)
            .map_err(|e| DomainError::Generic(e))?;
        
        // Create event manually
        let domain_events = vec![
            DialogDomainEvent::DialogMetadataSet(DialogMetadataSet {
                dialog_id: cmd.dialog_id,
                key: cmd.key,
                value: cmd.value,
                set_at: Utc::now(),
            })
        ];

        Ok(domain_events)
    }

    /// Handle AddParticipant command
    pub fn handle_add_participant(&self, cmd: AddParticipant) -> DomainResult<Vec<DialogDomainEvent>> {
        // Load dialog aggregate
        let entity_id = EntityId::<DialogMarker>::from_uuid(cmd.dialog_id);
        let mut dialog = self.repository.load(entity_id)
            .map_err(|e| DomainError::Generic(e))?
            .ok_or_else(|| DomainError::EntityNotFound { 
                entity_type: "Dialog".to_string(),
                id: cmd.dialog_id.to_string(),
            })?;

        // Add participant
        let _events = dialog.add_participant(cmd.participant.clone())
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        // Save aggregate
        self.repository.save(&dialog)
            .map_err(|e| DomainError::Generic(e))?;
        
        // Create event manually
        let domain_events = vec![
            DialogDomainEvent::ParticipantAdded(ParticipantAdded {
                dialog_id: cmd.dialog_id,
                participant: cmd.participant,
                added_at: Utc::now(),
            })
        ];

        Ok(domain_events)
    }

    /// Handle RemoveParticipant command
    pub fn handle_remove_participant(&self, cmd: RemoveParticipant) -> DomainResult<Vec<DialogDomainEvent>> {
        // Load dialog aggregate
        let entity_id = EntityId::<DialogMarker>::from_uuid(cmd.dialog_id);
        let mut dialog = self.repository.load(entity_id)
            .map_err(|e| DomainError::Generic(e))?
            .ok_or_else(|| DomainError::EntityNotFound { 
                entity_type: "Dialog".to_string(),
                id: cmd.dialog_id.to_string(),
            })?;

        // Remove participant
        let _events = dialog.remove_participant(cmd.participant_id, cmd.reason.clone())
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        // Save aggregate
        self.repository.save(&dialog)
            .map_err(|e| DomainError::Generic(e))?;
        
        // Create event manually
        let domain_events = vec![
            DialogDomainEvent::ParticipantRemoved(ParticipantRemoved {
                dialog_id: cmd.dialog_id,
                participant_id: cmd.participant_id,
                removed_at: Utc::now(),
                reason: cmd.reason,
            })
        ];

        Ok(domain_events)
    }

    /// Handle MarkTopicComplete command
    pub fn handle_mark_topic_complete(&self, cmd: MarkTopicComplete) -> DomainResult<Vec<DialogDomainEvent>> {
        // Load dialog aggregate
        let entity_id = EntityId::<DialogMarker>::from_uuid(cmd.dialog_id);
        let mut dialog = self.repository.load(entity_id)
            .map_err(|e| DomainError::Generic(e))?
            .ok_or_else(|| DomainError::EntityNotFound { 
                entity_type: "Dialog".to_string(),
                id: cmd.dialog_id.to_string(),
            })?;

        // Mark topic complete
        let _events = dialog.mark_topic_complete(cmd.topic_id, cmd.resolution.clone())
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        // Save aggregate
        self.repository.save(&dialog)
            .map_err(|e| DomainError::Generic(e))?;
        
        // Create event manually
        let domain_events = vec![
            DialogDomainEvent::TopicCompleted(TopicCompleted {
                dialog_id: cmd.dialog_id,
                topic_id: cmd.topic_id,
                completed_at: Utc::now(),
                resolution: cmd.resolution,
            })
        ];

        Ok(domain_events)
    }

    /// Handle AddContextVariable command
    pub fn handle_add_context_variable(&self, cmd: AddContextVariable) -> DomainResult<Vec<DialogDomainEvent>> {
        // Load dialog aggregate
        let entity_id = EntityId::<DialogMarker>::from_uuid(cmd.dialog_id);
        let mut dialog = self.repository.load(entity_id)
            .map_err(|e| DomainError::Generic(e))?
            .ok_or_else(|| DomainError::EntityNotFound { 
                entity_type: "Dialog".to_string(),
                id: cmd.dialog_id.to_string(),
            })?;

        // Add context variable
        let _events = dialog.add_context_variable(cmd.variable.clone())
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        // Save aggregate
        self.repository.save(&dialog)
            .map_err(|e| DomainError::Generic(e))?;
        
        // Create event manually
        let domain_events = vec![
            DialogDomainEvent::ContextVariableAdded(ContextVariableAdded {
                dialog_id: cmd.dialog_id,
                variable: cmd.variable,
                added_at: Utc::now(),
            })
        ];

        Ok(domain_events)
    }
}