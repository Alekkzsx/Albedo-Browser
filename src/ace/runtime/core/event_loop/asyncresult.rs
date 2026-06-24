use super::*;
use rquickjs::{Function, Persistent};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};


pub struct AsyncResult {
    pub id: u32,
    pub result: Result<(u16, String), String>,
}
