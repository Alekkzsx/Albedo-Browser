use super::*;
use rquickjs::{Function, Persistent};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};



#[derive(Clone)]
pub struct TimerTask {
    pub id: u32,
    pub callback: UnsafeSendVal<Persistent<Function<'static>>>,
    pub deadline: Instant,
    pub interval: Option<Duration>,
}
