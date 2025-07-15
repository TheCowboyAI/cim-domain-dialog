//! Example of basic dialog flow using Dialog domain
//!
//! This demonstrates:
//! - Starting a dialog
//! - Adding turns to the dialog  
//! - Using different message types
//! - Ending the dialog

use cim_domain_dialog::{
    aggregate::DialogType,
    events::{DialogDomainEvent, DialogStarted, TurnAdded, DialogEnded},
    projections::SimpleProjectionUpdater,
    queries::{DialogQuery, DialogQueryHandler, DialogQueryResult},
    value_objects::{
        Message, MessageContent, MessageIntent, Participant, ParticipantRole, 
        ParticipantType, Turn, TurnMetadata, TurnType, ConversationMetrics,
    },
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Dialog Domain Example ===\n");

    // Initialize projection updater (simple event handler)
    let mut updater = SimpleProjectionUpdater::new();
    let dialog_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let agent_id = Uuid::new_v4();

    // Step 1: Start a dialog
    println!("1. Starting dialog...");
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

    updater.handle_event(start_event).await?;
    println!("   ✓ Dialog started with ID: {}", dialog_id);

    // Step 2: User sends first message
    println!("\n2. User sending first message...");
    let turn1 = DialogDomainEvent::TurnAdded(TurnAdded {
        dialog_id,
        turn: Turn {
            turn_id: Uuid::new_v4(),
            turn_number: 1,
            participant_id: user_id,
            message: Message {
                content: MessageContent::Text("Hello! I need help with my account.".to_string()),
                intent: Some(MessageIntent::Question),
                language: "en".to_string(),
                sentiment: Some(0.2),
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

    updater.handle_event(turn1).await?;
    println!("   ✓ User: \"Hello! I need help with my account.\"");

    // Step 3: Agent responds
    println!("\n3. Agent responding...");
    let turn2 = DialogDomainEvent::TurnAdded(TurnAdded {
        dialog_id,
        turn: Turn {
            turn_id: Uuid::new_v4(),
            turn_number: 2,
            participant_id: agent_id,
            message: Message {
                content: MessageContent::Text(
                    "I'd be happy to help you with your account! Could you please specify what kind of assistance you need?".to_string()
                ),
                intent: Some(MessageIntent::Clarification),
                language: "en".to_string(),
                sentiment: Some(0.8),
                embeddings: None,
            },
            timestamp: Utc::now(),
            metadata: TurnMetadata {
                turn_type: TurnType::AgentResponse,
                confidence: Some(0.95),
                processing_time_ms: Some(250),
                references: vec![],
                properties: HashMap::new(),
            },
        },
        turn_number: 2,
    });

    updater.handle_event(turn2).await?;
    println!("   ✓ Agent: \"I'd be happy to help you with your account!\"");

    // Step 4: User provides more details
    println!("\n4. User providing more details...");
    let turn3 = DialogDomainEvent::TurnAdded(TurnAdded {
        dialog_id,
        turn: Turn {
            turn_id: Uuid::new_v4(),
            turn_number: 3,
            participant_id: user_id,
            message: Message {
                content: MessageContent::Text(
                    "I forgot my password and can't log in.".to_string()
                ),
                intent: Some(MessageIntent::Statement),
                language: "en".to_string(),
                sentiment: Some(-0.3),
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
        turn_number: 3,
    });

    updater.handle_event(turn3).await?;
    println!("   ✓ User: \"I forgot my password and can't log in.\"");

    // Step 5: Agent provides structured response
    println!("\n5. Agent providing structured solution...");
    let solution_data = serde_json::json!({
        "steps": [
            {
                "step": 1,
                "action": "Click 'Forgot Password' on login page",
                "details": "Look for the link below the password field"
            },
            {
                "step": 2,
                "action": "Enter your email address",
                "details": "Use the email associated with your account"
            },
            {
                "step": 3,
                "action": "Check your email",
                "details": "You'll receive a password reset link within 5 minutes"
            }
        ],
        "alternative": "If you don't receive the email, check your spam folder or contact support"
    });

    let turn4 = DialogDomainEvent::TurnAdded(TurnAdded {
        dialog_id,
        turn: Turn {
            turn_id: Uuid::new_v4(),
            turn_number: 4,
            participant_id: agent_id,
            message: Message {
                content: MessageContent::Structured(solution_data),
                intent: Some(MessageIntent::Answer),
                language: "en".to_string(),
                sentiment: Some(0.7),
                embeddings: None,
            },
            timestamp: Utc::now(),
            metadata: TurnMetadata {
                turn_type: TurnType::AgentResponse,
                confidence: Some(0.98),
                processing_time_ms: Some(180),
                references: vec![],
                properties: HashMap::new(),
            },
        },
        turn_number: 4,
    });

    updater.handle_event(turn4).await?;
    println!("   ✓ Agent: [Structured response with password reset steps]");

    // Step 6: Query the dialog
    println!("\n6. Querying dialog information...");
    let updater_arc = Arc::new(RwLock::new(updater));
    let query_handler = DialogQueryHandler::new(updater_arc.clone());

    let result = query_handler.execute(DialogQuery::GetDialogById { dialog_id }).await;
    if let DialogQueryResult::Dialog(Some(dialog)) = result {
        println!("   Dialog type: {:?}", dialog.dialog_type);
        println!("   Status: {:?}", dialog.status);
        println!("   Turns: {}", dialog.turns.len());
        println!("   Participants: {}", dialog.participants.len());
    }

    // Step 7: End the dialog
    println!("\n7. Ending dialog...");
    let end_event = DialogDomainEvent::DialogEnded(DialogEnded {
        dialog_id,
        ended_at: Utc::now(),
        reason: Some("Issue resolved - password reset instructions provided".to_string()),
        final_metrics: ConversationMetrics {
            turn_count: 4,
            avg_response_time_ms: 215.0,
            topic_switches: 0,
            clarification_count: 1,
            sentiment_trend: 0.6,
            coherence_score: 0.92,
        },
    });

    let mut updater = updater_arc.write().await;
    updater.handle_event(end_event).await?;
    println!("   ✓ Dialog ended successfully");

    // Final query
    println!("\n8. Final dialog state:");
    let result = query_handler.execute(DialogQuery::GetDialogById { dialog_id }).await;
    if let DialogQueryResult::Dialog(Some(dialog)) = result {
        println!("   Status: {:?}", dialog.status);
        if let Some(metrics) = &dialog.metrics {
            println!("   Average response time: {:.0}ms", metrics.avg_response_time_ms);
            println!("   Sentiment trend: {:.2}", metrics.sentiment_trend);
            println!("   Coherence score: {:.2}", metrics.coherence_score);
        }
    }

    println!("\n=== Example completed successfully! ===");
    Ok(())
}