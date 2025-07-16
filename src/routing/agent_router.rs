//! Agent dialog router for message distribution

use crate::value_objects::{Message, Participant, ParticipantType};
// Use a simple string ID instead of importing from agent coordination
type AgentId = String;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Routing decision for a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Target agents to receive the message
    pub targets: Vec<AgentId>,
    
    /// Routing strategy used
    pub strategy: String,
    
    /// Confidence score for the routing decision
    pub confidence: f32,
    
    /// Metadata about the routing
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Agent dialog router for intelligent message distribution
pub struct AgentDialogRouter {
    /// Available routing strategies
    strategies: Vec<Box<dyn crate::routing::strategies::RoutingStrategy>>,
    
    /// Agent capabilities cache
    agent_capabilities: HashMap<AgentId, Vec<String>>,
    
    /// Active dialog channels
    channels: HashMap<Uuid, crate::routing::channel::DialogChannel>,
}

impl AgentDialogRouter {
    /// Create a new agent dialog router
    pub fn new() -> Self {
        Self {
            strategies: vec![
                Box::new(crate::routing::strategies::BroadcastStrategy::new()),
                Box::new(crate::routing::strategies::CapabilityBasedStrategy::new()),
                Box::new(crate::routing::strategies::RoundRobinStrategy::new()),
            ],
            agent_capabilities: HashMap::new(),
            channels: HashMap::new(),
        }
    }
    
    /// Register agent capabilities
    pub fn register_agent(&mut self, agent_id: AgentId, capabilities: Vec<String>) {
        self.agent_capabilities.insert(agent_id, capabilities);
    }
    
    /// Route a message to appropriate agents
    pub fn route_message(
        &self,
        message: &Message,
        participants: &[Participant],
        context: &crate::routing::context_sharing::SharedContext,
    ) -> RoutingDecision {
        // Extract agent participants
        let agent_participants: Vec<&Participant> = participants
            .iter()
            .filter(|p| matches!(p.participant_type, ParticipantType::AIAgent))
            .collect();
        
        if agent_participants.is_empty() {
            return RoutingDecision {
                targets: vec![],
                strategy: "none".to_string(),
                confidence: 1.0,
                metadata: HashMap::new(),
            };
        }
        
        // Try each strategy and pick the best one
        let mut best_decision: Option<RoutingDecision> = None;
        let mut best_score = 0.0;
        
        for strategy in &self.strategies {
            if let Some(decision) = strategy.route(message, &agent_participants, context, &self.agent_capabilities) {
                let score = decision.confidence * strategy.priority();
                if score > best_score {
                    best_score = score;
                    best_decision = Some(decision);
                }
            }
        }
        
        best_decision.unwrap_or_else(|| RoutingDecision {
            targets: vec![],
            strategy: "fallback".to_string(),
            confidence: 0.0,
            metadata: HashMap::new(),
        })
    }
    
    /// Create a dialog channel for a group of agents
    pub fn create_agent_channel(
        &mut self,
        agents: Vec<AgentId>,
        channel_type: crate::routing::channel::ChannelType,
    ) -> crate::routing::channel::ChannelId {
        let channel = crate::routing::channel::DialogChannel::new(agents, channel_type);
        let channel_id = channel.id;
        self.channels.insert(channel.id.0, channel);
        channel_id
    }
    
    /// Get agents in a channel
    pub fn get_channel_agents(&self, channel_id: &crate::routing::channel::ChannelId) -> Option<Vec<AgentId>> {
        self.channels.get(&channel_id.0).map(|c| c.agents.clone())
    }
    
    /// Broadcast to all agents in a channel
    pub fn broadcast_to_channel(
        &self,
        channel_id: &crate::routing::channel::ChannelId,
        message: &Message,
    ) -> Option<RoutingDecision> {
        self.channels.get(&channel_id.0).map(|channel| {
            RoutingDecision {
                targets: channel.agents.clone(),
                strategy: "channel_broadcast".to_string(),
                confidence: 1.0,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("channel_id".to_string(), serde_json::json!(channel_id.0));
                    meta.insert("channel_type".to_string(), serde_json::json!(channel.channel_type));
                    meta
                },
            }
        })
    }
}

impl Default for AgentDialogRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_objects::{MessageContent, MessageIntent};
    use chrono::Utc;
    
    #[test]
    fn test_agent_routing() {
        let mut router = AgentDialogRouter::new();
        
        // Register agents
        router.register_agent(
            "deploy-agent".to_string(),
            vec!["deployment".to_string(), "infrastructure".to_string()],
        );
        router.register_agent(
            "monitor-agent".to_string(),
            vec!["monitoring".to_string(), "alerts".to_string()],
        );
        
        // Create participants
        let participants = vec![
            Participant {
                id: Uuid::new_v4(),
                name: "Deploy Agent".to_string(),
                participant_type: ParticipantType::AIAgent,
                role: crate::value_objects::ParticipantRole::Assistant,
                metadata: HashMap::new(),
            },
            Participant {
                id: Uuid::new_v4(),
                name: "Monitor Agent".to_string(),
                participant_type: ParticipantType::AIAgent,
                role: crate::value_objects::ParticipantRole::Assistant,
                metadata: HashMap::new(),
            },
        ];
        
        // Create a deployment message
        let message = Message {
            content: MessageContent::Text("Deploy the new service".to_string()),
            intent: Some(MessageIntent::Command),
            language: "en".to_string(),
            sentiment: None,
            embeddings: None,
        };
        
        // Route the message
        let context = crate::routing::context_sharing::SharedContext::new();
        let decision = router.route_message(&message, &participants, &context);
        
        assert!(!decision.targets.is_empty());
        assert!(decision.confidence > 0.0);
    }
}