use super::*;
use rquickjs::{Function, Persistent};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};



impl EventLoop {

    /// Drain all pending messages for processing in run_pending().
    pub fn take_pending_messages(&mut self) -> VecDeque<PendingMessage> {
        std::mem::take(&mut self.pending_messages)
    }

    /// TODO: add docs
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

    /// TODO: add docs
    pub fn take_resolution(&mut self, id: u32) -> Option<PromiseResolution> {
        self.pending_resolutions.remove(&id)
    }

    /// TODO: add docs
    pub fn receive_async_results(&mut self) -> Vec<AsyncResult> {
        let mut results = Vec::new();
        while let Ok(res) = self.async_receiver.try_recv() {
            results.push(res);
        }
        results
    }

    /// TODO: add docs
    pub fn receive_idb_events(&mut self) -> Vec<IDBEventMessage> {
        let mut events = Vec::new();
        while let Ok(event) = self.idb_receiver.try_recv() {
            events.push(event);
        }
        events
    }

    /// TODO: add docs
    pub fn get_next_idb_callback_id(&mut self) -> usize {
        let id = self.next_idb_callback_id;
        self.next_idb_callback_id += 1;
        id
    }

    /// TODO: add docs
    pub fn queue_macro_task<F>(&mut self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.macro_tasks.push_back(Box::new(task));
    }

    /// TODO: add docs
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

    /// TODO: add docs
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
}
