//! Dialog domain queries demonstration
//!
//! This example shows how to use the Dialog domain query system
//! to search and retrieve dialog data.

use cim_domain_dialog::{
    aggregate::{DialogStatus, DialogType},
    events::{DialogDomainEvent, DialogStarted, TurnAdded, DialogEnded},
    projections::SimpleProjectionUpdater,
    queries::{DialogQuery, DialogQueryHandler, DialogQueryResult},
    value_objects::{
        Participant, ParticipantType, ParticipantRole, Turn, Message, 
        MessageContent, MessageIntent, ConversationMetrics, TurnMetadata, TurnType
    },
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Dialog Domain Query Demonstration ===\n");

    // Create projection updater
    let mut updater = SimpleProjectionUpdater::new();
    
    // Create some test dialogs
    println!("Creating test dialogs...");
    
    // Dialog 1: Support conversation
    let dialog1_id = Uuid::new_v4();
    let user1_id = Uuid::new_v4();
    
    updater.handle_event(DialogDomainEvent::DialogStarted(DialogStarted {
        dialog_id: dialog1_id,
        dialog_type: DialogType::Support,
        primary_participant: Participant {
            id: user1_id,
            participant_type: ParticipantType::Human,
            role: ParticipantRole::Primary,
            name: "Alice".to_string(),
            metadata: HashMap::new(),
        },
        started_at: Utc::now() - chrono::Duration::hours(2),
    })).await?;
    
    // Add some turns
    updater.handle_event(DialogDomainEvent::TurnAdded(TurnAdded {
        dialog_id: dialog1_id,
        turn: Turn {
            turn_id: Uuid::new_v4(),
            turn_number: 1,
            participant_id: user1_id,
            message: Message {
                content: MessageContent::Text("I need help with my order".to_string()),
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
    })).await?;
    
    // Dialog 2: Group conversation
    let dialog2_id = Uuid::new_v4();
    let user2_id = Uuid::new_v4();
    
    updater.handle_event(DialogDomainEvent::DialogStarted(DialogStarted {
        dialog_id: dialog2_id,
        dialog_type: DialogType::Group,
        primary_participant: Participant {
            id: user2_id,
            participant_type: ParticipantType::Human,
            role: ParticipantRole::Primary,
            name: "Bob".to_string(),
            metadata: HashMap::new(),
        },
        started_at: Utc::now() - chrono::Duration::hours(1),
    })).await?;
    
    // Dialog 3: Completed support dialog
    let dialog3_id = Uuid::new_v4();
    let user3_id = Uuid::new_v4();
    
    updater.handle_event(DialogDomainEvent::DialogStarted(DialogStarted {
        dialog_id: dialog3_id,
        dialog_type: DialogType::Support,
        primary_participant: Participant {
            id: user3_id,
            participant_type: ParticipantType::Human,
            role: ParticipantRole::Primary,
            name: "Charlie".to_string(),
            metadata: HashMap::new(),
        },
        started_at: Utc::now() - chrono::Duration::days(1),
    })).await?;
    
    // End dialog 3
    updater.handle_event(DialogDomainEvent::DialogEnded(DialogEnded {
        dialog_id: dialog3_id,
        ended_at: Utc::now() - chrono::Duration::hours(20),
        reason: Some("Issue resolved".to_string()),
        final_metrics: ConversationMetrics {
            turn_count: 5,
            avg_response_time_ms: 2000.0,
            topic_switches: 2,
            clarification_count: 1,
            sentiment_trend: 0.8,
            coherence_score: 0.9,
        },
    })).await?;
    
    println!("Created 3 test dialogs\n");
    
    // Create query handler
    let updater_arc = Arc::new(RwLock::new(updater));
    let handler = DialogQueryHandler::new(updater_arc);
    
    // Demonstrate various queries
    println!("=== Query Demonstrations ===\n");
    
    // 1. Get dialog by ID
    println!("1. Get specific dialog by ID:");
    let result = handler.execute(DialogQuery::GetDialogById { dialog_id: dialog1_id }).await;
    match result {
        DialogQueryResult::Dialog(Some(dialog)) => {
            println!("   Found dialog: {} (Type: {:?}, Status: {:?})", 
                dialog.dialog_id, dialog.dialog_type, dialog.status);
        }
        _ => println!("   Dialog not found"),
    }
    
    // 2. Get all active dialogs
    println!("\n2. Get all active dialogs:");
    let result = handler.execute(DialogQuery::GetActiveDialogs).await;
    match result {
        DialogQueryResult::Dialogs(dialogs) => {
            println!("   Found {} active dialogs", dialogs.len());
            for dialog in dialogs {
                println!("   - {} ({:?})", dialog.primary_participant.name, dialog.dialog_type);
            }
        }
        _ => println!("   No active dialogs found"),
    }
    
    // 3. Get dialogs by type
    println!("\n3. Get Support dialogs:");
    let result = handler.execute(DialogQuery::GetDialogsByType { 
        dialog_type: DialogType::Support 
    }).await;
    match result {
        DialogQueryResult::Dialogs(dialogs) => {
            println!("   Found {} support dialogs", dialogs.len());
            for dialog in dialogs {
                println!("   - {} (Status: {:?})", 
                    dialog.primary_participant.name, dialog.status);
            }
        }
        _ => println!("   No support dialogs found"),
    }
    
    // 4. Get dialogs by status
    println!("\n4. Get completed dialogs:");
    let result = handler.execute(DialogQuery::GetDialogsByStatus { 
        status: DialogStatus::Ended 
    }).await;
    match result {
        DialogQueryResult::Dialogs(dialogs) => {
            println!("   Found {} completed dialogs", dialogs.len());
            for dialog in dialogs {
                println!("   - {} (Ended at: {:?})", 
                    dialog.primary_participant.name, 
                    dialog.ended_at.map(|t| t.format("%Y-%m-%d %H:%M").to_string()));
            }
        }
        _ => println!("   No completed dialogs found"),
    }
    
    // 5. Search by text
    println!("\n5. Search for 'order' in messages:");
    let result = handler.execute(DialogQuery::SearchDialogsByText { 
        search_text: "order".to_string() 
    }).await;
    match result {
        DialogQueryResult::Dialogs(dialogs) => {
            println!("   Found {} dialogs containing 'order'", dialogs.len());
            for dialog in dialogs {
                println!("   - {} ({:?})", dialog.primary_participant.name, dialog.dialog_type);
            }
        }
        _ => println!("   No dialogs found"),
    }
    
    // 6. Get statistics
    println!("\n6. Get dialog statistics:");
    let result = handler.execute(DialogQuery::GetDialogStatistics).await;
    match result {
        DialogQueryResult::Statistics(stats) => {
            println!("   Total dialogs: {}", stats.total_dialogs);
            println!("   Active: {}", stats.active_dialogs);
            println!("   Completed: {}", stats.completed_dialogs);
            println!("   Paused: {}", stats.paused_dialogs);
            println!("   Average turn count: {:.2}", stats.average_turn_count);
            println!("   Total participants: {}", stats.total_participants);
            println!("   By type:");
            for (dialog_type, count) in stats.dialogs_by_type {
                println!("     - {:?}: {}", dialog_type, count);
            }
        }
        _ => println!("   Error getting statistics"),
    }
    
    // 7. Date range query
    println!("\n7. Get dialogs from last 2 hours:");
    let start_date = Utc::now() - chrono::Duration::hours(2);
    let end_date = Utc::now();
    let result = handler.execute(DialogQuery::GetDialogsInDateRange { 
        start_date, 
        end_date 
    }).await;
    match result {
        DialogQueryResult::Dialogs(dialogs) => {
            println!("   Found {} dialogs in date range", dialogs.len());
            for dialog in dialogs {
                println!("   - {} (Started: {})", 
                    dialog.primary_participant.name,
                    dialog.started_at.format("%H:%M").to_string());
            }
        }
        _ => println!("   No dialogs found in range"),
    }
    
    println!("\n=== Query demonstration complete ===");
    
    Ok(())
}