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
// SERVICE WORKER MANAGER (Global Registry)
// ============================================================================

pub struct ServiceWorkerManager {
    pub registrations: Arc<Mutex<HashMap<String, Arc<ServiceWorkerRegistration>>>>,
    pub db: Arc<ServiceWorkerDatabase>,
}

impl Clone for ServiceWorkerManager {
pub(crate) fn clone(&self) -> Self {
        Self {
            registrations: self.registrations.clone(),
            db: self.db.clone(),
        }
    }
}

impl ServiceWorkerManager {
    /// TODO: add docs
    pub fn new(db: Arc<ServiceWorkerDatabase>) -> Self {
        Self {
            registrations: Arc::new(Mutex::new(HashMap::new())),
            db,
        }
    }

    /// TODO: add docs
    pub fn load_from_db(&self) -> std::result::Result<(), String> {
        let saved = self
            .db
            .get_all_registrations()
            .map_err(|e| format!("DB Error: {}", e))?;

        let mut regs = self.registrations.lock().unwrap_or_else(|e| e.into_inner());
        for data in saved {
            let reg = Arc::new(ServiceWorkerRegistration::new(
                data.scope.clone(),
                data.script_url.clone(),
                data.origin.clone(),
                self.db.clone(),
            ));

            // Restore ID using unsafe as it's immutable field
            // SAFETY: The Arc is exclusively owned at this point (no other references exist).
            // The cast is needed to set the immutable id field during hydration from DB.
            let mut_reg = unsafe { &mut *(Arc::as_ptr(&reg) as *mut ServiceWorkerRegistration) };
            mut_reg.id = data.id.clone();

            let key = format!("{}#{}", data.origin, data.scope);
            regs.insert(key, reg);
        }
        tracing::info!(count = regs.len(), "Hydrated Service Worker registrations from DB");
        Ok(())
    }

    /// TODO: add docs
    pub fn register(&self, reg: Arc<ServiceWorkerRegistration>) -> std::result::Result<(), String> {
        let mut registrations = self.registrations.lock().map_err(|e| e.to_string())?;

        // Persist to DB
        let reg_data = SwRegistrationData {
            id: reg.id.clone(),
            scope: reg.scope.clone(),
            script_url: reg.script_url.clone(),
            origin: reg.origin.clone(),
            last_update_check: 0,
        };
        self.db
            .save_registration(&reg_data)
            .map_err(|e| e.to_string())?;

        let key = format!("{}#{}", reg.origin, reg.scope);
        registrations.insert(key, reg);
        Ok(())
    }

    /// TODO: add docs
    pub fn unregister(&self, origin: &str, scope: &str) -> std::result::Result<bool, String> {
        let mut registrations = self.registrations.lock().map_err(|e| e.to_string())?;

        let key = format!("{}#{}", origin, scope);
        if let Some(reg) = registrations.remove(&key) {
            self.db
                .delete_registration(&reg.id)
                .map_err(|e| e.to_string())?;
            return Ok(true);
        }
        Ok(false)
    }

    /// TODO: add docs
    pub fn dispatch_fetch_event(
        &self,
        instance: &ServiceWorkerInstance,
        request: RequestContext,
    ) -> JsResult<InterceptResult> {
        let (tx, rx) = tokio::sync::oneshot::channel::<InterceptResult>();

        if let Some(event_tx) = &*instance.event_tx.lock().unwrap_or_else(|e| e.into_inner()) {
            let _ = event_tx.send(SwEvent::Fetch(request, tx));

            // Wait for response with timeout
            match rx.blocking_recv() {
                Ok(res) => return Ok(res),
                Err(_) => return Ok(InterceptResult::PassThrough),
            }
        }

        Ok(InterceptResult::PassThrough)
    }

    /// TODO: add docs
    pub fn find_for_url(
        &self,
        origin: &str,
        url: &str,
    ) -> JsResult<Option<Arc<ServiceWorkerRegistration>>> {
        let registrations = self
            .registrations
            .lock()
            .map_err(|_| rquickjs::Error::Unknown)?;
        for reg in registrations.values() {
            if reg.origin == origin && reg.scope_matches(url) {
                return Ok(Some(reg.clone()));
            }
        }
        Ok(None)
    }

    /// TODO: add docs
    pub fn get_all_for_origin(
        &self,
        origin: &str,
    ) -> std::result::Result<Vec<Arc<ServiceWorkerRegistration>>, String> {
        let registrations = self.registrations.lock().map_err(|e| e.to_string())?;
        let mut results = Vec::new();
        for reg in registrations.values() {
            if reg.origin == origin {
                results.push(reg.clone());
            }
        }
        Ok(results)
    }

    /// TODO: add docs
    pub fn install_worker(
        &self,
        reg: Arc<ServiceWorkerRegistration>,
    ) -> std::result::Result<(), String> {
        let script_url = reg.script_url.clone();
        let origin = reg.origin.clone();

        // 1. Fetch Script (Blocking)
        let client = crate::network::client::FetchClient::new();
        let response = client
            .fetch(
                &script_url,
                None,
                crate::network::security::Origin::from_url(&origin),
            )
            .map_err(|e| format!("Failed to fetch SW script: {}", e))?;

        if !response.ok() {
            return Err(format!(
                "SW script fetch failed with status {}",
                response.status
            ));
        }

        let script_content = response.text();
        let instance = Arc::new(ServiceWorkerInstance::new(
            Uuid::new_v4().to_string(),
            script_url.clone(),
        ));

        reg.set_installing(Some(instance.clone()))?;
        instance.spawn(script_url, origin);

        if let Some(tx) = &*instance.event_tx.lock().unwrap_or_else(|e| e.into_inner()) {
            let _ = tx.send(SwEvent::Execute(script_content));
            let _ = tx.send(SwEvent::Dispatch("install".to_string()));
        }

        reg.set_installing(None)?;
        reg.set_installed(Some(instance))?;

        Ok(())
    }
}

impl Default for ServiceWorkerManager {
pub(crate) fn default() -> Self {
        let db = Arc::new(ServiceWorkerDatabase::new(std::path::PathBuf::from("sw.db")).expect("Albedo Engine: internal invariant violated"));
        Self::new(db)
    }
}
