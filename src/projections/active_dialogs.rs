//! ActiveDialogs projection - real-time tracking of active conversations
//!
//! This projection maintains a lightweight view of all currently active dialogs
//! for quick access and monitoring.

use super::DialogProjection;
use crate::aggregate::{DialogStatus, DialogType};
use crate::events::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Summary of an active dialog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveDialogSummary {
    pub dialog_id: Uuid,
    pub dialog_type: DialogType,
    pub status: DialogStatus,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub participant_count: usize,
    pub active_participant_ids: HashSet<String>,
    pub turn_count: usize,
    pub current_topic: Option<String>,
    pub current_context: String,
    pub activity_level: ActivityLevel,
}

/// Activity level of a dialog
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ActivityLevel {
    Idle,      // No activity for > 5 minutes
    Low,       // Activity within last 5 minutes
    Medium,    // Multiple turns in last 5 minutes
    High,      // Rapid back-and-forth conversation
}

impl ActivityLevel {
    fn from_activity(last_activity: DateTime<Utc>, recent_turns: usize) -> Self {
        let now = Utc::now();
        let duration = now.signed_duration_since(last_activity);
        
        if duration.num_minutes() > 5 {
            ActivityLevel::Idle
        } else if recent_turns > 10 {
            ActivityLevel::High
        } else if recent_turns > 3 {
            ActivityLevel::Medium
        } else {
            ActivityLevel::Low
        }
    }
}

/// Active dialogs projection
#[derive(Debug, Clone)]
pub struct ActiveDialogs {
    pub dialogs: HashMap<Uuid, ActiveDialogSummary>,
    pub by_participant: HashMap<String, HashSet<Uuid>>,
    pub by_type: HashMap<DialogType, HashSet<Uuid>>,
    pub by_activity: HashMap<ActivityLevel, HashSet<Uuid>>,
    pub recent_turns: HashMap<Uuid, Vec<DateTime<Utc>>>,
}

impl ActiveDialogs {
    pub fn new() -> Self {
        Self {
            dialogs: HashMap::new(),
            by_participant: HashMap::new(),
            by_type: HashMap::new(),
            by_activity: HashMap::new(),
            recent_turns: HashMap::new(),
        }
    }
    
    /// Get all active dialogs
    pub fn get_all(&self) -> Vec<&ActiveDialogSummary> {
        self.dialogs.values()
            .filter(|d| d.status == DialogStatus::Active)
            .collect()
    }
    
    /// Get dialogs for a participant
    pub fn get_by_participant(&self, participant_id: &str) -> Vec<&ActiveDialogSummary> {
        self.by_participant.get(participant_id)
            .map(|dialog_ids| {
                dialog_ids.iter()
                    .filter_map(|id| self.dialogs.get(id))
                    .filter(|d| d.status == DialogStatus::Active)
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get dialogs by type
    pub fn get_by_type(&self, dialog_type: &DialogType) -> Vec<&ActiveDialogSummary> {
        self.by_type.get(dialog_type)
            .map(|dialog_ids| {
                dialog_ids.iter()
                    .filter_map(|id| self.dialogs.get(id))
                    .filter(|d| d.status == DialogStatus::Active)
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get dialogs by activity level
    pub fn get_by_activity(&self, level: ActivityLevel) -> Vec<&ActiveDialogSummary> {
        self.by_activity.get(&level)
            .map(|dialog_ids| {
                dialog_ids.iter()
                    .filter_map(|id| self.dialogs.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Update activity level for a dialog
    fn update_activity_level(&mut self, dialog_id: Uuid) {
        if let Some(summary) = self.dialogs.get_mut(&dialog_id) {
            let recent_turns = self.recent_turns.get(&dialog_id)
                .map(|turns| {
                    let five_minutes_ago = Utc::now() - chrono::Duration::minutes(5);
                    turns.iter().filter(|t| **t > five_minutes_ago).count()
                })
                .unwrap_or(0);
            
            let old_level = summary.activity_level;
            let new_level = ActivityLevel::from_activity(summary.last_activity, recent_turns);
            
            if old_level != new_level {
                // Update activity index
                if let Some(old_set) = self.by_activity.get_mut(&old_level) {
                    old_set.remove(&dialog_id);
                }
                
                self.by_activity
                    .entry(new_level)
                    .or_insert_with(HashSet::new)
                    .insert(dialog_id);
                
                summary.activity_level = new_level;
            }
        }
    }
    
    /// Clean up old turn timestamps (keep only last 10 minutes)
    fn cleanup_turn_history(&mut self, dialog_id: &Uuid) {
        if let Some(turns) = self.recent_turns.get_mut(dialog_id) {
            let ten_minutes_ago = Utc::now() - chrono::Duration::minutes(10);
            turns.retain(|t| *t > ten_minutes_ago);
        }
    }
}

impl DialogProjection for ActiveDialogs {
    fn apply_event(&mut self, event: &DialogDomainEvent) {
        match event {
            DialogDomainEvent::Started(e) => {
                let mut active_participants = HashSet::new();
                for participant in &e.participants {
                    active_participants.insert(participant.id.clone());
                    
                    self.by_participant
                        .entry(participant.id.clone())
                        .or_insert_with(HashSet::new)
                        .insert(e.dialog_id);
                }
                
                let summary = ActiveDialogSummary {
                    dialog_id: e.dialog_id,
                    dialog_type: e.dialog_type.clone(),
                    status: DialogStatus::Active,
                    started_at: e.timestamp,
                    last_activity: e.timestamp,
                    participant_count: e.participants.len(),
                    active_participant_ids: active_participants,
                    turn_count: 0,
                    current_topic: None,
                    current_context: "default".to_string(),
                    activity_level: ActivityLevel::Low,
                };
                
                self.by_type
                    .entry(e.dialog_type.clone())
                    .or_insert_with(HashSet::new)
                    .insert(e.dialog_id);
                
                self.by_activity
                    .entry(ActivityLevel::Low)
                    .or_insert_with(HashSet::new)
                    .insert(e.dialog_id);
                
                self.dialogs.insert(e.dialog_id, summary);
                self.recent_turns.insert(e.dialog_id, vec![e.timestamp]);
            }
            
            DialogDomainEvent::TurnAdded(e) => {
                if let Some(summary) = self.dialogs.get_mut(&e.dialog_id) {
                    summary.turn_count += 1;
                    summary.last_activity = e.timestamp;
                    
                    if let Some(topic_id) = &e.turn.topic_id {
                        summary.current_topic = Some(topic_id.clone());
                    }
                    
                    // Track turn time
                    self.recent_turns
                        .entry(e.dialog_id)
                        .or_insert_with(Vec::new)
                        .push(e.timestamp);
                    
                    self.cleanup_turn_history(&e.dialog_id);
                    self.update_activity_level(e.dialog_id);
                }
            }
            
            DialogDomainEvent::ParticipantAdded(e) => {
                if let Some(summary) = self.dialogs.get_mut(&e.dialog_id) {
                    summary.participant_count += 1;
                    summary.active_participant_ids.insert(e.participant.id.clone());
                    summary.last_activity = e.timestamp;
                    
                    self.by_participant
                        .entry(e.participant.id.clone())
                        .or_insert_with(HashSet::new)
                        .insert(e.dialog_id);
                    
                    self.update_activity_level(e.dialog_id);
                }
            }
            
            DialogDomainEvent::ParticipantRemoved(e) => {
                if let Some(summary) = self.dialogs.get_mut(&e.dialog_id) {
                    summary.active_participant_ids.remove(&e.participant_id);
                    summary.last_activity = e.timestamp;
                    
                    if let Some(dialog_ids) = self.by_participant.get_mut(&e.participant_id) {
                        dialog_ids.remove(&e.dialog_id);
                    }
                    
                    self.update_activity_level(e.dialog_id);
                }
            }
            
            DialogDomainEvent::ContextSwitched(e) => {
                if let Some(summary) = self.dialogs.get_mut(&e.dialog_id) {
                    summary.current_context = e.new_context.context_id.clone();
                    summary.last_activity = e.timestamp;
                    self.update_activity_level(e.dialog_id);
                }
            }
            
            DialogDomainEvent::Paused(e) => {
                if let Some(summary) = self.dialogs.get_mut(&e.dialog_id) {
                    summary.status = DialogStatus::Paused;
                    summary.last_activity = e.timestamp;
                    
                    // Remove from activity tracking when paused
                    if let Some(level_set) = self.by_activity.get_mut(&summary.activity_level) {
                        level_set.remove(&e.dialog_id);
                    }
                }
            }
            
            DialogDomainEvent::Resumed(e) => {
                if let Some(summary) = self.dialogs.get_mut(&e.dialog_id) {
                    summary.status = DialogStatus::Active;
                    summary.last_activity = e.timestamp;
                    
                    // Re-add to activity tracking
                    self.by_activity
                        .entry(summary.activity_level)
                        .or_insert_with(HashSet::new)
                        .insert(e.dialog_id);
                    
                    self.update_activity_level(e.dialog_id);
                }
            }
            
            DialogDomainEvent::Ended(e) => {
                // Remove from all indices
                if let Some(summary) = self.dialogs.remove(&e.dialog_id) {
                    // Remove from participant index
                    for participant_id in &summary.active_participant_ids {
                        if let Some(dialog_ids) = self.by_participant.get_mut(participant_id) {
                            dialog_ids.remove(&e.dialog_id);
                        }
                    }
                    
                    // Remove from type index
                    if let Some(dialog_ids) = self.by_type.get_mut(&summary.dialog_type) {
                        dialog_ids.remove(&e.dialog_id);
                    }
                    
                    // Remove from activity index
                    if let Some(dialog_ids) = self.by_activity.get_mut(&summary.activity_level) {
                        dialog_ids.remove(&e.dialog_id);
                    }
                }
                
                // Clean up turn history
                self.recent_turns.remove(&e.dialog_id);
            }
            
            _ => {} // Other events don't affect active dialogs significantly
        }
    }
    
    fn id(&self) -> &str {
        "active_dialogs"
    }
}

/// Repository for active dialogs
#[async_trait]
pub trait ActiveDialogsRepository: Send + Sync {
    /// Get the active dialogs projection
    async fn get(&self) -> Result<ActiveDialogs, Box<dyn std::error::Error>>;
    
    /// Save the active dialogs projection
    async fn save(&self, active: ActiveDialogs) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Get activity statistics
    async fn get_statistics(&self) -> Result<ActivityStatistics, Box<dyn std::error::Error>>;
}

/// Activity statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityStatistics {
    pub total_active: usize,
    pub total_paused: usize,
    pub by_type: HashMap<DialogType, usize>,
    pub by_activity_level: HashMap<ActivityLevel, usize>,
    pub busiest_participants: Vec<(String, usize)>,
}

/// In-memory implementation
pub struct InMemoryActiveDialogsRepository {
    active: Arc<RwLock<ActiveDialogs>>,
}

impl InMemoryActiveDialogsRepository {
    pub fn new() -> Self {
        Self {
            active: Arc::new(RwLock::new(ActiveDialogs::new())),
        }
    }
}

#[async_trait]
impl ActiveDialogsRepository for InMemoryActiveDialogsRepository {
    async fn get(&self) -> Result<ActiveDialogs, Box<dyn std::error::Error>> {
        let active = self.active.read().await;
        Ok(active.clone())
    }
    
    async fn save(&self, active: ActiveDialogs) -> Result<(), Box<dyn std::error::Error>> {
        let mut stored = self.active.write().await;
        *stored = active;
        Ok(())
    }
    
    async fn get_statistics(&self) -> Result<ActivityStatistics, Box<dyn std::error::Error>> {
        let active = self.active.read().await;
        
        let total_active = active.dialogs.values()
            .filter(|d| d.status == DialogStatus::Active)
            .count();
        
        let total_paused = active.dialogs.values()
            .filter(|d| d.status == DialogStatus::Paused)
            .count();
        
        let mut by_type = HashMap::new();
        for (dialog_type, dialog_ids) in &active.by_type {
            let count = dialog_ids.iter()
                .filter(|id| active.dialogs.get(id)
                    .map(|d| d.status == DialogStatus::Active)
                    .unwrap_or(false))
                .count();
            by_type.insert(dialog_type.clone(), count);
        }
        
        let mut by_activity_level = HashMap::new();
        for (level, dialog_ids) in &active.by_activity {
            by_activity_level.insert(*level, dialog_ids.len());
        }
        
        let mut participant_counts: HashMap<String, usize> = HashMap::new();
        for (participant_id, dialog_ids) in &active.by_participant {
            let active_count = dialog_ids.iter()
                .filter(|id| active.dialogs.get(id)
                    .map(|d| d.status == DialogStatus::Active)
                    .unwrap_or(false))
                .count();
            if active_count > 0 {
                participant_counts.insert(participant_id.clone(), active_count);
            }
        }
        
        let mut busiest_participants: Vec<(String, usize)> = participant_counts.into_iter().collect();
        busiest_participants.sort_by(|a, b| b.1.cmp(&a.1));
        busiest_participants.truncate(10);
        
        Ok(ActivityStatistics {
            total_active,
            total_paused,
            by_type,
            by_activity_level,
            busiest_participants,
        })
    }
}