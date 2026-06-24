use super::*;
use rquickjs::{Function, Persistent};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};



pub struct EventLoop {
    macro_tasks: VecDeque<Box<dyn FnOnce() + Send>>,
    timers: HashMap<u32, TimerTask>,
    next_timer_id: u32,
    // Idle Callbacks
    idle_callbacks: HashMap<u32, IdleCallbackTask>,
    next_idle_id: u32,
    // Async bridge
    pub async_sender: Sender<AsyncResult>,
    async_receiver: Receiver<AsyncResult>,
    pending_resolutions: HashMap<u32, PromiseResolution>,
    next_resolution_id: u32,
    pub next_observer_id: usize,
    raf_callbacks: Vec<UnsafeSendVal<Persistent<Function<'static>>>>,
    /// Cross-runtime messages queued via postMessage
    pub pending_messages: VecDeque<PendingMessage>,

    // IndexedDB Event Bridge
    pub idb_sender: Sender<IDBEventMessage>,
    idb_receiver: Receiver<IDBEventMessage>,
    pub next_idb_callback_id: usize,

    // Service Worker & Background Sync
    pub background_sync_queue:
        Option<std::sync::Arc<crate::ace::runtime::core::service_worker::BackgroundSyncQueue>>,
    pub periodic_sync_scheduler:
        Option<std::sync::Arc<crate::ace::runtime::core::service_worker::PeriodicSyncScheduler>>,
    pub fetch_interceptor_chain: Option<
        std::sync::Arc<
            std::sync::Mutex<crate::ace::runtime::core::service_worker::FetchInterceptorChain>,
        >,
    >,
    pub online_status: std::sync::Arc<std::sync::Mutex<bool>>, // true = online
}
