//! Dialog domain module
//!
//! This domain manages conversations, dialog contexts, and interaction history
//! for AI agents. It provides:
//! - Multi-turn conversation tracking with context preservation
//! - Topic management and semantic understanding
//! - Context switching and state management
//! - Participant tracking (agents, users, systems)
//! - Integration with conceptual spaces for semantic analysis
//!
//! The Dialog domain serves as the memory system for agent conversations,
//! storing both the structure (turns, topics) and semantics (embeddings, context)
//! of interactions.

pub mod aggregate;
pub mod commands;
pub mod events;
pub mod handlers;
pub mod projections;
pub mod queries;
pub mod routing;
pub mod value_objects;

// Re-export main types
pub use aggregate::{
    ContextState, ConversationContext, Dialog, DialogMarker, DialogStatus, DialogType,
};

pub use commands::{
    AddContextVariable, AddParticipant, AddTurn, EndDialog, MarkTopicComplete, PauseDialog,
    RemoveParticipant, ResumeDialog, SetDialogMetadata, StartDialog, SwitchContext, UpdateContext,
};

pub use events::{
    ContextSwitched, ContextUpdated, ContextVariableAdded, DialogDomainEvent, DialogEnded, 
    DialogMetadataSet, DialogPaused, DialogResumed, DialogStarted, ParticipantAdded, 
    ParticipantRemoved, TopicCompleted, TurnAdded,
};

pub use handlers::{DialogCommandHandler, DialogEventHandler};
pub use projections::{SimpleDialogView, SimpleProjectionUpdater};
pub use queries::{DialogQuery, DialogQueryHandler};

pub use value_objects::{
    ContextScope, ContextVariable, ConversationMetrics, EngagementMetrics, Message, MessageContent,
    MessageIntent, Participant, ParticipantRole, ParticipantType, Topic, TopicRelevance,
    TopicStatus, Turn, TurnMetadata, TurnType,
};
