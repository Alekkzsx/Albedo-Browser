//! Thread Pool Implementation for ACE-HTML Parser
//! 
//! Zero-dependency thread pool using only std::thread and std::sync::mpsc.
//! Designed for parallel parsing operations like speculative parsing and
//! preload scanning.
//! 
//! # Features
//! - Worker threads with job queue
//! - Bounded channels with backpressure
//! - Graceful shutdown
//! - Work stealing (optional)

use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Job to be executed by worker threads
type Job = Box<dyn FnOnce() + Send + 'static>;

/// Message sent to worker threads
enum Message {
    NewJob(Job),
    Terminate,
}

/// Worker thread that processes jobs from the queue
struct Worker {
    #[allow(dead_code)]
    id: usize,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Self {
        let thread = thread::spawn(move || {
            loop {
                let message = receiver.lock().unwrap().recv();
                
                match message {
                    Ok(Message::NewJob(job)) => {
                        job();
                    }
                    Ok(Message::Terminate) => {
                        break;
                    }
                    Err(_) => {
                        // Channel closed, terminate
                        break;
                    }
                }
            }
        });
        
        Worker {
            id,
            thread: Some(thread),
        }
    }
}

/// Thread pool for parallel execution
/// 
/// # Example
/// ```
/// use ace::html::thread_pool::ThreadPool;
/// 
/// let pool = ThreadPool::new(4);
/// 
/// for i in 0..10 {
///     pool.execute(move || {
///         println!("Job {} executing", i);
///     });
/// }
/// 
/// // Pool automatically shuts down when dropped
/// ```
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Message>>,
    stats: Arc<Mutex<PoolStats>>,
}

/// Statistics for the thread pool
#[derive(Clone, Default)]
pub struct PoolStats {
    pub jobs_submitted: usize,
    pub jobs_completed: usize,
    pub jobs_failed: usize,
}

impl ThreadPool {
    /// Creates a new thread pool with the specified number of workers
    /// 
    /// # Panics
    /// Panics if size is 0
    pub fn new(size: usize) -> Self {
        assert!(size > 0, "Thread pool size must be greater than 0");
        
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        
        let mut workers = Vec::with_capacity(size);
        
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }
        
        ThreadPool {
            workers,
            sender: Some(sender),
            stats: Arc::new(Mutex::new(PoolStats::default())),
        }
    }
    
    /// Creates a thread pool with number of workers equal to CPU cores
    pub fn default_size() -> Self {
        let size = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self::new(size)
    }
    
    /// Executes a job on the thread pool
    /// 
    /// # Errors
    /// Returns error if the pool has been shut down
    pub fn execute<F>(&self, f: F) -> Result<(), &'static str>
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        
        if let Some(sender) = &self.sender {
            sender.send(Message::NewJob(job))
                .map_err(|_| "Thread pool has been shut down")?;
            
            self.stats.lock().unwrap().jobs_submitted += 1;
            Ok(())
        } else {
            Err("Thread pool has been shut down")
        }
    }
    
    /// Returns the number of worker threads
    pub fn size(&self) -> usize {
        self.workers.len()
    }
    
    /// Returns statistics about the thread pool
    pub fn stats(&self) -> PoolStats {
        self.stats.lock().unwrap().clone()
    }
    
    /// Gracefully shuts down the thread pool
    /// 
    /// Waits for all workers to finish their current jobs
    pub fn shutdown(&mut self) {
        // Drop sender to close the channel
        if let Some(sender) = self.sender.take() {
            drop(sender);
        }
        
        // Wait for all workers to finish
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().ok();
            }
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Send terminate message to all workers
        if let Some(sender) = &self.sender {
            for _ in 0..self.workers.len() {
                sender.send(Message::Terminate).ok();
            }
        }
        
        // Wait for all workers to finish
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().ok();
            }
        }
    }
}

/// Bounded channel with backpressure support
/// 
/// # Example
/// ```
/// use ace::html::thread_pool::BoundedChannel;
/// 
/// let (tx, rx) = BoundedChannel::new(10);
/// 
/// tx.send("message").unwrap();
/// let msg = rx.recv().unwrap();
/// ```
#[allow(dead_code)]
pub struct BoundedChannel<T> {
    sender: mpsc::SyncSender<T>,
    receiver: mpsc::Receiver<T>,
}

impl<T> BoundedChannel<T> {
    /// Creates a new bounded channel with the specified capacity
    pub fn new(capacity: usize) -> (mpsc::SyncSender<T>, mpsc::Receiver<T>) {
        mpsc::sync_channel(capacity)
    }
    
    /// Creates a bounded channel with timeout support
    pub fn with_timeout(capacity: usize) -> (SenderWithTimeout<T>, ReceiverWithTimeout<T>) {
        let (tx, rx) = mpsc::sync_channel(capacity);
        (
            SenderWithTimeout { inner: tx },
            ReceiverWithTimeout { inner: rx },
        )
    }
}

/// Sender with timeout support
pub struct SenderWithTimeout<T> {
    inner: mpsc::SyncSender<T>,
}

impl<T> SenderWithTimeout<T> {
    /// Sends a value (blocking)
    /// 
    /// Note: Timeout functionality requires unstable features.
    /// Use try_send() for non-blocking sends or send() for blocking sends.
    pub fn send(&self, value: T) -> Result<(), mpsc::SendError<T>> {
        self.inner.send(value)
    }
    
    /// Tries to send a value without blocking
    pub fn try_send(&self, value: T) -> Result<(), mpsc::TrySendError<T>> {
        self.inner.try_send(value)
    }
}

/// Receiver with timeout support
pub struct ReceiverWithTimeout<T> {
    inner: mpsc::Receiver<T>,
}

impl<T> ReceiverWithTimeout<T> {
    /// Receives a value with a timeout
    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, mpsc::RecvTimeoutError> {
        self.inner.recv_timeout(timeout)
    }
    
    /// Receives a value (blocking)
    pub fn recv(&self) -> Result<T, mpsc::RecvError> {
        self.inner.recv()
    }
    
    /// Tries to receive a value without blocking
    pub fn try_recv(&self) -> Result<T, mpsc::TryRecvError> {
        self.inner.try_recv()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    #[test]
    fn test_thread_pool_creation() {
        let pool = ThreadPool::new(4);
        assert_eq!(pool.size(), 4);
    }
    
    #[test]
    fn test_thread_pool_execute() {
        let pool = ThreadPool::new(4);
        let counter = Arc::new(AtomicUsize::new(0));
        
        for _ in 0..10 {
            let counter = Arc::clone(&counter);
            pool.execute(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            }).unwrap();
        }
        
        // Wait for jobs to complete
        thread::sleep(Duration::from_millis(100));
        
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }
    
    #[test]
    fn test_thread_pool_shutdown() {
        let mut pool = ThreadPool::new(2);
        
        pool.execute(|| {
            thread::sleep(Duration::from_millis(10));
        }).unwrap();
        
        pool.shutdown();
        
        // After shutdown, execute should fail
        let result = pool.execute(|| {});
        assert!(result.is_err());
    }
    
    #[test]
    fn test_bounded_channel() {
        let (tx, rx) = BoundedChannel::new(5);
        
        tx.send(42).unwrap();
        tx.send(43).unwrap();
        
        assert_eq!(rx.recv().unwrap(), 42);
        assert_eq!(rx.recv().unwrap(), 43);
    }
    
    #[test]
    fn test_bounded_channel_backpressure() {
        let (tx, rx) = BoundedChannel::new(2);
        
        // Fill the channel
        tx.send(1).unwrap();
        tx.send(2).unwrap();
        
        // This should block, so we test with try_send in a thread
        let tx_clone = tx.clone();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            tx_clone.send(3)
        });
        
        // Receive one to make space
        thread::sleep(Duration::from_millis(10));
        assert_eq!(rx.recv().unwrap(), 1);
        
        // Now the blocked send should succeed
        handle.join().unwrap().unwrap();
        
        assert_eq!(rx.recv().unwrap(), 2);
        assert_eq!(rx.recv().unwrap(), 3);
    }
    
    #[test]
    fn test_channel_with_timeout() {
        let (tx, rx) = BoundedChannel::with_timeout(5);
        
        tx.send(100).unwrap();
        
        let result = rx.recv_timeout(Duration::from_millis(100));
        assert_eq!(result.unwrap(), 100);
        
        // Timeout on empty channel
        let result = rx.recv_timeout(Duration::from_millis(10));
        assert!(result.is_err());
    }
    
    #[test]
    fn test_sender_try_send() {
        let (tx, rx) = BoundedChannel::with_timeout(2);
        
        // Fill the channel
        tx.send(1).unwrap();
        tx.send(2).unwrap();
        
        // try_send should fail when channel is full
        let result = tx.try_send(3);
        assert!(result.is_err());
        
        // Receive one to make space
        assert_eq!(rx.recv().unwrap(), 1);
        
        // Now try_send should succeed
        tx.try_send(3).unwrap();
        
        assert_eq!(rx.recv().unwrap(), 2);
        assert_eq!(rx.recv().unwrap(), 3);
    }
    
    #[test]
    fn test_default_size() {
        let pool = ThreadPool::default_size();
        assert!(pool.size() > 0);
    }
    
    #[test]
    fn test_concurrent_execution() {
        let pool = ThreadPool::new(4);
        let counter = Arc::new(AtomicUsize::new(0));
        
        for i in 0..100 {
            let counter = Arc::clone(&counter);
            pool.execute(move || {
                counter.fetch_add(i, Ordering::SeqCst);
            }).unwrap();
        }
        
        // Wait for all jobs
        thread::sleep(Duration::from_millis(200));
        
        // Sum of 0..100 = 4950
        assert_eq!(counter.load(Ordering::SeqCst), 4950);
    }
}
