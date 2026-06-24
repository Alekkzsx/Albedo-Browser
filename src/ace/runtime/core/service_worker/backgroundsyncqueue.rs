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



pub struct BackgroundSyncQueue {
    pub queue: Arc<Mutex<Vec<SyncTask>>>,
    pub db: Arc<ServiceWorkerDatabase>,
}

impl Clone for BackgroundSyncQueue {
pub(crate) fn clone(&self) -> Self {
        Self {
            queue: self.queue.clone(),
            db: self.db.clone(),
        }
    }
}

impl BackgroundSyncQueue {
    /// TODO: add docs
    pub fn new(db: Arc<ServiceWorkerDatabase>) -> Self {
        Self {
            queue: Arc::new(Mutex::new(Vec::new())),
            db,
        }
    }

    /// TODO: add docs
    pub fn register(&self, tag: String, reg_id: String) -> std::result::Result<(), String> {
        let task_data = SwSyncTaskData {
            id: Uuid::new_v4().to_string(),
            tag: tag.clone(),
            registration_id: reg_id.clone(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Albedo Engine: internal invariant violated")
                .as_secs(),
            retry_count: 0,
        };

        // Persist to DB
        self.db
            .save_sync_task(&task_data)
            .map_err(|e| e.to_string())?;

        let task = SyncTask {
            id: task_data.id,
            tag,
            registration_id: reg_id,
            created_at: Instant::now(),
            retry_count: 0,
            last_retry: None,
            max_retries: 3,
        };

        let mut queue = self.queue.lock().map_err(|e| e.to_string())?;
        queue.push(task);
        Ok(())
    }

    /// TODO: add docs
    pub fn take_pending(&self) -> Vec<SyncTask> {
        let mut queue = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *queue)
    }

    /// TODO: add docs
    pub fn reschedule(&self, task: SyncTask) -> std::result::Result<(), String> {
        let mut queue = self.queue.lock().map_err(|e| e.to_string())?;
        queue.push(task);
        Ok(())
    }
}
