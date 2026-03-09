use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use rquickjs::{Persistent, Function, Result as JsResult};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use crate::runtime::core::runtime::JsRuntime;
use crate::runtime::bindings::webapi::idb_service::worker::IDBWorkerCommand;

// ============================================================================
// ENUMS & BASIC TYPES
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub enum ServiceWorkerState {
    Installing,
    Installed,      // waiting state
    Activating,
    Activated,      // controllers are activated
    Redundant,
}

#[derive(Clone, Debug)]
pub enum UpdateViaCache {
    Imports,       // Only update imports
    All,           // Always check all URLs
    None,          // Never check (cache always)
}

#[derive(Clone, Debug)]
pub enum CacheMode {
    Default,
    NoStore,
    Reload,
    NoCache,
    ForceCache,
    OnlyIfCached,
}

#[derive(Clone, Debug)]
pub enum RedirectMode {
    Follow,
    Error,
    Manual,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CacheSource {
    Network,
    ServiceWorkerCache,
    IndexedDB,
}

// ============================================================================
// REQUEST/RESPONSE CONTEXT
// ============================================================================

#[derive(Clone, Debug)]
pub struct RequestContext {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub mode: String,           // cors, no-cors, same-origin, navigate
    pub credentials: String,    // omit, same-origin, include
    pub cache_mode: CacheMode,
    pub redirect: RedirectMode,
}

#[derive(Clone, Debug)]
pub struct ResponseContext {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub url: String,
    pub redirected: bool,
    pub cached_from: CacheSource,
}

// ============================================================================
// FETCH INTERCEPTOR CHAIN (Middleware Pattern)
// ============================================================================

#[derive(Clone, Debug)]
pub enum InterceptResult {
    Handled(ResponseContext),       // SW called respondWith()
    Modified(RequestContext),        // SW modified request
    PassThrough,                    // SW skipped
}

pub trait FetchInterceptor: Send + Sync {
    fn before_fetch(&self, req: &RequestContext) -> InterceptResult;
    fn after_fetch(&self, req: &RequestContext, res: &ResponseContext) -> ResponseContext;
}

pub struct FetchInterceptorChain {
    interceptors: Vec<Box<dyn FetchInterceptor>>,
}

impl FetchInterceptorChain {
    pub fn new() -> Self {
        Self {
            interceptors: Vec::new(),
        }
    }

    pub fn add_interceptor(&mut self, interceptor: Box<dyn FetchInterceptor>) {
        self.interceptors.push(interceptor);
    }

    pub fn clear(&mut self) {
        self.interceptors.clear();
    }

    pub fn execute(&self, req: RequestContext) -> Option<ResponseContext> {
        for interceptor in &self.interceptors {
            match interceptor.before_fetch(&req) {
                InterceptResult::Handled(res) => {
                    return Some(res);
                }
                InterceptResult::Modified(_modified_req) => {
                    // TODO: handle modified request
                    continue;
                }
                InterceptResult::PassThrough => {
                    continue;
                }
            }
        }
        None
    }
}

// ============================================================================
// BACKGROUND SYNC QUEUE
// ============================================================================

#[derive(Clone, Debug)]
pub struct SyncTask {
    pub id: String,
    pub tag: String,
    pub registration_id: String,
    pub created_at: Instant,
    pub retry_count: u32,
    pub last_retry: Option<Instant>,
    pub max_retries: u32,
}

pub struct BackgroundSyncQueue {
    pub queue: Arc<Mutex<Vec<SyncTask>>>,
    pub idb_worker: Arc<UnboundedSender<IDBWorkerCommand>>,
}

impl Clone for BackgroundSyncQueue {
    fn clone(&self) -> Self {
        Self {
            queue: self.queue.clone(),
            idb_worker: self.idb_worker.clone(),
        }
    }
}

impl BackgroundSyncQueue {
    pub fn new(idb_worker: Arc<UnboundedSender<IDBWorkerCommand>>) -> Self {
        Self {
            queue: Arc::new(Mutex::new(Vec::new())),
            idb_worker,
        }
    }

    pub fn register(&self, tag: String, reg_id: String) -> std::result::Result<(), String> {
        let task = SyncTask {
            id: Uuid::new_v4().to_string(),
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

    pub fn take_pending(&self) -> Vec<SyncTask> {
        let mut queue = self.queue.lock().unwrap();
        std::mem::take(&mut *queue)
    }

    pub fn reschedule(&self, task: SyncTask) -> std::result::Result<(), String> {
        let mut queue = self.queue.lock().map_err(|e| e.to_string())?;
        queue.push(task);
        Ok(())
    }
}

// ============================================================================
// PERIODIC SYNC SCHEDULER
// ============================================================================

#[derive(Clone, Debug)]
pub struct PeriodicSyncTask {
    pub id: String,
    pub tag: String,
    pub registration_id: String,
    pub min_interval_ms: u64,
    pub last_executed: Option<Instant>,
    pub enabled: bool,
}

pub struct PeriodicSyncScheduler {
    pub tasks: Arc<Mutex<HashMap<String, PeriodicSyncTask>>>,
    pub idb_worker: Arc<UnboundedSender<IDBWorkerCommand>>,
}

impl Clone for PeriodicSyncScheduler {
    fn clone(&self) -> Self {
        Self {
            tasks: self.tasks.clone(),
            idb_worker: self.idb_worker.clone(),
        }
    }
}

impl PeriodicSyncScheduler {
    pub fn new(idb_worker: Arc<UnboundedSender<IDBWorkerCommand>>) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            idb_worker,
        }
    }

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

    pub fn check_due(&self) -> Vec<PeriodicSyncTask> {
        let mut tasks = self.tasks.lock().unwrap();
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

    pub fn unregister(&self, tag: &str) -> std::result::Result<(), String> {
        let mut tasks = self.tasks.lock().map_err(|e| e.to_string())?;
        tasks.remove(tag);
        Ok(())
    }
}

// ============================================================================
// SERVICE WORKER INSTANCE (Isolated Runtime + State Machine)
// ============================================================================

#[derive(Clone)]
pub struct ServiceWorkerInstance {
    pub id: usize,          // JsRuntime ID
    pub state: Arc<Mutex<ServiceWorkerState>>,
    pub rt: Arc<Mutex<JsRuntime>>,

    pub fetch_handler: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
    pub install_handler: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
    pub activate_handler: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
    pub message_handler: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
    pub sync_handler: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
    pub periodic_sync_handler: Arc<Mutex<Option<Persistent<Function<'static>>>>>,

    pub pending_clients: Arc<Mutex<Vec<ClientInfo>>>,
}

#[derive(Clone, Debug)]
pub struct ClientInfo {
    pub id: String,
    pub url: String,
    pub frame_type: String,  // "top-level", "nested", "iframe", "worker"
    pub focused: bool,
}

impl ServiceWorkerInstance {
    pub fn new(rt: JsRuntime) -> Self {
        let id = rt.id;
        Self {
            id,
            state: Arc::new(Mutex::new(ServiceWorkerState::Installing)),
            rt: Arc::new(Mutex::new(rt)),
            fetch_handler: Arc::new(Mutex::new(None)),
            install_handler: Arc::new(Mutex::new(None)),
            activate_handler: Arc::new(Mutex::new(None)),
            message_handler: Arc::new(Mutex::new(None)),
            sync_handler: Arc::new(Mutex::new(None)),
            periodic_sync_handler: Arc::new(Mutex::new(None)),
            pending_clients: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn set_state(&self, state: ServiceWorkerState) -> std::result::Result<(), String> {
        let mut current_state = self
            .state
            .lock()
            .map_err(|e| format!("Failed to acquire state lock: {}", e))?;
        *current_state = state;
        Ok(())
    }

    pub fn get_state(&self) -> std::result::Result<ServiceWorkerState, String> {
        let state = self
            .state
            .lock()
            .map_err(|e| format!("Failed to acquire state lock: {}", e))?;
        Ok(state.clone())
    }

    pub fn register_fetch_handler(
        &self,
        handler: Persistent<Function<'static>>,
    ) -> std::result::Result<(), String> {
        let mut h = self
            .fetch_handler
            .lock()
            .map_err(|e| format!("Failed to lock fetch_handler: {}", e))?;
        *h = Some(handler);
        Ok(())
    }

    pub fn register_install_handler(
        &self,
        handler: Persistent<Function<'static>>,
    ) -> std::result::Result<(), String> {
        let mut h = self
            .install_handler
            .lock()
            .map_err(|e| format!("Failed to lock install_handler: {}", e))?;
        *h = Some(handler);
        Ok(())
    }

    pub fn register_activate_handler(
        &self,
        handler: Persistent<Function<'static>>,
    ) -> std::result::Result<(), String> {
        let mut h = self
            .activate_handler
            .lock()
            .map_err(|e| format!("Failed to lock activate_handler: {}", e))?;
        *h = Some(handler);
        Ok(())
    }

    pub fn register_sync_handler(
        &self,
        handler: Persistent<Function<'static>>,
    ) -> std::result::Result<(), String> {
        let mut h = self
            .sync_handler
            .lock()
            .map_err(|e| format!("Failed to lock sync_handler: {}", e))?;
        *h = Some(handler);
        Ok(())
    }

    pub fn register_periodic_sync_handler(
        &self,
        handler: Persistent<Function<'static>>,
    ) -> std::result::Result<(), String> {
        let mut h = self
            .periodic_sync_handler
            .lock()
            .map_err(|e| format!("Failed to lock periodic_sync_handler: {}", e))?;
        *h = Some(handler);
        Ok(())
    }

    pub fn add_client(&self, client: ClientInfo) -> std::result::Result<(), String> {
        let mut clients = self
            .pending_clients
            .lock()
            .map_err(|e| format!("Failed to lock clients: {}", e))?;
        clients.push(client);
        Ok(())
    }

    pub fn get_clients(&self) -> std::result::Result<Vec<ClientInfo>, String> {
        let clients = self
            .pending_clients
            .lock()
            .map_err(|e| format!("Failed to lock clients: {}", e))?;
        Ok(clients.clone())
    }
}

// ============================================================================
// SERVICE WORKER REGISTRATION (Registration Context)
// ============================================================================

#[derive(Clone)]
pub struct ServiceWorkerRegistration {
    pub id: String,                 // unique ID (UUID)
    pub scope: String,              // e.g., "/app/" or "/"
    pub script_url: String,         // URL to fetch SW script
    pub origin: String,             // Same-origin enforcement

    pub installing: Arc<Mutex<Option<Arc<ServiceWorkerInstance>>>>,
    pub installed: Arc<Mutex<Option<Arc<ServiceWorkerInstance>>>>,   // waiting
    pub activating: Arc<Mutex<Option<Arc<ServiceWorkerInstance>>>>,
    pub active: Arc<Mutex<Option<Arc<ServiceWorkerInstance>>>>,      // current controller

    pub update_via_cache: UpdateViaCache,
    pub last_update_check: Arc<Mutex<Instant>>,
    pub auto_update_interval: Duration,

    pub request_interceptors: Arc<Mutex<FetchInterceptorChain>>,
    pub background_sync_queue: BackgroundSyncQueue,
    pub periodic_sync_scheduler: PeriodicSyncScheduler,
}

impl ServiceWorkerRegistration {
    pub fn new(
        scope: String,
        script_url: String,
        origin: String,
        idb_worker: Arc<UnboundedSender<IDBWorkerCommand>>,
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
            background_sync_queue: BackgroundSyncQueue::new(idb_worker.clone()),
            periodic_sync_scheduler: PeriodicSyncScheduler::new(idb_worker),
        }
    }

    pub fn set_installing(
        &self,
        instance: Arc<ServiceWorkerInstance>,
    ) -> std::result::Result<(), String> {
        let mut installing = self
            .installing
            .lock()
            .map_err(|e| format!("Failed to lock installing: {}", e))?;
        *installing = Some(instance);
        Ok(())
    }

    pub fn set_installed(
        &self,
        instance: Arc<ServiceWorkerInstance>,
    ) -> std::result::Result<(), String> {
        let mut installed = self
            .installed
            .lock()
            .map_err(|e| format!("Failed to lock installed: {}", e))?;
        *installed = Some(instance);
        Ok(())
    }

    pub fn set_activating(
        &self,
        instance: Arc<ServiceWorkerInstance>,
    ) -> std::result::Result<(), String> {
        let mut activating = self
            .activating
            .lock()
            .map_err(|e| format!("Failed to lock activating: {}", e))?;
        *activating = Some(instance);
        Ok(())
    }

    pub fn set_active(
        &self,
        instance: Arc<ServiceWorkerInstance>,
    ) -> std::result::Result<(), String> {
        let mut active = self
            .active
            .lock()
            .map_err(|e| format!("Failed to lock active: {}", e))?;
        *active = Some(instance);
        Ok(())
    }

    pub fn get_active(&self) -> std::result::Result<Option<Arc<ServiceWorkerInstance>>, String> {
        let active = self
            .active
            .lock()
            .map_err(|e| format!("Failed to lock active: {}", e))?;
        Ok(active.clone())
    }

    pub fn get_installing(
        &self,
    ) -> std::result::Result<Option<Arc<ServiceWorkerInstance>>, String> {
        let installing = self
            .installing
            .lock()
            .map_err(|e| format!("Failed to lock installing: {}", e))?;
        Ok(installing.clone())
    }

    pub fn scope_matches(&self, url: &str) -> bool {
        // Simple scope matching: "/app/" matches "/app/page", but not "/application"
        url.starts_with(&self.scope)
    }

    pub fn should_update_check(&self) -> std::result::Result<bool, String> {
        let last_check = self
            .last_update_check
            .lock()
            .map_err(|e| format!("Failed to lock last_update_check: {}", e))?;

        let elapsed = Instant::now().duration_since(*last_check);
        Ok(elapsed >= self.auto_update_interval)
    }

    pub fn mark_update_checked(&self) -> std::result::Result<(), String> {
        let mut last_check = self
            .last_update_check
            .lock()
            .map_err(|e| format!("Failed to lock last_update_check: {}", e))?;
        *last_check = Instant::now();
        Ok(())
    }
}

// ============================================================================
// SERVICE WORKER MANAGER (Global Registry)
// ============================================================================

pub struct ServiceWorkerManager {
    pub registrations: Arc<Mutex<HashMap<String, Arc<ServiceWorkerRegistration>>>>,
}

impl Clone for ServiceWorkerManager {
    fn clone(&self) -> Self {
        Self {
            registrations: self.registrations.clone(),
        }
    }
}

impl ServiceWorkerManager {
    pub fn new() -> Self {
        Self {
            registrations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register(
        &self,
        reg: Arc<ServiceWorkerRegistration>,
    ) -> std::result::Result<(), String> {
        let mut registrations = self
            .registrations
            .lock()
            .map_err(|e| format!("Failed to lock registrations: {}", e))?;

        let key = format!("{}#{}", reg.origin, reg.scope);
        registrations.insert(key, reg);
        Ok(())
    }

    pub fn unregister(&self, origin: &str, scope: &str) -> std::result::Result<bool, String> {
        let mut registrations = self
            .registrations
            .lock()
            .map_err(|e| format!("Failed to lock registrations: {}", e))?;

        let key = format!("{}#{}", origin, scope);
        Ok(registrations.remove(&key).is_some())
    }

    pub fn find_for_url(
        &self,
        origin: &str,
        url: &str,
    ) -> std::result::Result<Option<Arc<ServiceWorkerRegistration>>, String> {
        let registrations = self
            .registrations
            .lock()
            .map_err(|e| format!("Failed to lock registrations: {}", e))?;

        for (key, reg) in registrations.iter() {
            if key.starts_with(origin) && reg.scope_matches(url) {
                return Ok(Some(reg.clone()));
            }
        }
        Ok(None)
    }

    pub fn get_all_for_origin(
        &self,
        origin: &str,
    ) -> std::result::Result<Vec<Arc<ServiceWorkerRegistration>>, String> {
        let registrations = self
            .registrations
            .lock()
            .map_err(|e| format!("Failed to lock registrations: {}", e))?;

        let result: Vec<_> = registrations
            .iter()
            .filter(|(key, _)| key.starts_with(origin))
            .map(|(_, reg)| reg.clone())
            .collect();

        Ok(result)
    }

    pub fn clear_all(&self) -> std::result::Result<(), String> {
        let mut registrations = self
            .registrations
            .lock()
            .map_err(|e| format!("Failed to lock registrations: {}", e))?;
        registrations.clear();
        Ok(())
    }
}

impl Default for ServiceWorkerManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

pub fn parse_scope_from_options(options: &HashMap<String, String>) -> String {
    options
        .get("scope")
        .cloned()
        .unwrap_or_else(|| "/".to_string())
}

pub fn calculate_scope_from_script_url(script_url: &str) -> String {
    // Extract directory from script URL
    // e.g., "/assets/sw.js" -> "/assets/"
    if let Some(last_slash) = script_url.rfind('/') {
        script_url[..=last_slash].to_string()
    } else {
        "/".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_matching() {
        let reg = ServiceWorkerRegistration::new(
            "/app/".to_string(),
            "http://example.com/app/sw.js".to_string(),
            "http://example.com".to_string(),
            Arc::new(|_| {}), // dummy
        );

        assert!(reg.scope_matches("/app/page"));
        assert!(reg.scope_matches("/app/nested/page"));
        assert!(!reg.scope_matches("/application"));
    }

    #[test]
    fn test_sync_task_creation() {
        let (tx, _) = std::sync::mpsc::channel();
        let idb = Arc::new(move |_| {
            let _ = tx.send(());
        });

        let queue = BackgroundSyncQueue::new(Arc::new(idb));
        queue
            .register("upload".to_string(), "reg-1".to_string())
            .unwrap();

        let pending = queue.take_pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].tag, "upload");
    }

    #[test]
    fn test_periodic_sync_due_check() {
        let (tx, _) = std::sync::mpsc::channel();
        let idb = Arc::new(move |_| {
            let _ = tx.send(());
        });

        let scheduler = PeriodicSyncScheduler::new(Arc::new(idb));
        scheduler
            .register("cleanup".to_string(), 100, "reg-1".to_string())
            .unwrap();

        // Should be due initially (last_executed = None)
        let due = scheduler.check_due();
        assert_eq!(due.len(), 1);

        // Should not be due immediately after
        let due2 = scheduler.check_due();
        assert!(due2.is_empty());
    }

    #[test]
    fn test_sw_state_transitions() {
        let rt = unsafe {
            // This is unsafe in tests, would need proper mock
            JsRuntime::new().unwrap()
        };

        let sw = ServiceWorkerInstance::new(rt);

        assert_eq!(
            sw.get_state().unwrap(),
            ServiceWorkerState::Installing
        );

        sw.set_state(ServiceWorkerState::Installed)
            .unwrap();
        assert_eq!(
            sw.get_state().unwrap(),
            ServiceWorkerState::Installed
        );
    }
}
