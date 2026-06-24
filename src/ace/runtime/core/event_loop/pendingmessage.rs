use super::*;
use rquickjs::{Function, Persistent};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};



/// A message queued via postMessage for delivery on next run_pending()
#[derive(Clone)]
pub struct PendingMessage {
    pub data_json: String,
    pub origin: String,
    pub source_runtime_id: Option<usize>,
}
