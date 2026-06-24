use super::*;
use rquickjs::{Function, Persistent};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};



impl EventLoop {

    /// TODO: add docs
    pub fn push_raf_callback(&mut self, callback: Persistent<Function<'static>>) {
        self.raf_callbacks.push(UnsafeSendVal(callback));
    }

    /// TODO: add docs
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
