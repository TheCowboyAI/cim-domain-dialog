//! Basic Conversation Example
//!
//! This example demonstrates how to:
//! - Start a new conversation
//! - Send messages between participants
//! - Track conversation context
//! - End a conversation

use cim_domain_dialog::{
    aggregate::Conversation,
    commands::{EndConversation, SendMessage, StartConversation},
    events::{ConversationEnded, ConversationStarted, MessageSent},
    handlers::ConversationCommandHandler,
    queries::{ConversationQueryHandler, GetConversationHistory},
    value_objects::{ConversationContext, MessageContent, Participant},
};
use std::time::SystemTime;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CIM Dialog Domain Example ===\n");

    // Initialize handlers
    let command_handler = ConversationCommandHandler::new();
    let query_handler = ConversationQueryHandler::new();

    // Create participants
    let user_id = Uuid::new_v4();
    let agent_id = Uuid::new_v4();
    let conversation_id = Uuid::new_v4();

    // Step 1: Start a conversation
    println!("1. Starting conversation...");
    let start_command = StartConversation {
        conversation_id,
        participants: vec![
            Participant::User {
                id: user_id,
                name: "Alice".to_string(),
            },
            Participant::Agent {
                id: agent_id,
                name: "Assistant".to_string(),
            },
        ],
        context: ConversationContext {
            topic: Some("Technical Support".to_string()),
            metadata: Default::default(),
        },
    };

    let events = command_handler.handle(start_command).await?;
    println!("   Conversation started! Events: {:?}\n", events.len());

    // Step 2: User sends first message
    println!("2. User sending message...");
    let user_message = SendMessage {
        conversation_id,
        sender: Participant::User {
            id: user_id,
            name: "Alice".to_string(),
        },
        content: MessageContent::Text {
            text: "Hello! I need help setting up a workflow.".to_string(),
        },
        timestamp: SystemTime::now(),
    };

    let events = command_handler.handle(user_message).await?;
    println!("   Message sent! Events: {:?}\n", events.len());

    // Step 3: Agent responds
    println!("3. Agent responding...");
    let agent_response = SendMessage {
        conversation_id,
        sender: Participant::Agent { id: agent_id, name: "Assistant".to_string() },
        content: MessageContent::Text {
            text: "I'd be happy to help you set up a workflow! What kind of workflow are you looking to create?".to_string(),
        },
        timestamp: SystemTime::now(),
    };

    let events = command_handler.handle(agent_response).await?;
    println!("   Response sent! Events: {:?}\n", events.len());

    // Step 4: User provides more details
    println!("4. User providing details...");
    let user_details = SendMessage {
        conversation_id,
        sender: Participant::User {
            id: user_id,
            name: "Alice".to_string(),
        },
        content: MessageContent::Text {
            text: "I need a document approval workflow with multiple reviewers.".to_string(),
        },
        timestamp: SystemTime::now(),
    };

    let events = command_handler.handle(user_details).await?;
    println!("   Details sent! Events: {:?}\n", events.len());

    // Step 5: Agent provides structured response
    println!("5. Agent providing structured guidance...");
    let structured_response = SendMessage {
        conversation_id,
        sender: Participant::Agent {
            id: agent_id,
            name: "Assistant".to_string(),
        },
        content: MessageContent::Structured {
            format: "workflow_steps".to_string(),
            data: serde_json::json!({
                "steps": [
                    {
                        "step": 1,
                        "action": "Create workflow template",
                        "description": "Define the approval stages and reviewers"
                    },
                    {
                        "step": 2,
                        "action": "Configure notifications",
                        "description": "Set up email/system notifications for reviewers"
                    },
                    {
                        "step": 3,
                        "action": "Test workflow",
                        "description": "Run a test document through the approval process"
                    }
                ]
            }),
        },
        timestamp: SystemTime::now(),
    };

    let events = command_handler.handle(structured_response).await?;
    println!("   Structured response sent! Events: {:?}\n", events.len());

    // Step 6: Query conversation history
    println!("6. Retrieving conversation history...");
    let history_query = GetConversationHistory {
        conversation_id,
        include_metadata: true,
    };

    let history = query_handler.handle(history_query).await?;
    println!("   Conversation has {} messages", history.messages.len());

    for (idx, message) in history.messages.iter().enumerate() {
        println!(
            "   Message {}: {} - \"{}\"",
            idx + 1,
            message.sender_name(),
            message.content_preview()
        );
    }

    println!();

    // Step 7: End conversation
    println!("7. Ending conversation...");
    let end_command = EndConversation {
        conversation_id,
        reason: Some("Issue resolved".to_string()),
    };

    let events = command_handler.handle(end_command).await?;
    println!("   Conversation ended! Events: {:?}", events.len());

    println!("\n=== Example completed successfully! ===");
    Ok(())
}

// Helper trait for demo
trait MessageHelpers {
    fn sender_name(&self) -> &str;
    fn content_preview(&self) -> String;
}

// Note: In real implementation, these would be part of the domain
impl MessageHelpers for cim_domain_dialog::value_objects::Message {
    fn sender_name(&self) -> &str {
        match &self.sender {
            Participant::User { name, .. } => name,
            Participant::Agent { name, .. } => name,
        }
    }

    fn content_preview(&self) -> String {
        match &self.content {
            MessageContent::Text { text } => {
                if text.len() > 50 {
                    format!("{&text[..47]}...")
                } else {
                    text.clone()
                }
            }
            MessageContent::Structured { format, .. } => {
                format!("[Structured: {format}]")
            }
        }
    }
}
