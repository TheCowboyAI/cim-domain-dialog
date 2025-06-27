# CIM Dialog Domain

The Dialog domain manages conversational interactions between users and AI agents within the CIM system.

## Overview

The Dialog domain provides:
- Conversation lifecycle management
- Message tracking and history
- Multi-participant conversations
- Context preservation across interactions
- Structured and unstructured message types

## Key Concepts

### Conversation
Represents a dialogue session between participants with:
- Unique conversation ID
- List of participants (users, agents)
- Conversation context and metadata
- Message history
- State tracking (active, paused, ended)

### Participants
- **Users**: Human participants in conversations
- **Agents**: AI assistants or automated systems
- Each participant has unique ID and attributes

### Messages
- **Text Messages**: Plain text communication
- **Structured Messages**: JSON/data payloads for rich interactions
- Timestamps and sender attribution
- Optional metadata and attachments

## Architecture

```
┌─────────────────┐
│   Commands      │
├─────────────────┤
│ StartConversation│
│ SendMessage     │
│ EndConversation │
└────────┬────────┘
         │
    ┌────▼────┐
    │Aggregate│
    │  Logic  │
    └────┬────┘
         │
┌────────▼────────┐
│     Events      │
├─────────────────┤
│ConversationStarted│
│ MessageSent     │
│ConversationEnded│
└─────────────────┘
```

## Usage Example

```rust
use cim_domain_dialog::{
    commands::StartConversation,
    value_objects::{Participant, ConversationContext},
};

// Start a new conversation
let command = StartConversation {
    conversation_id: ConversationId::new(),
    participants: vec![
        Participant::User { id, name: "Alice".into() },
        Participant::Agent { id, name: "Assistant".into() },
    ],
    context: ConversationContext::default(),
};
```

## Integration Points

- **Agent Domain**: AI agents participate in conversations
- **Identity Domain**: User authentication and profiles
- **Workflow Domain**: Conversations can trigger workflows
- **Document Domain**: Share and discuss documents

## Testing

Run domain tests:
```bash
cargo test -p cim-domain-dialog
```

## See Also

- [User Stories](doc/user-stories.md)
- [API Documentation](doc/api.md)
- [Examples](examples/) 