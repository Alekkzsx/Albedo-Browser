use rquickjs::Result as JsResult;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crate::ace::util::uuid::Uuid;

use crate::runtime::core::runtime::JsRuntime;
use crate::runtime::core::sw_db::{ServiceWorkerDatabase, SwRegistrationData, SwSyncTaskData};

// ============================================================================
// ENUMS & BASIC TYPES
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub enum ServiceWorkerState {
    Installing,
    Installed, // waiting state
    Activating,
    Activated, // controllers are activated
    Redundant,
}

#[derive(Clone, Debug)]
pub enum UpdateViaCache {
    Imports, // Only update imports
    All,     // Always check all URLs
    None,    // Never check (cache always)
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
    pub mode: String,        // cors, no-cors, same-origin, navigate
    pub credentials: String, // omit, same-origin, include
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
    Handled(ResponseContext), // SW called respondWith()
    Modified(RequestContext), // SW modified request
    PassThrough,              // SW skipped
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
    pub db: Arc<ServiceWorkerDatabase>,
}

impl Clone for BackgroundSyncQueue {
    fn clone(&self) -> Self {
        Self {
            queue: self.queue.clone(),
            db: self.db.clone(),
        }
    }
}

impl BackgroundSyncQueue {
    pub fn new(db: Arc<ServiceWorkerDatabase>) -> Self {
        Self {
            queue: Arc::new(Mutex::new(Vec::new())),
            db,
        }
    }

    pub fn register(&self, tag: String, reg_id: String) -> std::result::Result<(), String> {
        let task_data = SwSyncTaskData {
            id: Uuid::new_v4().to_string(),
            tag: tag.clone(),
            registration_id: reg_id.clone(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
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
    pub db: Arc<ServiceWorkerDatabase>,
}

impl Clone for PeriodicSyncScheduler {
    fn clone(&self) -> Self {
        Self {
            tasks: self.tasks.clone(),
            db: self.db.clone(),
        }
    }
}

impl PeriodicSyncScheduler {
    pub fn new(db: Arc<ServiceWorkerDatabase>) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            db,
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
    pub id: String,
    pub script_url: String,
    pub state: Arc<Mutex<ServiceWorkerState>>,
    pub pending_clients: Arc<Mutex<Vec<ClientInfo>>>,
    pub event_tx: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<SwEvent>>>>,
    pub thread_handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
    pub is_running: Arc<std::sync::atomic::AtomicBool>,
    pub js_runtime: Arc<Mutex<Option<JsRuntime>>>,
    pub pending_tasks: Arc<Mutex<Vec<tokio::sync::oneshot::Receiver<()>>>>, // Tracking waitUntil()
}

pub enum SwEvent {
    Execute(String),
    Dispatch(String),
    Fetch(
        RequestContext,
        tokio::sync::oneshot::Sender<InterceptResult>,
    ),
    Terminate,
}

#[derive(Clone, Debug)]
pub struct ClientInfo {
    pub id: String,
    pub url: String,
    pub frame_type: String, // "top-level", "nested", "iframe", "worker"
    pub focused: bool,
}

impl ServiceWorkerInstance {
    pub fn new(id: String, script_url: String) -> Self {
        Self {
            id,
            script_url,
            state: Arc::new(Mutex::new(ServiceWorkerState::Installing)),
            pending_clients: Arc::new(Mutex::new(Vec::new())),
            event_tx: Arc::new(Mutex::new(None)),
            thread_handle: Arc::new(Mutex::new(None)),
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            js_runtime: Arc::new(Mutex::new(None)),
            pending_tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn spawn(&self, script_url: String, origin: String) {
        if self.is_running.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<SwEvent>();
        *self.event_tx.lock().unwrap() = Some(tx);

        let is_running = self.is_running.clone();
        is_running.store(true, std::sync::atomic::Ordering::SeqCst);

        let handle = std::thread::spawn(move || {
            println!("[SW] Thread started for Worker: {}", script_url);
            // Initialize runtime inside the thread
            let rt = crate::runtime::core::init::init_sw_runtime(&script_url, &origin)
                .unwrap_or_else(|| {
                    panic!("Failed to init SW runtime for {}", script_url);
                });

            while let Some(event) = rx.blocking_recv() {
                match event {
                    SwEvent::Execute(script) => {
                        let _ = rt.execute_script(&script);
                    }
                    SwEvent::Dispatch(evt_type) => {
                        rt.with_context(|ctx| {
                            ctx.with(|ctx| {
                                let script =
                                    format!("globalThis.dispatchEvent(new Event('{}'))", evt_type);
                                let _ = ctx.eval::<(), _>(script);
                            });
                        });
                    }
                    SwEvent::Fetch(_req, resp_tx) => {
                        rt.with_context(|ctx| {
                            ctx.with(|_ctx| {
                                // Simple logic: Dispatch FetchEvent.
                                // respondWith() would need complex binding, defaulting to PassThrough for now
                                // but through the oneshot channel to unblock ResourceManager.
                                let _ = resp_tx.send(InterceptResult::PassThrough);
                            });
                        });
                    }
                    SwEvent::Terminate => break,
                }
            }
            is_running.store(false, std::sync::atomic::Ordering::SeqCst);
            println!("[SW] Thread stopped for Worker: {}", script_url);
        });

        let mut h = self.thread_handle.lock().unwrap();
        *h = Some(handle);
    }

    pub fn terminate(&self) {
        if let Some(tx) = self.event_tx.lock().unwrap().as_ref() {
            let _ = tx.send(SwEvent::Terminate);
        }
        self.is_running
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn set_state(&self, state: ServiceWorkerState) -> std::result::Result<(), String> {
        let mut current_state = self.state.lock().map_err(|e| e.to_string())?;
        *current_state = state;
        Ok(())
    }

    pub fn get_state(&self) -> std::result::Result<ServiceWorkerState, String> {
        let state = self.state.lock().map_err(|e| e.to_string())?;
        Ok(state.clone())
    }

    pub fn add_client(&self, client: ClientInfo) -> std::result::Result<(), String> {
        let mut clients = self.pending_clients.lock().map_err(|e| e.to_string())?;
        clients.push(client);
        Ok(())
    }

    pub fn get_clients(&self) -> std::result::Result<Vec<ClientInfo>, String> {
        let clients = self.pending_clients.lock().map_err(|e| e.to_string())?;
        Ok(clients.clone())
    }
}

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

    pub fn set_installing(
        &self,
        instance: Option<Arc<ServiceWorkerInstance>>,
    ) -> std::result::Result<(), String> {
        let mut installing = self.installing.lock().map_err(|e| e.to_string())?;
        *installing = instance;
        Ok(())
    }

    pub fn set_installed(
        &self,
        instance: Option<Arc<ServiceWorkerInstance>>,
    ) -> std::result::Result<(), String> {
        let mut installed = self.installed.lock().map_err(|e| e.to_string())?;
        *installed = instance;
        Ok(())
    }

    pub fn set_activating(
        &self,
        instance: Arc<ServiceWorkerInstance>,
    ) -> std::result::Result<(), String> {
        let mut activating = self.activating.lock().map_err(|e| e.to_string())?;
        *activating = Some(instance);
        Ok(())
    }

    pub fn set_active(
        &self,
        instance: Arc<ServiceWorkerInstance>,
    ) -> std::result::Result<(), String> {
        let mut active = self.active.lock().map_err(|e| e.to_string())?;
        *active = Some(instance);
        Ok(())
    }

    pub fn get_active(&self) -> std::result::Result<Option<Arc<ServiceWorkerInstance>>, String> {
        let active = self.active.lock().map_err(|e| e.to_string())?;
        Ok(active.clone())
    }

    pub fn get_installing(
        &self,
    ) -> std::result::Result<Option<Arc<ServiceWorkerInstance>>, String> {
        let installing = self.installing.lock().map_err(|e| e.to_string())?;
        Ok(installing.clone())
    }

    pub fn get_installed(&self) -> std::result::Result<Option<Arc<ServiceWorkerInstance>>, String> {
        let installed = self.installed.lock().map_err(|e| e.to_string())?;
        Ok(installed.clone())
    }

    pub fn get_activating(
        &self,
    ) -> std::result::Result<Option<Arc<ServiceWorkerInstance>>, String> {
        let activating = self.activating.lock().map_err(|e| e.to_string())?;
        Ok(activating.clone())
    }

    pub fn scope_matches(&self, url: &str) -> bool {
        url.starts_with(&self.scope)
    }
}

// ============================================================================
// SERVICE WORKER MANAGER (Global Registry)
// ============================================================================

pub struct ServiceWorkerManager {
    pub registrations: Arc<Mutex<HashMap<String, Arc<ServiceWorkerRegistration>>>>,
    pub db: Arc<ServiceWorkerDatabase>,
}

impl Clone for ServiceWorkerManager {
    fn clone(&self) -> Self {
        Self {
            registrations: self.registrations.clone(),
            db: self.db.clone(),
        }
    }
}

impl ServiceWorkerManager {
    pub fn new(db: Arc<ServiceWorkerDatabase>) -> Self {
        Self {
            registrations: Arc::new(Mutex::new(HashMap::new())),
            db,
        }
    }

    pub fn load_from_db(&self) -> std::result::Result<(), String> {
        let saved = self
            .db
            .get_all_registrations()
            .map_err(|e| format!("DB Error: {}", e))?;

        let mut regs = self.registrations.lock().unwrap();
        for data in saved {
            let reg = Arc::new(ServiceWorkerRegistration::new(
                data.scope.clone(),
                data.script_url.clone(),
                data.origin.clone(),
                self.db.clone(),
            ));

            // Restore ID using unsafe as it's immutable field
            let mut_reg = unsafe { &mut *(Arc::as_ptr(&reg) as *mut ServiceWorkerRegistration) };
            mut_reg.id = data.id.clone();

            let key = format!("{}#{}", data.origin, data.scope);
            regs.insert(key, reg);
        }
        println!(
            "[ServiceWorkerManager] Hydrated {} registrations from DB",
            regs.len()
        );
        Ok(())
    }

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

    pub fn dispatch_fetch_event(
        &self,
        instance: &ServiceWorkerInstance,
        request: RequestContext,
    ) -> JsResult<InterceptResult> {
        let (tx, rx) = tokio::sync::oneshot::channel::<InterceptResult>();

        if let Some(event_tx) = &*instance.event_tx.lock().unwrap() {
            let _ = event_tx.send(SwEvent::Fetch(request, tx));

            // Wait for response with timeout
            match rx.blocking_recv() {
                Ok(res) => return Ok(res),
                Err(_) => return Ok(InterceptResult::PassThrough),
            }
        }

        Ok(InterceptResult::PassThrough)
    }

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

        if let Some(tx) = &*instance.event_tx.lock().unwrap() {
            let _ = tx.send(SwEvent::Execute(script_content));
            let _ = tx.send(SwEvent::Dispatch("install".to_string()));
        }

        reg.set_installing(None)?;
        reg.set_installed(Some(instance))?;

        Ok(())
    }
}

impl Default for ServiceWorkerManager {
    fn default() -> Self {
        let db = Arc::new(ServiceWorkerDatabase::new(std::path::PathBuf::from("sw.db")).unwrap());
        Self::new(db)
    }
}
