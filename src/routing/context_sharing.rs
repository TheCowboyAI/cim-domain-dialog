//! Context sharing and propagation for multi-agent dialogs

use crate::value_objects::{ContextVariable, ContextScope};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Shared context between multiple agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedContext {
    /// Variables in the shared context
    pub variables: HashMap<String, ContextVariable>,
    
    /// Context metadata
    pub metadata: HashMap<String, serde_json::Value>,
    
    /// Last update timestamp
    pub last_updated: DateTime<Utc>,
    
    /// Version for conflict resolution
    pub version: u64,
}

impl SharedContext {
    /// Create a new shared context
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            metadata: HashMap::new(),
            last_updated: Utc::now(),
            version: 1,
        }
    }
    
    /// Add or update a variable
    pub fn set_variable(&mut self, name: String, value: serde_json::Value, scope: ContextScope) {
        self.variables.insert(name.clone(), ContextVariable {
            name: name.clone(),
            value,
            scope,
            set_at: Utc::now(),
            expires_at: None,
            source: uuid::Uuid::new_v4(),
        });
        self.last_updated = Utc::now();
        self.version += 1;
    }
    
    /// Get a variable value
    pub fn get_variable(&self, name: &str) -> Option<&serde_json::Value> {
        self.variables.get(name).map(|var| &var.value)
    }
    
    /// Remove expired variables
    pub fn cleanup_expired(&mut self) {
        let now = Utc::now();
        self.variables.retain(|_, var| {
            var.expires_at.map(|exp| exp > now).unwrap_or(true)
        });
    }
    
    /// Merge another context into this one
    pub fn merge(&mut self, other: &SharedContext, strategy: &ContextMergeStrategy) {
        match strategy {
            ContextMergeStrategy::TakeNewest => {
                for (name, var) in &other.variables {
                    if let Some(existing) = self.variables.get(name) {
                        if var.set_at > existing.set_at {
                            self.variables.insert(name.clone(), var.clone());
                        }
                    } else {
                        self.variables.insert(name.clone(), var.clone());
                    }
                }
            }
            ContextMergeStrategy::TakeOldest => {
                for (name, var) in &other.variables {
                    if !self.variables.contains_key(name) {
                        self.variables.insert(name.clone(), var.clone());
                    }
                }
            }
            ContextMergeStrategy::Union => {
                for (name, var) in &other.variables {
                    self.variables.insert(name.clone(), var.clone());
                }
            }
            ContextMergeStrategy::Custom(merger) => {
                merger(self, other);
            }
        }
        
        self.last_updated = Utc::now();
        self.version += 1;
    }
}

impl Default for SharedContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Strategy for merging contexts
#[derive(Clone)]
pub enum ContextMergeStrategy {
    /// Take the newest value for each variable
    TakeNewest,
    /// Keep the oldest value for each variable
    TakeOldest,
    /// Union of all variables (last write wins)
    Union,
    /// Custom merge function
    Custom(fn(&mut SharedContext, &SharedContext)),
}

/// Context propagation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPropagation {
    /// Whether to propagate global variables
    pub propagate_global: bool,
    
    /// Whether to propagate session variables
    pub propagate_session: bool,
    
    /// Whether to propagate turn variables
    pub propagate_turn: bool,
    
    /// Specific variables to always propagate
    pub always_propagate: Vec<String>,
    
    /// Specific variables to never propagate
    pub never_propagate: Vec<String>,
}

impl Default for ContextPropagation {
    fn default() -> Self {
        Self {
            propagate_global: true,
            propagate_session: true,
            propagate_turn: false,
            always_propagate: vec![],
            never_propagate: vec![],
        }
    }
}

impl ContextPropagation {
    /// Check if a variable should be propagated
    pub fn should_propagate(&self, var: &ContextVariable) -> bool {
        // Check explicit rules first
        if self.never_propagate.contains(&var.name) {
            return false;
        }
        if self.always_propagate.contains(&var.name) {
            return true;
        }
        
        // Check scope-based rules
        match var.scope {
            ContextScope::Global => self.propagate_global,
            ContextScope::Dialog => true, // Map Session to Dialog
            ContextScope::Turn => self.propagate_turn,
            ContextScope::Topic => true, // Propagate topic-scoped vars
            ContextScope::Participant => true, // Propagate participant-scoped vars
        }
    }
    
    /// Filter context based on propagation rules
    pub fn filter_context(&self, context: &SharedContext) -> SharedContext {
        let mut filtered = SharedContext::new();
        
        for (name, var) in &context.variables {
            if self.should_propagate(var) {
                filtered.variables.insert(name.clone(), var.clone());
            }
        }
        
        filtered.metadata = context.metadata.clone();
        filtered
    }
}

/// Context synchronization for distributed agents
pub struct ContextSync {
    /// Local context version
    local_version: u64,
    
    /// Known remote versions
    remote_versions: HashMap<String, u64>,
}

impl ContextSync {
    pub fn new() -> Self {
        Self {
            local_version: 0,
            remote_versions: HashMap::new(),
        }
    }
    
    /// Check if sync is needed with a remote agent
    pub fn needs_sync(&self, agent_id: &str, remote_version: u64) -> bool {
        self.remote_versions
            .get(agent_id)
            .map(|&v| v < remote_version)
            .unwrap_or(true)
    }
    
    /// Update remote version after sync
    pub fn update_remote_version(&mut self, agent_id: String, version: u64) {
        self.remote_versions.insert(agent_id, version);
    }
    
    /// Increment local version
    pub fn increment_local_version(&mut self) {
        self.local_version += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_context_merge() {
        let mut ctx1 = SharedContext::new();
        ctx1.set_variable("var1".to_string(), json!("value1"), ContextScope::Global);
        ctx1.set_variable("shared".to_string(), json!("old"), ContextScope::Session);
        
        let mut ctx2 = SharedContext::new();
        ctx2.set_variable("var2".to_string(), json!("value2"), ContextScope::Global);
        ctx2.set_variable("shared".to_string(), json!("new"), ContextScope::Session);
        
        // Test TakeNewest strategy
        let mut merged = ctx1.clone();
        merged.merge(&ctx2, &ContextMergeStrategy::TakeNewest);
        
        assert_eq!(merged.get_variable("var1"), Some(&json!("value1")));
        assert_eq!(merged.get_variable("var2"), Some(&json!("value2")));
        assert_eq!(merged.get_variable("shared"), Some(&json!("new")));
    }
    
    #[test]
    fn test_context_propagation() {
        let mut context = SharedContext::new();
        context.set_variable("global_var".to_string(), json!("global"), ContextScope::Global);
        context.set_variable("session_var".to_string(), json!("session"), ContextScope::Session);
        context.set_variable("turn_var".to_string(), json!("turn"), ContextScope::Turn);
        
        let prop_rules = ContextPropagation {
            propagate_global: true,
            propagate_session: true,
            propagate_turn: false,
            always_propagate: vec![],
            never_propagate: vec![],
        };
        
        let filtered = prop_rules.filter_context(&context);
        
        assert!(filtered.variables.contains_key("global_var"));
        assert!(filtered.variables.contains_key("session_var"));
        assert!(!filtered.variables.contains_key("turn_var"));
    }
}