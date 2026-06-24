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



// ============================================================================
// SERVICE WORKER REGISTRATION (Registration Context)
// ============================================================================

#[derive(Clone)]
pub struct ServiceWorkerRegistration {
    pub id: String,         // unique ID (UUID)
    pub scope: String,      // e.g., "/app/" or "/"
    pub script_url: String, // URL to fetch SW script
    pub origin: String,     // Same-origin enforcement

    pub installing: Arc<Mutex<Option<Arc<ServiceWorkerInstance>>>>,
    pub installed: Arc<Mutex<Option<Arc<ServiceWorkerInstance>>>>, // waiting
    pub activating: Arc<Mutex<Option<Arc<ServiceWorkerInstance>>>>,
    pub active: Arc<Mutex<Option<Arc<ServiceWorkerInstance>>>>, // current controller

    pub update_via_cache: UpdateViaCache,
    pub last_update_check: Arc<Mutex<Instant>>,
    pub auto_update_interval: Duration,

    pub request_interceptors: Arc<Mutex<FetchInterceptorChain>>,
    pub background_sync_queue: BackgroundSyncQueue,
    pub periodic_sync_scheduler: PeriodicSyncScheduler,
}

impl ServiceWorkerRegistration {
    /// TODO: add docs
    pub fn new(
        scope: String,
        script_url: String,
        origin: String,
        db: Arc<ServiceWorkerDatabase>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            scope,
            script_url,
            origin,
            installing: Arc::new(Mutex::new(None)),
            installed: Arc::new(Mutex::new(None)),
            activating: Arc::new(Mutex::new(None)),
            active: Arc::new(Mutex::new(None)),
            update_via_cache: UpdateViaCache::Imports,
            last_update_check: Arc::new(Mutex::new(Instant::now())),
            auto_update_interval: Duration::from_secs(86400), // 24 hours
            request_interceptors: Arc::new(Mutex::new(FetchInterceptorChain::new())),
            background_sync_queue: BackgroundSyncQueue::new(db.clone()),
            periodic_sync_scheduler: PeriodicSyncScheduler::new(db),
        }
    }

    /// TODO: add docs
    pub fn set_installing(
        &self,
        instance: Option<Arc<ServiceWorkerInstance>>,
    ) -> std::result::Result<(), String> {
        let mut installing = self.installing.lock().map_err(|e| e.to_string())?;
        *installing = instance;
        Ok(())
    }

    /// TODO: add docs
    pub fn set_installed(
        &self,
        instance: Option<Arc<ServiceWorkerInstance>>,
    ) -> std::result::Result<(), String> {
        let mut installed = self.installed.lock().map_err(|e| e.to_string())?;
        *installed = instance;
        Ok(())
    }

    /// TODO: add docs
    pub fn set_activating(
        &self,
        instance: Arc<ServiceWorkerInstance>,
    ) -> std::result::Result<(), String> {
        let mut activating = self.activating.lock().map_err(|e| e.to_string())?;
        *activating = Some(instance);
        Ok(())
    }

    /// TODO: add docs
    pub fn set_active(
        &self,
        instance: Arc<ServiceWorkerInstance>,
    ) -> std::result::Result<(), String> {
        let mut active = self.active.lock().map_err(|e| e.to_string())?;
        *active = Some(instance);
        Ok(())
    }

    /// TODO: add docs
    pub fn get_active(&self) -> std::result::Result<Option<Arc<ServiceWorkerInstance>>, String> {
        let active = self.active.lock().map_err(|e| e.to_string())?;
        Ok(active.clone())
    }

    /// TODO: add docs
    pub fn get_installing(
        &self,
    ) -> std::result::Result<Option<Arc<ServiceWorkerInstance>>, String> {
        let installing = self.installing.lock().map_err(|e| e.to_string())?;
        Ok(installing.clone())
    }

    /// TODO: add docs
    pub fn get_installed(&self) -> std::result::Result<Option<Arc<ServiceWorkerInstance>>, String> {
        let installed = self.installed.lock().map_err(|e| e.to_string())?;
        Ok(installed.clone())
    }

    /// TODO: add docs
    pub fn get_activating(
        &self,
    ) -> std::result::Result<Option<Arc<ServiceWorkerInstance>>, String> {
        let activating = self.activating.lock().map_err(|e| e.to_string())?;
        Ok(activating.clone())
    }

    /// TODO: add docs
    pub fn scope_matches(&self, url: &str) -> bool {
        url.starts_with(&self.scope)
    }
}
