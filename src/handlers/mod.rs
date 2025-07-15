//! Dialog command and event handlers

pub mod command_handler;

pub use command_handler::DialogCommandHandler;

/// Handler for dialog events
pub struct DialogEventHandler;

impl DialogEventHandler {
    /// Create a new dialog event handler
    pub fn new() -> Self {
        Self
    }
}

impl Default for DialogEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

// Event handler implementations will process dialog events to update projections,
// trigger workflows, and handle cross-domain integrations