//! Dialog domain events

use chrono::{DateTime, Utc};
use cim_domain::DomainEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::value_objects::{ContextVariable, ConversationMetrics, Participant, Topic, Turn};

/// Dialog started event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogStarted {
    pub dialog_id: Uuid,
    pub dialog_type: crate::DialogType,
    pub primary_participant: Participant,
    pub started_at: DateTime<Utc>,
}

impl DomainEvent for DialogStarted {
    fn subject(&self) -> String {
        "dialog.started.v1".to_string()
    }

    fn aggregate_id(&self) -> Uuid {
        self.dialog_id
    }

    fn event_type(&self) -> &'static str {
        "DialogStarted"
    }
}

/// Dialog ended event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogEnded {
    pub dialog_id: Uuid,
    pub ended_at: DateTime<Utc>,
    pub reason: Option<String>,
    pub final_metrics: ConversationMetrics,
}

impl DomainEvent for DialogEnded {
    fn subject(&self) -> String {
        "dialog.ended.v1".to_string()
    }

    fn aggregate_id(&self) -> Uuid {
        self.dialog_id
    }

    fn event_type(&self) -> &'static str {
        "DialogEnded"
    }
}

/// Turn added to dialog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnAdded {
    pub dialog_id: Uuid,
    pub turn: Turn,
    pub turn_number: u32,
}

impl DomainEvent for TurnAdded {
    fn subject(&self) -> String {
        "dialog.turn.added.v1".to_string()
    }

    fn aggregate_id(&self) -> Uuid {
        self.dialog_id
    }

    fn event_type(&self) -> &'static str {
        "TurnAdded"
    }
}

/// Context switched event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSwitched {
    pub dialog_id: Uuid,
    pub previous_topic: Option<Uuid>,
    pub new_topic: Topic,
    pub switched_at: DateTime<Utc>,
}

impl DomainEvent for ContextSwitched {
    fn subject(&self) -> String {
        "dialog.context.switched.v1".to_string()
    }

    fn aggregate_id(&self) -> Uuid {
        self.dialog_id
    }

    fn event_type(&self) -> &'static str {
        "ContextSwitched"
    }
}

/// Context updated event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextUpdated {
    pub dialog_id: Uuid,
    pub updated_variables: HashMap<String, serde_json::Value>,
    pub updated_at: DateTime<Utc>,
}

impl DomainEvent for ContextUpdated {
    fn subject(&self) -> String {
        "dialog.context.updated.v1".to_string()
    }

    fn aggregate_id(&self) -> Uuid {
        self.dialog_id
    }

    fn event_type(&self) -> &'static str {
        "ContextUpdated"
    }
}

/// Dialog paused event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogPaused {
    pub dialog_id: Uuid,
    pub paused_at: DateTime<Utc>,
    pub context_snapshot: HashMap<String, ContextVariable>,
}

impl DomainEvent for DialogPaused {
    fn subject(&self) -> String {
        "dialog.paused.v1".to_string()
    }

    fn aggregate_id(&self) -> Uuid {
        self.dialog_id
    }

    fn event_type(&self) -> &'static str {
        "DialogPaused"
    }
}

/// Dialog resumed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogResumed {
    pub dialog_id: Uuid,
    pub resumed_at: DateTime<Utc>,
}

impl DomainEvent for DialogResumed {
    fn subject(&self) -> String {
        "dialog.resumed.v1".to_string()
    }

    fn aggregate_id(&self) -> Uuid {
        self.dialog_id
    }

    fn event_type(&self) -> &'static str {
        "DialogResumed"
    }
}

/// Dialog metadata set event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogMetadataSet {
    pub dialog_id: Uuid,
    pub key: String,
    pub value: serde_json::Value,
    pub set_at: DateTime<Utc>,
}

impl DomainEvent for DialogMetadataSet {
    fn subject(&self) -> String {
        "dialog.metadata.set.v1".to_string()
    }

    fn aggregate_id(&self) -> Uuid {
        self.dialog_id
    }

    fn event_type(&self) -> &'static str {
        "DialogMetadataSet"
    }
}

/// Participant added event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantAdded {
    pub dialog_id: Uuid,
    pub participant: Participant,
    pub added_at: DateTime<Utc>,
}

impl DomainEvent for ParticipantAdded {
    fn subject(&self) -> String {
        "dialog.participant.added.v1".to_string()
    }

    fn aggregate_id(&self) -> Uuid {
        self.dialog_id
    }

    fn event_type(&self) -> &'static str {
        "ParticipantAdded"
    }
}

/// Participant removed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantRemoved {
    pub dialog_id: Uuid,
    pub participant_id: Uuid,
    pub removed_at: DateTime<Utc>,
    pub reason: Option<String>,
}

impl DomainEvent for ParticipantRemoved {
    fn subject(&self) -> String {
        "dialog.participant.removed.v1".to_string()
    }

    fn aggregate_id(&self) -> Uuid {
        self.dialog_id
    }

    fn event_type(&self) -> &'static str {
        "ParticipantRemoved"
    }
}

/// Topic completed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicCompleted {
    pub dialog_id: Uuid,
    pub topic_id: Uuid,
    pub completed_at: DateTime<Utc>,
    pub resolution: Option<String>,
}

impl DomainEvent for TopicCompleted {
    fn subject(&self) -> String {
        "dialog.topic.completed.v1".to_string()
    }

    fn aggregate_id(&self) -> Uuid {
        self.dialog_id
    }

    fn event_type(&self) -> &'static str {
        "TopicCompleted"
    }
}

/// Context variable added event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextVariableAdded {
    pub dialog_id: Uuid,
    pub variable: ContextVariable,
    pub added_at: DateTime<Utc>,
}

impl DomainEvent for ContextVariableAdded {
    fn subject(&self) -> String {
        "dialog.context.variable.added.v1".to_string()
    }

    fn aggregate_id(&self) -> Uuid {
        self.dialog_id
    }

    fn event_type(&self) -> &'static str {
        "ContextVariableAdded"
    }
}

/// Dialog domain event enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogDomainEvent {
    DialogStarted(DialogStarted),
    DialogEnded(DialogEnded),
    DialogPaused(DialogPaused),
    DialogResumed(DialogResumed),
    TurnAdded(TurnAdded),
    ParticipantAdded(ParticipantAdded),
    ParticipantRemoved(ParticipantRemoved),
    ContextSwitched(ContextSwitched),
    ContextUpdated(ContextUpdated),
    ContextVariableAdded(ContextVariableAdded),
    DialogMetadataSet(DialogMetadataSet),
    TopicCompleted(TopicCompleted),
}

impl DomainEvent for DialogDomainEvent {
    fn subject(&self) -> String {
        match self {
            Self::DialogStarted(e) => e.subject(),
            Self::DialogEnded(e) => e.subject(),
            Self::DialogPaused(e) => e.subject(),
            Self::DialogResumed(e) => e.subject(),
            Self::TurnAdded(e) => e.subject(),
            Self::ParticipantAdded(e) => e.subject(),
            Self::ParticipantRemoved(e) => e.subject(),
            Self::ContextSwitched(e) => e.subject(),
            Self::ContextUpdated(e) => e.subject(),
            Self::ContextVariableAdded(e) => e.subject(),
            Self::DialogMetadataSet(e) => e.subject(),
            Self::TopicCompleted(e) => e.subject(),
        }
    }

    fn aggregate_id(&self) -> Uuid {
        match self {
            Self::DialogStarted(e) => e.aggregate_id(),
            Self::DialogEnded(e) => e.aggregate_id(),
            Self::DialogPaused(e) => e.aggregate_id(),
            Self::DialogResumed(e) => e.aggregate_id(),
            Self::TurnAdded(e) => e.aggregate_id(),
            Self::ParticipantAdded(e) => e.aggregate_id(),
            Self::ParticipantRemoved(e) => e.aggregate_id(),
            Self::ContextSwitched(e) => e.aggregate_id(),
            Self::ContextUpdated(e) => e.aggregate_id(),
            Self::ContextVariableAdded(e) => e.aggregate_id(),
            Self::DialogMetadataSet(e) => e.aggregate_id(),
            Self::TopicCompleted(e) => e.aggregate_id(),
        }
    }

    fn event_type(&self) -> &'static str {
        match self {
            Self::DialogStarted(e) => e.event_type(),
            Self::DialogEnded(e) => e.event_type(),
            Self::DialogPaused(e) => e.event_type(),
            Self::DialogResumed(e) => e.event_type(),
            Self::TurnAdded(e) => e.event_type(),
            Self::ParticipantAdded(e) => e.event_type(),
            Self::ParticipantRemoved(e) => e.event_type(),
            Self::ContextSwitched(e) => e.event_type(),
            Self::ContextUpdated(e) => e.event_type(),
            Self::ContextVariableAdded(e) => e.event_type(),
            Self::DialogMetadataSet(e) => e.event_type(),
            Self::TopicCompleted(e) => e.event_type(),
        }
    }
}
