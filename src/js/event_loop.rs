use std::collections::{VecDeque, HashMap};
use std::time::{Duration, Instant};
use rquickjs::{Persistent, Function, Ctx, Value};

#[derive(Clone)]
pub struct TimerTask {
    pub id: u32,
    pub callback: Persistent<Function<'static>>,
    pub deadline: Instant,
    pub interval: Option<Duration>,
}

pub struct EventLoop {
    macro_tasks: VecDeque<Box<dyn FnOnce()>>,
    timers: HashMap<u32, TimerTask>,
    next_timer_id: u32,
}

impl EventLoop {
    pub fn new() -> Self {
        Self {
            macro_tasks: VecDeque::new(),
            timers: HashMap::new(),
            next_timer_id: 1,
        }
    }

    pub fn queue_macro_task<F>(&mut self, task: F)
    where
        F: FnOnce() + 'static,
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
            callback,
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
    pub fn take_pending_tasks(&mut self) -> (Vec<TimerTask>, VecDeque<Box<dyn FnOnce()>>) {
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

        let ready_macros = std::mem::take(&mut self.macro_tasks);
        
        (ready_timers, ready_macros)
    }
}
