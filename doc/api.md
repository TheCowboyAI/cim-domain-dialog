# Dialog API Documentation

## Overview

The Dialog domain API provides commands, queries, and events for {domain purpose}.

## Commands

### CreateDialog

Creates a new dialog in the system.

```rust
use cim_domain_dialog::commands::CreateDialog;

let command = CreateDialog {
    id: DialogId::new(),
    // ... fields
};
```

**Fields:**
- `id`: Unique identifier for the dialog
- `field1`: Description
- `field2`: Description

**Validation:**
- Field1 must be non-empty
- Field2 must be valid

**Events Emitted:**
- `DialogCreated`

### UpdateDialog

Updates an existing dialog.

```rust
use cim_domain_dialog::commands::UpdateDialog;

let command = UpdateDialog {
    id: entity_id,
    // ... fields to update
};
```

**Fields:**
- `id`: Identifier of the dialog to update
- `field1`: New value (optional)

**Events Emitted:**
- `DialogUpdated`

## Queries

### GetDialogById

Retrieves a dialog by its identifier.

```rust
use cim_domain_dialog::queries::GetDialogById;

let query = GetDialogById {
    id: entity_id,
};
```

**Returns:** `Option<DialogView>`

### List{Entities}

Lists all {entities} with optional filtering.

```rust
use cim_domain_dialog::queries::List{Entities};

let query = List{Entities} {
    filter: Some(Filter {
        // ... filter criteria
    }),
    pagination: Some(Pagination {
        page: 1,
        per_page: 20,
    }),
};
```

**Returns:** `Vec<DialogView>`

## Events

### DialogCreated

Emitted when a new dialog is created.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogCreated {
    pub id: DialogId,
    pub timestamp: SystemTime,
    // ... other fields
}
```

### DialogUpdated

Emitted when a dialog is updated.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogUpdated {
    pub id: DialogId,
    pub changes: Vec<FieldChange>,
    pub timestamp: SystemTime,
}
```

## Value Objects

### DialogId

Unique identifier for {entities}.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DialogId(Uuid);

impl DialogId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}
```

### {ValueObject}

Represents {description}.

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct {ValueObject} {
    pub field1: String,
    pub field2: i32,
}
```

## Error Handling

The domain uses the following error types:

```rust
#[derive(Debug, thiserror::Error)]
pub enum DialogError {
    #[error("dialog not found: {id}")]
    NotFound { id: DialogId },
    
    #[error("Invalid {field}: {reason}")]
    ValidationError { field: String, reason: String },
    
    #[error("Operation not allowed: {reason}")]
    Forbidden { reason: String },
}
```

## Usage Examples

### Creating a New Dialog

```rust
use cim_domain_dialog::{
    commands::CreateDialog,
    handlers::handle_create_dialog,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = CreateDialog {
        id: DialogId::new(),
        name: "Example".to_string(),
        // ... other fields
    };
    
    let events = handle_create_dialog(command).await?;
    
    for event in events {
        println!("Event emitted: {:?}", event);
    }
    
    Ok(())
}
```

### Querying {Entities}

```rust
use cim_domain_dialog::{
    queries::{List{Entities}, execute_query},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let query = List{Entities} {
        filter: None,
        pagination: Some(Pagination {
            page: 1,
            per_page: 10,
        }),
    };
    
    let results = execute_query(query).await?;
    
    for item in results {
        println!("{:?}", item);
    }
    
    Ok(())
}
```

## Integration with Other Domains

This domain integrates with:

- **{Other Domain}**: Description of integration
- **{Other Domain}**: Description of integration

## Performance Considerations

- Commands are processed asynchronously
- Queries use indexed projections for fast retrieval
- Events are published to NATS for distribution

## Security Considerations

- All commands require authentication
- Authorization is enforced at the aggregate level
- Sensitive data is encrypted in events 