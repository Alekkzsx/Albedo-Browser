use rquickjs::{Function, Persistent};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

pub struct AsyncResult {
    pub id: u32,
    pub result: Result<(u16, String), String>,
}

#[derive(Clone)]
pub struct UnsafeSendVal<T>(pub T);
unsafe impl<T> Send for UnsafeSendVal<T> {}
unsafe impl<T> Sync for UnsafeSendVal<T> {}

pub struct PromiseResolution {
    pub resolve: UnsafeSendVal<Persistent<Function<'static>>>,
    pub reject: UnsafeSendVal<Persistent<Function<'static>>>,
}

#[derive(Clone)]
pub struct TimerTask {
    pub id: u32,
    pub callback: UnsafeSendVal<Persistent<Function<'static>>>,
    pub deadline: Instant,
    pub interval: Option<Duration>,
}

/// A message queued via postMessage for delivery on next run_pending()
#[derive(Clone)]
pub struct PendingMessage {
    pub data_json: String,
    pub origin: String,
    pub source_runtime_id: Option<usize>,
}

pub enum IDBEventMessage {
    Success {
        callback_id: usize,
        result_json: String,
    },
    DatabaseSuccess {
        callback_id: usize,
        db_name: String,
        version: u32,
    },
    Error {
        callback_id: usize,
        error_name: String,
        error_message: String,
    },
    UpgradeNeeded {
        request_callback_id: usize,
        transaction_id: usize,
        db_name: String,
        old_version: u32,
        new_version: u32,
    },
}

#[derive(Clone)]
pub struct IdleCallbackTask {
    pub id: u32,
    pub callback: UnsafeSendVal<Persistent<Function<'static>>>,
    pub timeout_deadline: Option<Instant>,
}

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

impl EventLoop {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let (idb_tx, idb_rx) = channel();

        Self {
            macro_tasks: VecDeque::new(),
            timers: HashMap::new(),
            next_timer_id: 1,
            idle_callbacks: HashMap::new(),
            next_idle_id: 1,
            async_sender: tx,
            async_receiver: rx,
            pending_resolutions: HashMap::new(),
            next_resolution_id: 1,
            next_observer_id: 0,
            raf_callbacks: Vec::new(),
            pending_messages: VecDeque::new(),
            idb_sender: idb_tx,
            idb_receiver: idb_rx,
            next_idb_callback_id: 1,
            background_sync_queue: None,
            periodic_sync_scheduler: None,
            fetch_interceptor_chain: None,
            online_status: std::sync::Arc::new(std::sync::Mutex::new(true)),
        }
    }

    pub fn push_idle_callback(
        &mut self,
        callback: Persistent<Function<'static>>,
        timeout_ms: Option<u64>,
    ) -> u32 {
        let id = self.next_idle_id;
        self.next_idle_id += 1;

        let timeout_deadline = timeout_ms.map(|ms| Instant::now() + Duration::from_millis(ms));

        self.idle_callbacks.insert(
            id,
            IdleCallbackTask {
                id,
                callback: UnsafeSendVal(callback),
                timeout_deadline,
            },
        );

        id
    }

    pub fn cancel_idle_callback(&mut self, id: u32) {
        self.idle_callbacks.remove(&id);
    }

    pub fn take_idle_callbacks(&mut self, frame_deadline: Instant) -> Vec<IdleCallbackTask> {
        let now = Instant::now();
        let mut to_run = Vec::new();
        let mut ids_to_remove = Vec::new();

        // 1. Run if timed out
        // 2. Or run if we have remaining idle time
        let has_idle_time = now < frame_deadline;

        for (id, task) in &self.idle_callbacks {
            let mut should_run = false;

            if let Some(deadline) = task.timeout_deadline {
                if now >= deadline {
                    should_run = true;
                }
            }

            if !should_run && has_idle_time {
                should_run = true;
            }

            if should_run {
                to_run.push(task.clone());
                ids_to_remove.push(*id);
            }
        }

        // Remove the ones we are going to run
        for id in ids_to_remove {
            self.idle_callbacks.remove(&id);
        }

        to_run
    }

    /// Queue a postMessage for delivery on the next run_pending() tick.
    /// Never blocks - safe to call from any context.
    pub fn enqueue_message(
        &mut self,
        data_json: String,
        origin: String,
        source_runtime_id: Option<usize>,
    ) {
        self.pending_messages.push_back(PendingMessage {
            data_json,
            origin,
            source_runtime_id,
        });
    }

    /// Drain all pending messages for processing in run_pending().
    pub fn take_pending_messages(&mut self) -> VecDeque<PendingMessage> {
        std::mem::take(&mut self.pending_messages)
    }

    pub fn register_promise(
        &mut self,
        resolve: Persistent<Function<'static>>,
        reject: Persistent<Function<'static>>,
    ) -> u32 {
        let id = self.next_resolution_id;
        self.next_resolution_id += 1;
        self.pending_resolutions.insert(
            id,
            PromiseResolution {
                resolve: UnsafeSendVal(resolve),
                reject: UnsafeSendVal(reject),
            },
        );
        id
    }

    pub fn take_resolution(&mut self, id: u32) -> Option<PromiseResolution> {
        self.pending_resolutions.remove(&id)
    }

    pub fn receive_async_results(&mut self) -> Vec<AsyncResult> {
        let mut results = Vec::new();
        while let Ok(res) = self.async_receiver.try_recv() {
            results.push(res);
        }
        results
    }

    pub fn receive_idb_events(&mut self) -> Vec<IDBEventMessage> {
        let mut events = Vec::new();
        while let Ok(event) = self.idb_receiver.try_recv() {
            events.push(event);
        }
        events
    }

    pub fn get_next_idb_callback_id(&mut self) -> usize {
        let id = self.next_idb_callback_id;
        self.next_idb_callback_id += 1;
        id
    }

    pub fn queue_macro_task<F>(&mut self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.macro_tasks.push_back(Box::new(task));
    }

    pub fn set_timer(
        &mut self,
        callback: Persistent<Function<'static>>,
        delay: u64,
        is_interval: bool,
    ) -> u32 {
        let id = self.next_timer_id;
        self.next_timer_id += 1;

        let duration = Duration::from_millis(delay);
        let deadline = Instant::now() + duration;

        let task = TimerTask {
            id,
            callback: UnsafeSendVal(callback),
            deadline,
            interval: if is_interval { Some(duration) } else { None },
        };

        self.timers.insert(id, task);
        id
    }

    pub fn clear_timer(&mut self, id: u32) {
        self.timers.remove(&id);
    }

    /// Extract all pending tasks ensuring no locks are held during execution later
    pub fn take_pending_tasks(&mut self) -> (Vec<TimerTask>, VecDeque<Box<dyn FnOnce() + Send>>) {
        let now = Instant::now();
        let mut expired_ids = Vec::new();

        for (id, task) in &self.timers {
            if task.deadline <= now {
                expired_ids.push(*id);
            }
        }

        let mut ready_timers = Vec::new();

        for id in expired_ids {
            if let Some(task) = self.timers.remove(&id) {
                // If it's an interval, we reschedule a clone immediately
                if let Some(interval) = task.interval {
                    let next_task = TimerTask {
                        id,
                        callback: task.callback.clone(),
                        deadline: now + interval,
                        interval: Some(interval),
                    };
                    self.timers.insert(id, next_task);
                }

                // Add the original task to the ready list
                ready_timers.push(task);
            }
        }

        (ready_timers, std::collections::VecDeque::new())
    }

    pub fn push_raf_callback(&mut self, callback: Persistent<Function<'static>>) {
        self.raf_callbacks.push(UnsafeSendVal(callback));
    }

    pub fn take_raf_callbacks(&mut self) -> Vec<UnsafeSendVal<Persistent<Function<'static>>>> {
        std::mem::take(&mut self.raf_callbacks)
    }

    /// Set background sync queue (called during runtime init)
    pub fn set_background_sync_queue(
        &mut self,
        queue: std::sync::Arc<crate::ace::runtime::core::service_worker::BackgroundSyncQueue>,
    ) {
        self.background_sync_queue = Some(queue);
    }

    /// Set periodic sync scheduler (called during runtime init)
    pub fn set_periodic_sync_scheduler(
        &mut self,
        scheduler: std::sync::Arc<crate::ace::runtime::core::service_worker::PeriodicSyncScheduler>,
    ) {
        self.periodic_sync_scheduler = Some(scheduler);
    }

    /// Set fetch interceptor chain (called during runtime init)
    pub fn set_fetch_interceptor_chain(
        &mut self,
        chain: std::sync::Arc<
            std::sync::Mutex<crate::ace::runtime::core::service_worker::FetchInterceptorChain>,
        >,
    ) {
        self.fetch_interceptor_chain = Some(chain);
    }

    /// Set online status
    pub fn set_online_status(&self, online: bool) -> Result<(), String> {
        let mut status = self
            .online_status
            .lock()
            .map_err(|e| format!("Failed to lock online_status: {}", e))?;
        *status = online;
        Ok(())
    }

    /// Get online status
    pub fn is_online(&self) -> Result<bool, String> {
        let status = self
            .online_status
            .lock()
            .map_err(|e| format!("Failed to lock online_status: {}", e))?;
        Ok(*status)
    }

    /// Get pending background sync tasks (for PHASE 3b in executor)
    pub fn take_pending_background_sync(
        &self,
    ) -> Vec<crate::ace::runtime::core::service_worker::SyncTask> {
        if let Some(queue) = &self.background_sync_queue {
            queue.take_pending()
        } else {
            Vec::new()
        }
    }

    /// Get pending periodic sync tasks (for PHASE 3b in executor)
    pub fn take_pending_periodic_sync(
        &self,
    ) -> Vec<crate::ace::runtime::core::service_worker::PeriodicSyncTask> {
        if let Some(scheduler) = &self.periodic_sync_scheduler {
            scheduler.check_due()
        } else {
            Vec::new()
        }
    }
}
