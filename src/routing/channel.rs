//! Dialog channels for agent communication groups

// Use a simple string ID instead of importing from agent coordination
type AgentId = String;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashSet;

/// Unique identifier for a dialog channel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(pub Uuid);

impl ChannelId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ChannelId {
    fn default() -> Self {
        Self::new()
    }
}

/// Types of dialog channels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelType {
    /// One-to-one dialog between two agents
    Direct,
    /// Group dialog with multiple agents
    Group,
    /// Broadcast channel (one-to-many)
    Broadcast,
    /// Topic-based channel
    Topic,
    /// Task-specific channel
    Task,
}

/// A dialog channel representing a communication group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogChannel {
    /// Unique identifier
    pub id: ChannelId,
    
    /// Type of channel
    pub channel_type: ChannelType,
    
    /// Agents in this channel
    pub agents: Vec<AgentId>,
    
    /// Channel metadata
    pub metadata: serde_json::Value,
    
    /// When the channel was created
    pub created_at: DateTime<Utc>,
    
    /// Whether the channel is active
    pub is_active: bool,
    
    /// Topic or purpose of the channel
    pub topic: Option<String>,
}

impl DialogChannel {
    /// Create a new dialog channel
    pub fn new(agents: Vec<AgentId>, channel_type: ChannelType) -> Self {
        Self {
            id: ChannelId::new(),
            channel_type,
            agents,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            created_at: Utc::now(),
            is_active: true,
            topic: None,
        }
    }
    
    /// Create a direct channel between two agents
    pub fn direct(agent1: AgentId, agent2: AgentId) -> Self {
        Self::new(vec![agent1, agent2], ChannelType::Direct)
    }
    
    /// Create a group channel
    pub fn group(agents: Vec<AgentId>) -> Self {
        Self::new(agents, ChannelType::Group)
    }
    
    /// Create a broadcast channel
    pub fn broadcast(broadcaster: AgentId, receivers: Vec<AgentId>) -> Self {
        let mut agents = vec![broadcaster];
        agents.extend(receivers);
        Self::new(agents, ChannelType::Broadcast)
    }
    
    /// Create a topic-based channel
    pub fn topic(agents: Vec<AgentId>, topic: String) -> Self {
        let mut channel = Self::new(agents, ChannelType::Topic);
        channel.topic = Some(topic);
        channel
    }
    
    /// Add an agent to the channel
    pub fn add_agent(&mut self, agent: AgentId) -> bool {
        if !self.agents.contains(&agent) {
            self.agents.push(agent);
            true
        } else {
            false
        }
    }
    
    /// Remove an agent from the channel
    pub fn remove_agent(&mut self, agent: &AgentId) -> bool {
        let initial_len = self.agents.len();
        self.agents.retain(|a| a != agent);
        self.agents.len() < initial_len
    }
    
    /// Check if an agent is in the channel
    pub fn has_agent(&self, agent: &AgentId) -> bool {
        self.agents.contains(agent)
    }
    
    /// Get the number of agents in the channel
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }
    
    /// Close the channel
    pub fn close(&mut self) {
        self.is_active = false;
    }
    
    /// Check if this is a private channel (direct between two agents)
    pub fn is_private(&self) -> bool {
        matches!(self.channel_type, ChannelType::Direct) && self.agents.len() == 2
    }
    
    /// Get unique agent pairs for direct messaging
    pub fn get_agent_pairs(&self) -> Vec<(AgentId, AgentId)> {
        let mut pairs = Vec::new();
        for i in 0..self.agents.len() {
            for j in (i + 1)..self.agents.len() {
                pairs.push((self.agents[i].clone(), self.agents[j].clone()));
            }
        }
        pairs
    }
}

/// Channel manager for tracking active channels
#[derive(Debug, Default)]
pub struct ChannelManager {
    channels: HashSet<ChannelId>,
    agent_channels: std::collections::HashMap<AgentId, HashSet<ChannelId>>,
}

impl ChannelManager {
    /// Create a new channel manager
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Register a channel
    pub fn register_channel(&mut self, channel: &DialogChannel) {
        self.channels.insert(channel.id);
        
        for agent in &channel.agents {
            self.agent_channels
                .entry(agent.clone())
                .or_default()
                .insert(channel.id);
        }
    }
    
    /// Unregister a channel
    pub fn unregister_channel(&mut self, channel_id: &ChannelId, agents: &[AgentId]) {
        self.channels.remove(channel_id);
        
        for agent in agents {
            if let Some(channels) = self.agent_channels.get_mut(agent) {
                channels.remove(channel_id);
            }
        }
    }
    
    /// Get all channels for an agent
    pub fn get_agent_channels(&self, agent: &AgentId) -> Vec<ChannelId> {
        self.agent_channels
            .get(agent)
            .map(|channels| channels.iter().copied().collect())
            .unwrap_or_default()
    }
    
    /// Check if a channel exists
    pub fn channel_exists(&self, channel_id: &ChannelId) -> bool {
        self.channels.contains(channel_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_direct_channel() {
        let agent1 = "agent-1".to_string();
        let agent2 = "agent-2".to_string();
        
        let channel = DialogChannel::direct(agent1.clone(), agent2.clone());
        
        assert_eq!(channel.channel_type, ChannelType::Direct);
        assert_eq!(channel.agent_count(), 2);
        assert!(channel.has_agent(&agent1));
        assert!(channel.has_agent(&agent2));
        assert!(channel.is_private());
    }
    
    #[test]
    fn test_group_channel() {
        let agents = vec![
            "agent-1".to_string(),
            "agent-2".to_string(),
            "agent-3".to_string(),
        ];
        
        let mut channel = DialogChannel::group(agents.clone());
        
        assert_eq!(channel.channel_type, ChannelType::Group);
        assert_eq!(channel.agent_count(), 3);
        assert!(!channel.is_private());
        
        // Add new agent
        let new_agent = "agent-4".to_string();
        assert!(channel.add_agent(new_agent.clone()));
        assert_eq!(channel.agent_count(), 4);
        
        // Remove agent
        assert!(channel.remove_agent(&agents[0]));
        assert_eq!(channel.agent_count(), 3);
    }
}