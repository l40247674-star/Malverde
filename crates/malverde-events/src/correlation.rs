use crate::event::{Event, EventType, EventId};
use malverde_core::{ProjectId, OperationId, ActorId};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// Event correlator for tracking event relationships
#[derive(Debug, Clone)]
pub struct EventCorrelator {
    events: Arc<parking_lot::RwLock<HashMap<EventId, Event>>>,
    parent_to_children: Arc<parking_lot::RwLock<HashMap<EventId, Vec<EventId>>>>,
    child_to_parent: Arc<parking_lot::RwLock<HashMap<EventId, EventId>>>,
    project_to_events: Arc<parking_lot::RwLock<HashMap<ProjectId, Vec<EventId>>>>,
    operation_to_events: Arc<parking_lot::RwLock<HashMap<OperationId, Vec<EventId>>>>,
    actor_to_events: Arc<parking_lot::RwLock<HashMap<ActorId, Vec<EventId>>>>,
}

impl EventCorrelator {
    pub fn new() -> Self {
        Self {
            events: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            parent_to_children: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            child_to_parent: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            project_to_events: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            operation_to_events: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            actor_to_events: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    pub fn add_event(&self, event: Event) {
        let event_id = event.id.clone();
        self.events.write().insert(event_id.clone(), event.clone());
        
        if let Some(parent_id) = &event.parent_id {
            self.parent_to_children.write().entry(parent_id.clone()).or_default().push(event_id.clone());
            self.child_to_parent.write().insert(event_id.clone(), parent_id.clone());
        }
        
        if let Some(project_id) = &event.project_id {
            self.project_to_events.write().entry(project_id.clone()).or_default().push(event_id.clone());
        }
        
        if let Some(operation_id) = &event.operation_id {
            self.operation_to_events.write().entry(operation_id.clone()).or_default().push(event_id.clone());
        }
        
        if let Some(actor_id) = &event.actor_id {
            self.actor_to_events.write().entry(actor_id.clone()).or_default().push(event_id.clone());
        }
    }

    pub fn get_event(&self, event_id: &EventId) -> Option<Event> {
        self.events.read().get(event_id).cloned()
    }

    pub fn get_children(&self, parent_id: &EventId) -> Vec<Event> {
        self.parent_to_children.read()
            .get(parent_id)
            .map(|children| {
                children.iter()
                    .filter_map(|child_id| self.get_event(child_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_parent(&self, child_id: &EventId) -> Option<Event> {
        self.child_to_parent.read()
            .get(child_id)
            .and_then(|parent_id| self.get_event(parent_id))
    }

    pub fn get_events_by_project(&self, project_id: &ProjectId) -> Vec<Event> {
        self.project_to_events.read()
            .get(project_id)
            .map(|event_ids| {
                event_ids.iter()
                    .filter_map(|id| self.get_event(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_events_by_operation(&self, operation_id: &OperationId) -> Vec<Event> {
        self.operation_to_events.read()
            .get(operation_id)
            .map(|event_ids| {
                event_ids.iter()
                    .filter_map(|id| self.get_event(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_events_by_actor(&self, actor_id: &ActorId) -> Vec<Event> {
        self.actor_to_events.read()
            .get(actor_id)
            .map(|event_ids| {
                event_ids.iter()
                    .filter_map(|id| self.get_event(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_chain(&self, event_id: &EventId) -> Vec<Event> {
        let mut chain = Vec::new();
        let mut current = Some(event_id.clone());
        
        let mut ancestors = Vec::new();
        while let Some(id) = current {
            if let Some(event) = self.get_event(&id) {
                ancestors.push(event);
                current = self.child_to_parent.read().get(&id).cloned();
            } else {
                break;
            }
        }
        ancestors.reverse();
        chain.extend(ancestors);
        
        if !chain.iter().any(|e| e.id == *event_id) {
            if let Some(event) = self.get_event(event_id) {
                chain.push(event);
            }
        }
        
        let mut queue = VecDeque::new();
        queue.push_back(event_id.clone());
        let mut visited = HashSet::new();
        visited.insert(event_id.clone());
        
        while let Some(id) = queue.pop_front() {
            let children = self.parent_to_children.read()
                .get(&id)
                .cloned()
                .unwrap_or_default();
            
            for child_id in children {
                if !visited.contains(&child_id) {
                    visited.insert(child_id.clone());
                    if let Some(event) = self.get_event(&child_id) {
                        chain.push(event);
                    }
                    queue.push_back(child_id);
                }
            }
        }
        
        chain
    }

    pub fn are_related(&self, event1: &EventId, event2: &EventId) -> bool {
        let chain1 = self.get_chain(event1);
        chain1.iter().any(|e| &e.id == event2)
    }

    pub fn get_root(&self, event_id: &EventId) -> Option<Event> {
        let mut current = event_id.clone();
        while let Some(parent_id) = self.child_to_parent.read().get(&current).cloned() {
            current = parent_id;
        }
        self.get_event(&current)
    }

    pub fn get_root_events(&self) -> Vec<Event> {
        self.events.read()
            .values()
            .filter(|event| event.parent_id.is_none())
            .cloned()
            .collect()
    }

    pub fn get_leaf_events(&self) -> Vec<Event> {
        let events = self.events.read();
        let has_children = |event_id: &EventId| {
            self.parent_to_children.read()
                .get(event_id)
                .map(|children| !children.is_empty())
                .unwrap_or(false)
        };
        events.values().filter(|event| !has_children(&event.id)).cloned().collect()
    }

    pub fn correlate_by_time(&self, event: &Event, window: std::time::Duration) -> Vec<Event> {
        let events = self.events.read();
        let target_time = event.timestamp;
        events.values()
            .filter(|e| {
                let diff = if e.timestamp > target_time {
                    e.timestamp - target_time
                } else {
                    target_time - e.timestamp
                };
                diff <= window
            })
            .cloned()
            .collect()
    }

    pub fn correlate_by_type(&self, event: &Event, types: &[EventType]) -> Vec<Event> {
        let events = self.events.read();
        events.values().filter(|e| types.contains(&e.event_type)).cloned().collect()
    }

    pub fn correlate_by_metadata(&self, event: &Event, key: &str, value: &str) -> Vec<Event> {
        let events = self.events.read();
        events.values()
            .filter(|e| e.metadata.get(key).map(|v| v == value).unwrap_or(false))
            .cloned()
            .collect()
    }

    pub fn clear(&self) {
        self.events.write().clear();
        self.parent_to_children.write().clear();
        self.child_to_parent.write().clear();
        self.project_to_events.write().clear();
        self.operation_to_events.write().clear();
        self.actor_to_events.write().clear();
    }

    pub fn stats(&self) -> CorrelatorStats {
        let events = self.events.read();
        let parent_to_children = self.parent_to_children.read();
        let project_to_events = self.project_to_events.read();
        let operation_to_events = self.operation_to_events.read();
        let actor_to_events = self.actor_to_events.read();
        
        CorrelatorStats {
            total_events: events.len(),
            total_projects: project_to_events.len(),
            total_operations: operation_to_events.len(),
            total_actors: actor_to_events.len(),
            total_parent_child_relationships: parent_to_children.iter().map(|(_, children)| children.len()).sum(),
            root_events: self.get_root_events().len(),
            leaf_events: self.get_leaf_events().len(),
        }
    }
}

/// Correlator statistics
#[derive(Debug, Clone)]
pub struct CorrelatorStats {
    pub total_events: usize,
    pub total_projects: usize,
    pub total_operations: usize,
    pub total_actors: usize,
    pub total_parent_child_relationships: usize,
    pub root_events: usize,
    pub leaf_events: usize,
}
