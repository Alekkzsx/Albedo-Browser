use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::engine::dom::AceDOM;

#[derive(Clone, Debug)]
pub struct IntersectionObserverEntry {
    pub time: f64,
    pub root_bounds: Option<(f32, f32, f32, f32)>, // x, y, w, h
    pub bounding_client_rect: (f32, f32, f32, f32),
    pub intersection_rect: (f32, f32, f32, f32),
    pub is_intersecting: bool,
    pub intersection_ratio: f32,
    pub target: usize,
}

#[derive(Clone, Debug)]
pub struct TargetState {
    pub last_intersection_ratio: f32,
    pub last_is_intersecting: bool,
    // -1.0 means uninitialized
}

impl Default for TargetState {
    fn default() -> Self {
        Self {
            last_intersection_ratio: -1.0, 
            last_is_intersecting: false,
        }
    }
}

#[derive(Clone)]
pub struct IntersectionObserverData {
    pub id: usize,
    pub root: Option<usize>, // None for viewport
    pub root_margin: String, // TODO: Parse into pixels/offsets
    pub thresholds: Vec<f32>,
    pub callback_id: usize, // ID mapping to the persistent JS callback
    pub target_states: HashMap<usize, TargetState>,
}

pub struct SharedIntersectionRegistry {
    // Map observer_id -> Data
    pub observers: HashMap<usize, IntersectionObserverData>,
    // Map observer_id -> List of target node indices
    pub targets: HashMap<usize, Vec<usize>>,
    // Queue of pending notifications: Map callback_id -> Vec<Entry>
    pub pending_notifications: HashMap<usize, Vec<IntersectionObserverEntry>>,
}

impl SharedIntersectionRegistry {
    pub fn new() -> Self {
        Self {
            observers: HashMap::new(),
            targets: HashMap::new(),
            pending_notifications: HashMap::new(),
        }
    }

    pub fn register_observer(&mut self, id: usize, mut data: IntersectionObserverData) {
        data.target_states = HashMap::new();
        self.observers.insert(id, data);
        self.targets.insert(id, Vec::new());
    }

    pub fn observe(&mut self, observer_id: usize, target_node: usize) {
        if let Some(list) = self.targets.get_mut(&observer_id) {
            if !list.contains(&target_node) {
                list.push(target_node);
                // Initialize state
                if let Some(data) = self.observers.get_mut(&observer_id) {
                    data.target_states.insert(target_node, TargetState::default());
                }
            }
        }
    }

    pub fn unobserve(&mut self, observer_id: usize, target_node: usize) {
        if let Some(list) = self.targets.get_mut(&observer_id) {
            list.retain(|&x| x != target_node);
        }
        if let Some(data) = self.observers.get_mut(&observer_id) {
             data.target_states.remove(&target_node);
        }
    }

    pub fn disconnect(&mut self, observer_id: usize) {
        self.targets.remove(&observer_id);
        // Note: We keep the observer data in 'observers' map mostly, 
        // effectively 'disconnect' clears targets.
        // Spec says disconnect clears all targets.
        self.targets.insert(observer_id, Vec::new());
    }
    
    pub fn queue_notification(&mut self, callback_id: usize, entry: IntersectionObserverEntry) {
        self.pending_notifications.entry(callback_id).or_insert_with(Vec::new).push(entry);
    }
    
    pub fn queue_notifications(&mut self, notifications: HashMap<usize, Vec<IntersectionObserverEntry>>) {
        for (callback_id, entries) in notifications {
            for entry in entries {
                self.queue_notification(callback_id, entry);
            }
        }
    }
    
    pub fn take_pending(&mut self) -> HashMap<usize, Vec<IntersectionObserverEntry>> {
        std::mem::take(&mut self.pending_notifications)
    }
}
