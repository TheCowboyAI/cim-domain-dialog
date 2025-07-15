//! Integration tests for Dialog domain
//!
//! These tests verify the complete flow of the Dialog domain including:
//! - Event generation and handling
//! - Projection updates
//! - Query execution
//! - State transitions

use cim_domain_dialog::{
    aggregate::{DialogStatus, DialogType},
    events::{DialogDomainEvent, DialogStarted, TurnAdded, DialogEnded, DialogPaused, DialogResumed},
    projections::SimpleProjectionUpdater,
    queries::{DialogQuery, DialogQueryHandler, DialogQueryResult},
    value_objects::{
        ConversationMetrics, Message, MessageContent, MessageIntent, Participant, 
        ParticipantRole, ParticipantType, Turn, TurnMetadata, TurnType,
    },
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Test the complete dialog lifecycle using events
#[tokio::test]
async fn test_dialog_lifecycle_with_events() {
    let updater = SimpleProjectionUpdater::new();
    let dialog_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    
    // Start dialog
    let start_event = DialogDomainEvent::DialogStarted(DialogStarted {
        dialog_id,
        dialog_type: DialogType::Support,
        primary_participant: Participant {
            id: user_id,
            participant_type: ParticipantType::Human,
            role: ParticipantRole::Primary,
            name: "Alice".to_string(),
            metadata: HashMap::new(),
        },
        started_at: Utc::now(),
    });
    
    updater.handle_event(start_event).await.unwrap();
    
    // Verify dialog was created
    let view = updater.get_view(&dialog_id);
    assert!(view.is_some());
    assert_eq!(view.unwrap().status, DialogStatus::Active);
    
    // Add a turn
    let turn_event = DialogDomainEvent::TurnAdded(TurnAdded {
        dialog_id,
        turn: Turn {
            turn_id: Uuid::new_v4(),
            turn_number: 1,
            participant_id: user_id,
            message: Message {
                content: MessageContent::Text("I need help with my account".to_string()),
                intent: Some(MessageIntent::Question),
                language: "en".to_string(),
                sentiment: Some(0.3),
                embeddings: None,
            },
            timestamp: Utc::now(),
            metadata: TurnMetadata {
                turn_type: TurnType::UserQuery,
                confidence: None,
                processing_time_ms: None,
                references: vec![],
                properties: HashMap::new(),
            },
        },
        turn_number: 1,
    });
    
    updater.handle_event(turn_event).await.unwrap();
    
    // Verify turn was added
    let view = updater.get_view(&dialog_id).unwrap();
    assert_eq!(view.turns.len(), 1);
    
    // End dialog
    let end_event = DialogDomainEvent::DialogEnded(DialogEnded {
        dialog_id,
        ended_at: Utc::now(),
        reason: Some("Issue resolved".to_string()),
        final_metrics: ConversationMetrics {
            turn_count: 1,
            avg_response_time_ms: 1000.0,
            topic_switches: 0,
            clarification_count: 0,
            sentiment_trend: 0.8,
            coherence_score: 0.9,
        },
    });
    
    updater.handle_event(end_event).await.unwrap();
    
    // Verify dialog ended
    let view = updater.get_view(&dialog_id).unwrap();
    assert_eq!(view.status, DialogStatus::Ended);
    assert!(view.ended_at.is_some());
}

/// Test projection updates from multiple events
#[tokio::test]
async fn test_projection_updates() {
    let updater = SimpleProjectionUpdater::new();
    
    // Create multiple dialogs
    let dialog_ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
    
    for (i, &dialog_id) in dialog_ids.iter().enumerate() {
        let event = DialogDomainEvent::DialogStarted(DialogStarted {
            dialog_id,
            dialog_type: if i % 2 == 0 { DialogType::Support } else { DialogType::Direct },
            primary_participant: Participant {
                id: Uuid::new_v4(),
                participant_type: ParticipantType::Human,
                role: ParticipantRole::Primary,
                name: format!("User{}", i),
                metadata: HashMap::new(),
            },
            started_at: Utc::now() - chrono::Duration::hours(i as i64),
        });
        
        updater.handle_event(event).await.unwrap();
    }
    
    // Verify all dialogs were created
    let all_dialogs = updater.get_all_dialogs();
    assert_eq!(all_dialogs.len(), 3);
    
    // End one dialog
    updater.handle_event(DialogDomainEvent::DialogEnded(DialogEnded {
        dialog_id: dialog_ids[0],
        ended_at: Utc::now(),
        reason: None,
        final_metrics: ConversationMetrics {
            turn_count: 2,
            avg_response_time_ms: 1500.0,
            topic_switches: 1,
            clarification_count: 0,
            sentiment_trend: 0.6,
            coherence_score: 0.75,
        },
    })).await.unwrap();
    
    // Check active dialogs
    let active_dialogs = updater.get_active_dialogs();
    assert_eq!(active_dialogs.len(), 2);
}

/// Test query functionality with complex scenarios
#[tokio::test]
async fn test_complex_queries() {
    let updater = SimpleProjectionUpdater::new();
    
    // Create dialogs with different characteristics
    let support_dialog_id = Uuid::new_v4();
    let direct_dialog_id = Uuid::new_v4();
    let group_dialog_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    
    // Support dialog with billing question
    updater.handle_event(DialogDomainEvent::DialogStarted(DialogStarted {
        dialog_id: support_dialog_id,
        dialog_type: DialogType::Support,
        primary_participant: Participant {
            id: user_id,
            participant_type: ParticipantType::Human,
            role: ParticipantRole::Primary,
            name: "BillingUser".to_string(),
            metadata: HashMap::new(),
        },
        started_at: Utc::now() - chrono::Duration::hours(2),
    })).await.unwrap();
    
    updater.handle_event(DialogDomainEvent::TurnAdded(TurnAdded {
        dialog_id: support_dialog_id,
        turn: Turn {
            turn_id: Uuid::new_v4(),
            turn_number: 1,
            participant_id: user_id,
            message: Message {
                content: MessageContent::Text("I have a billing question about my subscription".to_string()),
                intent: Some(MessageIntent::Question),
                language: "en".to_string(),
                sentiment: Some(0.2),
                embeddings: None,
            },
            timestamp: Utc::now() - chrono::Duration::hours(2),
            metadata: TurnMetadata {
                turn_type: TurnType::UserQuery,
                confidence: None,
                processing_time_ms: None,
                references: vec![],
                properties: HashMap::new(),
            },
        },
        turn_number: 1,
    })).await.unwrap();
    
    // Direct dialog
    updater.handle_event(DialogDomainEvent::DialogStarted(DialogStarted {
        dialog_id: direct_dialog_id,
        dialog_type: DialogType::Direct,
        primary_participant: Participant {
            id: Uuid::new_v4(),
            participant_type: ParticipantType::Human,
            role: ParticipantRole::Primary,
            name: "DirectUser".to_string(),
            metadata: HashMap::new(),
        },
        started_at: Utc::now() - chrono::Duration::minutes(30),
    })).await.unwrap();
    
    // Group dialog (ended)
    updater.handle_event(DialogDomainEvent::DialogStarted(DialogStarted {
        dialog_id: group_dialog_id,
        dialog_type: DialogType::Group,
        primary_participant: Participant {
            id: Uuid::new_v4(),
            participant_type: ParticipantType::Human,
            role: ParticipantRole::Primary,
            name: "GroupLead".to_string(),
            metadata: HashMap::new(),
        },
        started_at: Utc::now() - chrono::Duration::days(1),
    })).await.unwrap();
    
    updater.handle_event(DialogDomainEvent::DialogEnded(DialogEnded {
        dialog_id: group_dialog_id,
        ended_at: Utc::now() - chrono::Duration::hours(12),
        reason: Some("Meeting concluded".to_string()),
        final_metrics: ConversationMetrics {
            turn_count: 15,
            avg_response_time_ms: 2000.0,
            topic_switches: 5,
            clarification_count: 2,
            sentiment_trend: 0.7,
            coherence_score: 0.8,
        },
    })).await.unwrap();
    
    // Create query handler
    let updater_arc = Arc::new(RwLock::new(updater));
    let query_handler = DialogQueryHandler::new(updater_arc);
    
    // Test 1: Get by ID
    let result = query_handler.execute(DialogQuery::GetDialogById { 
        dialog_id: support_dialog_id 
    }).await;
    
    match result {
        DialogQueryResult::Dialog(Some(dialog)) => {
            assert_eq!(dialog.dialog_id, support_dialog_id);
            assert_eq!(dialog.dialog_type, DialogType::Support);
        }
        _ => panic!("Expected dialog result"),
    }
    
    // Test 2: Get by type
    let result = query_handler.execute(DialogQuery::GetDialogsByType { 
        dialog_type: DialogType::Support 
    }).await;
    
    match result {
        DialogQueryResult::Dialogs(dialogs) => {
            assert_eq!(dialogs.len(), 1);
            assert_eq!(dialogs[0].dialog_type, DialogType::Support);
        }
        _ => panic!("Expected dialogs result"),
    }
    
    // Test 3: Search by text
    let result = query_handler.execute(DialogQuery::SearchDialogsByText { 
        search_text: "billing".to_string() 
    }).await;
    
    match result {
        DialogQueryResult::Dialogs(dialogs) => {
            assert_eq!(dialogs.len(), 1);
            assert_eq!(dialogs[0].dialog_id, support_dialog_id);
        }
        _ => panic!("Expected dialogs result"),
    }
    
    // Test 4: Get active dialogs
    let result = query_handler.execute(DialogQuery::GetActiveDialogs).await;
    
    match result {
        DialogQueryResult::Dialogs(dialogs) => {
            assert_eq!(dialogs.len(), 2); // Support and Direct are active
        }
        _ => panic!("Expected dialogs result"),
    }
    
    // Test 5: Get by status
    let result = query_handler.execute(DialogQuery::GetDialogsByStatus { 
        status: DialogStatus::Ended 
    }).await;
    
    match result {
        DialogQueryResult::Dialogs(dialogs) => {
            assert_eq!(dialogs.len(), 1);
            assert_eq!(dialogs[0].dialog_id, group_dialog_id);
        }
        _ => panic!("Expected dialogs result"),
    }
    
    // Test 6: Date range query
    let start_date = Utc::now() - chrono::Duration::hours(3);
    let end_date = Utc::now();
    let result = query_handler.execute(DialogQuery::GetDialogsInDateRange { 
        start_date, 
        end_date 
    }).await;
    
    match result {
        DialogQueryResult::Dialogs(dialogs) => {
            assert_eq!(dialogs.len(), 2); // Support and Direct started in last 3 hours
        }
        _ => panic!("Expected dialogs result"),
    }
    
    // Test 7: Get statistics
    let result = query_handler.execute(DialogQuery::GetDialogStatistics).await;
    
    match result {
        DialogQueryResult::Statistics(stats) => {
            assert_eq!(stats.total_dialogs, 3);
            assert_eq!(stats.active_dialogs, 2);
            assert_eq!(stats.completed_dialogs, 1);
            assert_eq!(stats.total_participants, 3);
        }
        _ => panic!("Expected statistics result"),
    }
}

/// Test dialog state transitions
#[tokio::test]
async fn test_dialog_state_transitions() {
    let updater = SimpleProjectionUpdater::new();
    let dialog_id = Uuid::new_v4();
    
    // Start dialog
    updater.handle_event(DialogDomainEvent::DialogStarted(DialogStarted {
        dialog_id,
        dialog_type: DialogType::Task,
        primary_participant: Participant {
            id: Uuid::new_v4(),
            participant_type: ParticipantType::Human,
            role: ParticipantRole::Primary,
            name: "TaskUser".to_string(),
            metadata: HashMap::new(),
        },
        started_at: Utc::now(),
    })).await.unwrap();
    
    // Check initial state
    let view = updater.get_view(&dialog_id).unwrap();
    assert_eq!(view.status, DialogStatus::Active);
    
    // Pause dialog
    updater.handle_event(DialogDomainEvent::DialogPaused(DialogPaused {
        dialog_id,
        paused_at: Utc::now(),
        context_snapshot: HashMap::new(),
    })).await.unwrap();
    
    // Check paused state
    let view = updater.get_view(&dialog_id).unwrap();
    assert_eq!(view.status, DialogStatus::Paused);
    
    // Resume dialog
    updater.handle_event(DialogDomainEvent::DialogResumed(DialogResumed {
        dialog_id,
        resumed_at: Utc::now(),
    })).await.unwrap();
    
    // Check resumed state
    let view = updater.get_view(&dialog_id).unwrap();
    assert_eq!(view.status, DialogStatus::Active);
    
    // End dialog
    updater.handle_event(DialogDomainEvent::DialogEnded(DialogEnded {
        dialog_id,
        ended_at: Utc::now(),
        reason: Some("Task completed".to_string()),
        final_metrics: ConversationMetrics {
            turn_count: 3,
            avg_response_time_ms: 1500.0,
            topic_switches: 1,
            clarification_count: 0,
            sentiment_trend: 0.7,
            coherence_score: 0.85,
        },
    })).await.unwrap();
    
    // Check ended state
    let view = updater.get_view(&dialog_id).unwrap();
    assert_eq!(view.status, DialogStatus::Ended);
}

/// Test concurrent operations
#[tokio::test]
async fn test_concurrent_operations() {
    let updater = Arc::new(RwLock::new(SimpleProjectionUpdater::new()));
    let query_handler = DialogQueryHandler::new(updater.clone());
    
    // Create multiple dialogs concurrently
    let mut handles = vec![];
    
    for i in 0..5 {
        let updater_clone = updater.clone();
        let handle = tokio::spawn(async move {
            let dialog_id = Uuid::new_v4();
            let event = DialogDomainEvent::DialogStarted(DialogStarted {
                dialog_id,
                dialog_type: if i % 2 == 0 { DialogType::Support } else { DialogType::Direct },
                primary_participant: Participant {
                    id: Uuid::new_v4(),
                    participant_type: ParticipantType::Human,
                    role: ParticipantRole::Primary,
                    name: format!("User{}", i),
                    metadata: HashMap::new(),
                },
                started_at: Utc::now(),
            });
            
            let mut updater = updater_clone.write().await;
            updater.handle_event(event).await.unwrap();
        });
        handles.push(handle);
    }
    
    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Query and verify
    let result = query_handler.execute(DialogQuery::GetDialogStatistics).await;
    match result {
        DialogQueryResult::Statistics(stats) => {
            assert_eq!(stats.total_dialogs, 5);
            assert_eq!(stats.active_dialogs, 5);
        }
        _ => panic!("Expected statistics"),
    }
}

/// Test error scenarios and edge cases
#[tokio::test]
async fn test_edge_cases() {
    let updater = SimpleProjectionUpdater::new();
    let updater_arc = Arc::new(RwLock::new(updater));
    let query_handler = DialogQueryHandler::new(updater_arc);
    
    // Test 1: Query non-existent dialog
    let result = query_handler.execute(DialogQuery::GetDialogById { 
        dialog_id: Uuid::new_v4() 
    }).await;
    
    match result {
        DialogQueryResult::Dialog(None) => {
            // Expected - dialog doesn't exist
        }
        _ => panic!("Expected None for non-existent dialog"),
    }
    
    // Test 2: Search with no results
    let result = query_handler.execute(DialogQuery::SearchDialogsByText { 
        search_text: "nonexistent".to_string() 
    }).await;
    
    match result {
        DialogQueryResult::Dialogs(dialogs) => {
            assert_eq!(dialogs.len(), 0);
        }
        _ => panic!("Expected empty dialogs result"),
    }
    
    // Test 3: Statistics with no dialogs
    let result = query_handler.execute(DialogQuery::GetDialogStatistics).await;
    
    match result {
        DialogQueryResult::Statistics(stats) => {
            assert_eq!(stats.total_dialogs, 0);
            assert_eq!(stats.average_turn_count, 0.0);
        }
        _ => panic!("Expected statistics result"),
    }
}