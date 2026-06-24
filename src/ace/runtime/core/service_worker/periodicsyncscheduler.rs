use super::*;
use rquickjs::Result as JsResult;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crate::utils::uuid::Uuid;

use crate::ace::runtime::core::runtime::JsRuntime;
use crate::ace::runtime::core::sw_db::{ServiceWorkerDatabase, SwRegistrationData, SwSyncTaskData};

// ============================================================================
// ENUMS & BASIC TYPES
// ============================================================================



pub struct PeriodicSyncScheduler {
    pub tasks: Arc<Mutex<HashMap<String, PeriodicSyncTask>>>,
    pub db: Arc<ServiceWorkerDatabase>,
}

impl Clone for PeriodicSyncScheduler {
pub(crate) fn clone(&self) -> Self {
        Self {
            tasks: self.tasks.clone(),
            db: self.db.clone(),
        }
    }
}

impl PeriodicSyncScheduler {
    /// TODO: add docs
    pub fn new(db: Arc<ServiceWorkerDatabase>) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            db,
        }
    }

    /// TODO: add docs
    pub fn register(
        &self,
        tag: String,
        min_interval_ms: u64,
        reg_id: String,
    ) -> std::result::Result<(), String> {
        let task = PeriodicSyncTask {
            id: Uuid::new_v4().to_string(),
            tag: tag.clone(),
            registration_id: reg_id,
            min_interval_ms,
            last_executed: None,
            enabled: true,
        };

        let mut tasks = self.tasks.lock().map_err(|e| e.to_string())?;
        tasks.insert(tag, task);
        Ok(())
    }

    /// TODO: add docs
    pub fn check_due(&self) -> Vec<PeriodicSyncTask> {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let mut due = Vec::new();
        let now = Instant::now();

        for task in tasks.values_mut() {
            if !task.enabled {
                continue;
            }

            let should_execute = match task.last_executed {
                None => true,
                Some(last) => {
                    let elapsed = now.duration_since(last);
                    elapsed.as_millis() >= task.min_interval_ms as u128
                }
            };

            if should_execute {
                task.last_executed = Some(now);
                due.push(task.clone());
            }
        }

        due
    }

    /// TODO: add docs
    pub fn unregister(&self, tag: &str) -> std::result::Result<(), String> {
        let mut tasks = self.tasks.lock().map_err(|e| e.to_string())?;
        tasks.remove(tag);
        Ok(())
    }
}
