use super::*;
use rquickjs::{Function, Persistent};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};



impl EventLoop {
    /// TODO: add docs
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

    /// TODO: add docs
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

    /// TODO: add docs
    pub fn cancel_idle_callback(&mut self, id: u32) {
        self.idle_callbacks.remove(&id);
    }

    /// TODO: add docs
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
}
