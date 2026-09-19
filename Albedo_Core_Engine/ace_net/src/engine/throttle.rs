use crate::error::NetResult;
use crate::engine::priority::PriorityLevel;
use crate::engine::scheduler::ResourceScheduler;
use ace_core::id::RequestId;
use bytes::Bytes;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::future::Future;
use tokio::sync::watch;
use futures_core::Stream;

pub struct ThrottledStream {
    inner: crate::http::response::BoxByteStream,
    priority_rx: watch::Receiver<PriorityLevel>,
    scheduler: Arc<ResourceScheduler>,
    request_id: RequestId,
    sleep: Option<Pin<Box<tokio::time::Sleep>>>,
}

impl ThrottledStream {
    pub fn new(
        inner: crate::http::response::BoxByteStream,
        priority_rx: watch::Receiver<PriorityLevel>,
        scheduler: Arc<ResourceScheduler>,
        request_id: RequestId,
    ) -> Self {
        Self {
            inner,
            priority_rx,
            scheduler,
            request_id,
            sleep: None,
        }
    }
}

impl Drop for ThrottledStream {
    fn drop(&mut self) {
        self.scheduler.unregister_active_stream(self.request_id);
    }
}

impl Stream for ThrottledStream {
    type Item = NetResult<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(sleep) = &mut self.sleep {
            if sleep.as_mut().poll(cx).is_pending() {
                return Poll::Pending;
            }
            self.sleep = None;
        }

        let res = Pin::new(&mut self.inner).poll_next(cx);
        if res.is_ready() {
            let priority = *self.priority_rx.borrow();
            if priority <= PriorityLevel::Low {
                let delay = if priority == PriorityLevel::Lowest {
                    std::time::Duration::from_millis(50)
                } else {
                    std::time::Duration::from_millis(10)
                };
                self.sleep = Some(Box::pin(tokio::time::sleep(delay)));
            }
        }
        res
    }
}
