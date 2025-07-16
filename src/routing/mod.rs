//! Agent dialog routing module for multi-agent conversations

pub mod agent_router;
pub mod channel;
pub mod context_sharing;
pub mod strategies;

pub use agent_router::{AgentDialogRouter, RoutingDecision};
pub use channel::{DialogChannel, ChannelId, ChannelType};
pub use context_sharing::{ContextPropagation, SharedContext, ContextMergeStrategy};
pub use strategies::{RoutingStrategy, BroadcastStrategy, CapabilityBasedStrategy, RoundRobinStrategy};