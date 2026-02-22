use std::collections::{VecDeque, HashMap};
use std::time::{Duration, Instant};
use rquickjs::{Persistent, Function, Ctx, Value};
use std::sync::mpsc::{Sender, Receiver, channel};

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

pub struct EventLoop {
    macro_tasks: VecDeque<Box<dyn FnOnce() + Send>>,
    timers: HashMap<u32, TimerTask>,
    next_timer_id: u32,
    // Async bridge
    pub async_sender: Sender<AsyncResult>,
    async_receiver: Receiver<AsyncResult>,
    pending_resolutions: HashMap<u32, PromiseResolution>,
    next_resolution_id: u32,
    pub next_observer_id: usize,
    raf_callbacks: Vec<UnsafeSendVal<Persistent<Function<'static>>>>,
    /// Cross-runtime messages queued via postMessage
    pub pending_messages: VecDeque<PendingMessage>,
}


impl EventLoop {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            macro_tasks: VecDeque::new(),
            timers: HashMap::new(),
            next_timer_id: 1,
            async_sender: tx,
            async_receiver: rx,
            pending_resolutions: HashMap::new(),
            next_resolution_id: 1,
            next_observer_id: 0,
            raf_callbacks: Vec::new(),
            pending_messages: VecDeque::new(),
        }
    }

    /// Queue a postMessage for delivery on the next run_pending() tick.
    /// Never blocks - safe to call from any context.
    pub fn enqueue_message(&mut self, data_json: String, origin: String, source_runtime_id: Option<usize>) {
        self.pending_messages.push_back(PendingMessage { data_json, origin, source_runtime_id });
    }

    /// Drain all pending messages for processing in run_pending().
    pub fn take_pending_messages(&mut self) -> VecDeque<PendingMessage> {
        std::mem::take(&mut self.pending_messages)
    }

    pub fn register_promise(&mut self, resolve: Persistent<Function<'static>>, reject: Persistent<Function<'static>>) -> u32 {
        let id = self.next_resolution_id;
        self.next_resolution_id += 1;
        self.pending_resolutions.insert(id, PromiseResolution { resolve: UnsafeSendVal(resolve), reject: UnsafeSendVal(reject) });
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

    pub fn queue_macro_task<F>(&mut self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.macro_tasks.push_back(Box::new(task));
    }

    pub fn set_timer(&mut self, callback: Persistent<Function<'static>>, delay: u64, is_interval: bool) -> u32 {
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
}
