use super::*;
use rquickjs::{Function, Persistent};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};



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
