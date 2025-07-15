//! Tests for dialog command and event handlers

use cim_domain::{AggregateRepository, EntityId, InMemoryRepository};
use cim_domain_dialog::{
    aggregate::{Dialog, DialogType, DialogMarker},
    commands::*,
    handlers::DialogCommandHandler,
    value_objects::{Participant, ParticipantType, ParticipantRole, Turn, TurnType, TurnMetadata, Message, MessageContent, Topic, TopicStatus, TopicRelevance},
};
use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;

#[test]
fn test_handle_start_dialog() {
    // Setup
    let repository = Arc::new(InMemoryRepository::<Dialog>::new());
    let handler = DialogCommandHandler::new(repository.clone());

    // Create command
    let dialog_id = Uuid::new_v4();
    let participant = Participant {
        id: Uuid::new_v4(),
        participant_type: ParticipantType::Human,
        role: ParticipantRole::Primary,
        name: "Test User".to_string(),
        metadata: HashMap::new(),
    };
    
    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), serde_json::Value::String("test".to_string()));

    let cmd = StartDialog {
        id: dialog_id,
        dialog_type: DialogType::Direct,
        primary_participant: participant.clone(),
        metadata: Some(metadata),
    };

    // Execute
    let result = handler.handle_start_dialog(cmd);

    // Verify
    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 2); // DialogStarted + DialogMetadataSet

    // Check repository
    let entity_id = EntityId::<DialogMarker>::from_uuid(dialog_id);
    let stored = repository.load(entity_id).unwrap();
    assert!(stored.is_some());
}

#[test]
fn test_handle_add_turn() {
    // Setup
    let repository = Arc::new(InMemoryRepository::<Dialog>::new());
    let handler = DialogCommandHandler::new(repository.clone());

    // First create a dialog
    let dialog_id = Uuid::new_v4();
    let participant = Participant {
        id: Uuid::new_v4(),
        participant_type: ParticipantType::Human,
        role: ParticipantRole::Primary,
        name: "Test User".to_string(),
        metadata: HashMap::new(),
    };

    let start_cmd = StartDialog {
        id: dialog_id,
        dialog_type: DialogType::Direct,
        primary_participant: participant.clone(),
        metadata: None,
    };

    handler.handle_start_dialog(start_cmd).unwrap();

    // Now add a turn
    let message = Message {
        content: MessageContent::Text("Hello, world!".to_string()),
        intent: None,
        language: "en".to_string(),
        sentiment: None,
        embeddings: None,
    };

    let turn = Turn {
        turn_id: Uuid::new_v4(),
        turn_number: 1,
        participant_id: participant.id,
        message,
        timestamp: chrono::Utc::now(),
        metadata: TurnMetadata {
            turn_type: TurnType::UserQuery,
            confidence: None,
            processing_time_ms: None,
            references: Vec::new(),
            properties: HashMap::new(),
        },
    };

    let add_turn_cmd = AddTurn {
        dialog_id,
        turn: turn.clone(),
    };

    // Execute
    let result = handler.handle_add_turn(add_turn_cmd);

    // Verify
    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 1); // TurnAdded event

    // Check that turn was added to dialog
    let entity_id = EntityId::<DialogMarker>::from_uuid(dialog_id);
    let stored = repository.load(entity_id).unwrap();
    let dialog = stored.unwrap();
    assert_eq!(dialog.turn_count(), 1);
}

#[test]
fn test_handle_switch_context() {
    // Setup
    let repository = Arc::new(InMemoryRepository::<Dialog>::new());
    let handler = DialogCommandHandler::new(repository.clone());

    // Create dialog
    let dialog_id = Uuid::new_v4();
    let participant = Participant {
        id: Uuid::new_v4(),
        participant_type: ParticipantType::Human,
        role: ParticipantRole::Primary,
        name: "Test User".to_string(),
        metadata: HashMap::new(),
    };

    let start_cmd = StartDialog {
        id: dialog_id,
        dialog_type: DialogType::Direct,
        primary_participant: participant,
        metadata: None,
    };

    handler.handle_start_dialog(start_cmd).unwrap();

    // Switch context
    let topic = Topic {
        id: Uuid::new_v4(),
        name: "New Topic".to_string(),
        status: TopicStatus::Active,
        relevance: TopicRelevance {
            score: 0.8,
            last_updated: chrono::Utc::now(),
            decay_rate: 0.1,
        },
        introduced_at: chrono::Utc::now(),
        related_topics: Vec::new(),
        keywords: vec!["topic".to_string(), "new".to_string()],
        embedding: None,
    };

    let switch_cmd = SwitchContext {
        dialog_id,
        topic: topic.clone(),
    };

    // Execute
    let result = handler.handle_switch_context(switch_cmd);

    // Verify
    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 1); // ContextSwitched event
}

#[test]
fn test_handle_pause_resume_dialog() {
    // Setup
    let repository = Arc::new(InMemoryRepository::<Dialog>::new());
    let handler = DialogCommandHandler::new(repository.clone());

    // Create dialog
    let dialog_id = Uuid::new_v4();
    let participant = Participant {
        id: Uuid::new_v4(),
        participant_type: ParticipantType::Human,
        role: ParticipantRole::Primary,
        name: "Test User".to_string(),
        metadata: HashMap::new(),
    };

    let start_cmd = StartDialog {
        id: dialog_id,
        dialog_type: DialogType::Direct,
        primary_participant: participant,
        metadata: None,
    };

    handler.handle_start_dialog(start_cmd).unwrap();

    // Pause dialog
    let pause_cmd = PauseDialog { id: dialog_id };
    let result = handler.handle_pause_dialog(pause_cmd);
    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 1); // DialogPaused

    // Resume dialog
    let resume_cmd = ResumeDialog { id: dialog_id };
    let result = handler.handle_resume_dialog(resume_cmd);
    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 1); // DialogResumed
}

#[test]
fn test_handle_add_remove_participant() {
    // Setup
    let repository = Arc::new(InMemoryRepository::<Dialog>::new());
    let handler = DialogCommandHandler::new(repository.clone());

    // Create dialog
    let dialog_id = Uuid::new_v4();
    let primary_participant = Participant {
        id: Uuid::new_v4(),
        participant_type: ParticipantType::Human,
        role: ParticipantRole::Primary,
        name: "Primary User".to_string(),
        metadata: HashMap::new(),
    };

    let start_cmd = StartDialog {
        id: dialog_id,
        dialog_type: DialogType::Direct,
        primary_participant,
        metadata: None,
    };

    handler.handle_start_dialog(start_cmd).unwrap();

    // Add participant
    let new_participant = Participant {
        id: Uuid::new_v4(),
        participant_type: ParticipantType::AIAgent,
        role: ParticipantRole::Observer,
        name: "AI Assistant".to_string(),
        metadata: HashMap::new(),
    };

    let add_cmd = AddParticipant {
        dialog_id,
        participant: new_participant.clone(),
    };

    let result = handler.handle_add_participant(add_cmd);
    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 1); // ParticipantAdded

    // Remove participant
    let remove_cmd = RemoveParticipant {
        dialog_id,
        participant_id: new_participant.id,
        reason: Some("Test removal".to_string()),
    };

    let result = handler.handle_remove_participant(remove_cmd);
    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 1); // ParticipantRemoved
}

#[test]
fn test_handle_end_dialog() {
    // Setup
    let repository = Arc::new(InMemoryRepository::<Dialog>::new());
    let handler = DialogCommandHandler::new(repository.clone());

    // Create dialog
    let dialog_id = Uuid::new_v4();
    let participant = Participant {
        id: Uuid::new_v4(),
        participant_type: ParticipantType::Human,
        role: ParticipantRole::Primary,
        name: "Test User".to_string(),
        metadata: HashMap::new(),
    };

    let start_cmd = StartDialog {
        id: dialog_id,
        dialog_type: DialogType::Direct,
        primary_participant: participant,
        metadata: None,
    };

    handler.handle_start_dialog(start_cmd).unwrap();

    // End dialog
    let end_cmd = EndDialog {
        id: dialog_id,
        reason: Some("Test completion".to_string()),
    };

    // Execute
    let result = handler.handle_end_dialog(end_cmd);

    // Verify
    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 1); // DialogEnded event

    // Check dialog status
    let entity_id = EntityId::<DialogMarker>::from_uuid(dialog_id);
    let stored = repository.load(entity_id).unwrap();
    let dialog = stored.unwrap();
    assert!(dialog.is_ended());
}

#[test]
fn test_error_handling_dialog_not_found() {
    // Setup
    let repository = Arc::new(InMemoryRepository::<Dialog>::new());
    let handler = DialogCommandHandler::new(repository);

    // Try to end non-existent dialog
    let end_cmd = EndDialog {
        id: Uuid::new_v4(),
        reason: None,
    };

    // Execute
    let result = handler.handle_end_dialog(end_cmd);

    // Verify error
    assert!(result.is_err());
    match result.unwrap_err() {
        cim_domain::DomainError::EntityNotFound { entity_type, .. } => {
            assert_eq!(entity_type, "Dialog");
        }
        _ => panic!("Expected EntityNotFound error"),
    }
}