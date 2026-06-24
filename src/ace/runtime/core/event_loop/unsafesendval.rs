use super::*;
use rquickjs::{Function, Persistent};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};



#[derive(Clone)]
pub struct UnsafeSendVal<T>(pub T);
unsafe impl<T> Send for UnsafeSendVal<T> {}
unsafe impl<T> Sync for UnsafeSendVal<T> {}

pub struct PromiseResolution {
    pub resolve: UnsafeSendVal<Persistent<Function<'static>>>,
    pub reject: UnsafeSendVal<Persistent<Function<'static>>>,
}
