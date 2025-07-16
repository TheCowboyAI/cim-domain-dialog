//! Routing strategies for agent dialog distribution

use crate::value_objects::{Message, Participant, MessageIntent};
use crate::routing::{RoutingDecision, SharedContext};
// Use a simple string ID instead of importing from agent coordination
type AgentId = String;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for dialog routing strategies
pub trait RoutingStrategy: Send + Sync {
    /// Route a message to target agents
    fn route(
        &self,
        message: &Message,
        participants: &[&Participant],
        context: &SharedContext,
        agent_capabilities: &HashMap<AgentId, Vec<String>>,
    ) -> Option<RoutingDecision>;
    
    /// Get the priority of this strategy (higher = preferred)
    fn priority(&self) -> f32 {
        1.0
    }
    
    /// Get the name of this strategy
    fn name(&self) -> &str;
}

/// Broadcast strategy - sends to all agents
pub struct BroadcastStrategy {
    priority: f32,
}

impl BroadcastStrategy {
    pub fn new() -> Self {
        Self { priority: 0.5 }
    }
}

impl RoutingStrategy for BroadcastStrategy {
    fn route(
        &self,
        _message: &Message,
        participants: &[&Participant],
        _context: &SharedContext,
        _agent_capabilities: &HashMap<AgentId, Vec<String>>,
    ) -> Option<RoutingDecision> {
        let targets: Vec<AgentId> = participants
            .iter()
            .map(|p| p.id.to_string())
            .collect();
        
        if targets.is_empty() {
            return None;
        }
        
        Some(RoutingDecision {
            targets,
            strategy: self.name().to_string(),
            confidence: 1.0,
            metadata: HashMap::new(),
        })
    }
    
    fn priority(&self) -> f32 {
        self.priority
    }
    
    fn name(&self) -> &str {
        "broadcast"
    }
}

/// Capability-based routing strategy
pub struct CapabilityBasedStrategy {
    priority: f32,
}

impl CapabilityBasedStrategy {
    pub fn new() -> Self {
        Self { priority: 2.0 }
    }
    
    /// Extract required capabilities from message
    fn extract_required_capabilities(&self, message: &Message) -> Vec<String> {
        let mut capabilities = Vec::new();
        
        // Analyze message intent
        match &message.intent {
            Some(MessageIntent::Command) => {
                // Look for keywords in message content
                if let crate::value_objects::MessageContent::Text(text) = &message.content {
                    let text_lower = text.to_lowercase();
                    
                    if text_lower.contains("deploy") {
                        capabilities.push("deployment".to_string());
                    }
                    if text_lower.contains("monitor") || text_lower.contains("alert") {
                        capabilities.push("monitoring".to_string());
                    }
                    if text_lower.contains("analyze") || text_lower.contains("report") {
                        capabilities.push("analysis".to_string());
                    }
                    if text_lower.contains("configure") || text_lower.contains("setting") {
                        capabilities.push("configuration".to_string());
                    }
                }
            }
            Some(MessageIntent::Question) => {
                capabilities.push("query_processing".to_string());
            }
            _ => {}
        }
        
        // Check for explicit capabilities in message content
        // (metadata field doesn't exist in this Message struct)
        // In a real implementation, we could extract capabilities from structured content
        
        capabilities
    }
}

impl RoutingStrategy for CapabilityBasedStrategy {
    fn route(
        &self,
        message: &Message,
        participants: &[&Participant],
        _context: &SharedContext,
        agent_capabilities: &HashMap<AgentId, Vec<String>>,
    ) -> Option<RoutingDecision> {
        let required_capabilities = self.extract_required_capabilities(message);
        
        if required_capabilities.is_empty() {
            return None;
        }
        
        let mut targets = Vec::new();
        let mut capability_scores = HashMap::new();
        
        for participant in participants {
            let agent_id = participant.id.to_string();
            
            if let Some(capabilities) = agent_capabilities.get(&agent_id) {
                let mut score = 0.0;
                let mut matched = 0;
                
                for required in &required_capabilities {
                    if capabilities.contains(required) {
                        matched += 1;
                        score += 1.0;
                    }
                }
                
                if matched > 0 {
                    targets.push(agent_id.clone());
                    capability_scores.insert(agent_id.to_string(), score / required_capabilities.len() as f32);
                }
            }
        }
        
        if targets.is_empty() {
            return None;
        }
        
        let avg_score: f32 = capability_scores.values().sum::<f32>() / capability_scores.len() as f32;
        
        Some(RoutingDecision {
            targets,
            strategy: self.name().to_string(),
            confidence: avg_score,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("required_capabilities".to_string(), serde_json::json!(required_capabilities));
                meta.insert("capability_scores".to_string(), serde_json::json!(capability_scores));
                meta
            },
        })
    }
    
    fn priority(&self) -> f32 {
        self.priority
    }
    
    fn name(&self) -> &str {
        "capability_based"
    }
}

/// Round-robin routing strategy
pub struct RoundRobinStrategy {
    last_index: Arc<RwLock<usize>>,
    priority: f32,
}

impl RoundRobinStrategy {
    pub fn new() -> Self {
        Self {
            last_index: Arc::new(RwLock::new(0)),
            priority: 1.0,
        }
    }
}

impl RoutingStrategy for RoundRobinStrategy {
    fn route(
        &self,
        _message: &Message,
        participants: &[&Participant],
        _context: &SharedContext,
        _agent_capabilities: &HashMap<AgentId, Vec<String>>,
    ) -> Option<RoutingDecision> {
        if participants.is_empty() {
            return None;
        }
        
        let last_index = self.last_index.clone();
        let participant_count = participants.len();
        
        // Use blocking read since this is synchronous
        let current_index = {
            let mut index = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(last_index.write())
            });
            *index = (*index + 1) % participant_count;
            *index
        };
        
        let target = participants[current_index].id.to_string();
        
        Some(RoutingDecision {
            targets: vec![target],
            strategy: self.name().to_string(),
            confidence: 1.0,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("round_robin_index".to_string(), serde_json::json!(current_index));
                meta
            },
        })
    }
    
    fn priority(&self) -> f32 {
        self.priority
    }
    
    fn name(&self) -> &str {
        "round_robin"
    }
}

/// Priority-based routing strategy
pub struct PriorityBasedStrategy {
    agent_priorities: HashMap<AgentId, u8>,
}

impl PriorityBasedStrategy {
    pub fn new(agent_priorities: HashMap<AgentId, u8>) -> Self {
        Self { agent_priorities }
    }
}

impl RoutingStrategy for PriorityBasedStrategy {
    fn route(
        &self,
        message: &Message,
        participants: &[&Participant],
        _context: &SharedContext,
        _agent_capabilities: &HashMap<AgentId, Vec<String>>,
    ) -> Option<RoutingDecision> {
        // For high-priority messages, route to high-priority agents
        let priority_threshold = match &message.intent {
            Some(MessageIntent::Command) => 5,
            Some(MessageIntent::Feedback) => 3,
            _ => 7,
        };
        
        let mut targets = Vec::new();
        let mut selected_priorities = Vec::new();
        
        for participant in participants {
            let agent_id = participant.id.to_string();
            
            if let Some(&priority) = self.agent_priorities.get(&agent_id) {
                if priority <= priority_threshold {
                    targets.push(agent_id);
                    selected_priorities.push(priority);
                }
            }
        }
        
        if targets.is_empty() {
            return None;
        }
        
        let avg_priority: f32 = selected_priorities.iter().map(|&p| p as f32).sum::<f32>() 
            / selected_priorities.len() as f32;
        let confidence = 1.0 - (avg_priority / 10.0); // Higher priority = higher confidence
        
        Some(RoutingDecision {
            targets,
            strategy: self.name().to_string(),
            confidence,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("priority_threshold".to_string(), serde_json::json!(priority_threshold));
                meta.insert("average_priority".to_string(), serde_json::json!(avg_priority));
                meta
            },
        })
    }
    
    fn priority(&self) -> f32 {
        1.5
    }
    
    fn name(&self) -> &str {
        "priority_based"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_objects::{MessageContent, ParticipantRole, ParticipantType};
    use uuid::Uuid;
    use chrono::Utc;
    
    fn create_test_participant(name: &str) -> Participant {
        Participant {
            id: Uuid::new_v4(),
            name: name.to_string(),
            participant_type: ParticipantType::AIAgent,
            role: ParticipantRole::Assistant,
            metadata: HashMap::new(),
        }
    }
    
    fn create_test_message(content: &str, intent: MessageIntent) -> Message {
        Message {
            content: MessageContent::Text(content.to_string()),
            intent: Some(intent),
            language: "en".to_string(),
            sentiment: None,
            embeddings: None,
        }
    }
    
    #[test]
    fn test_broadcast_strategy() {
        let strategy = BroadcastStrategy::new();
        let participants = vec![
            create_test_participant("agent1"),
            create_test_participant("agent2"),
        ];
        let participant_refs: Vec<&Participant> = participants.iter().collect();
        
        let message = create_test_message("Hello", MessageIntent::Statement);
        let context = SharedContext::new();
        let capabilities = HashMap::new();
        
        let decision = strategy.route(&message, &participant_refs, &context, &capabilities);
        
        assert!(decision.is_some());
        let decision = decision.unwrap();
        assert_eq!(decision.targets.len(), 2);
        assert_eq!(decision.strategy, "broadcast");
    }
    
    #[test]
    fn test_capability_based_strategy() {
        let strategy = CapabilityBasedStrategy::new();
        let participants = vec![
            create_test_participant("deploy-agent"),
            create_test_participant("monitor-agent"),
        ];
        let participant_refs: Vec<&Participant> = participants.iter().collect();
        
        let message = create_test_message("Deploy the new service", MessageIntent::Command);
        let context = SharedContext::new();
        let mut capabilities = HashMap::new();
        capabilities.insert(
            participants[0].id.to_string(),
            vec!["deployment".to_string()],
        );
        capabilities.insert(
            participants[1].id.to_string(),
            vec!["monitoring".to_string()],
        );
        
        let decision = strategy.route(&message, &participant_refs, &context, &capabilities);
        
        assert!(decision.is_some());
        let decision = decision.unwrap();
        assert_eq!(decision.targets.len(), 1); // Only deploy-agent should be selected
        assert_eq!(decision.strategy, "capability_based");
    }
}