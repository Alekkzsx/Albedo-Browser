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

impl ServiceWorkerInstance {
    /// TODO: add docs
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

    /// TODO: add docs
    pub fn spawn(&self, script_url: String, origin: String) {
        if self.is_running.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<SwEvent>();
        *self.event_tx.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);

        let is_running = self.is_running.clone();
        is_running.store(true, std::sync::atomic::Ordering::SeqCst);

        let handle = std::thread::spawn(move || {
            tracing::info!(url = %script_url, "Service Worker thread started");
            // Initialize runtime inside the thread
            let rt = crate::ace::runtime::core::init::init_sw_runtime(&script_url, &origin)
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
            tracing::info!(url = %script_url, "Service Worker thread stopped");
        });

        let mut h = self.thread_handle.lock().unwrap_or_else(|e| e.into_inner());
        *h = Some(handle);
    }

    /// TODO: add docs
    pub fn terminate(&self) {
        if let Some(tx) = self.event_tx.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            let _ = tx.send(SwEvent::Terminate);
        }
        self.is_running
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// TODO: add docs
    pub fn set_state(&self, state: ServiceWorkerState) -> std::result::Result<(), String> {
        let mut current_state = self.state.lock().map_err(|e| e.to_string())?;
        *current_state = state;
        Ok(())
    }

    /// TODO: add docs
    pub fn get_state(&self) -> std::result::Result<ServiceWorkerState, String> {
        let state = self.state.lock().map_err(|e| e.to_string())?;
        Ok(state.clone())
    }

    /// TODO: add docs
    pub fn add_client(&self, client: ClientInfo) -> std::result::Result<(), String> {
        let mut clients = self.pending_clients.lock().map_err(|e| e.to_string())?;
        clients.push(client);
        Ok(())
    }

    /// TODO: add docs
    pub fn get_clients(&self) -> std::result::Result<Vec<ClientInfo>, String> {
        let clients = self.pending_clients.lock().map_err(|e| e.to_string())?;
        Ok(clients.clone())
    }
}
