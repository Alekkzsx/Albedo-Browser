use ace_core::event_loop::EventLoop;
use ace_core::time::MockClock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_idle_task_execution_within_budget() {
    let mock_clock = Arc::new(MockClock::new(1000));
    let event_loop = EventLoop::with_clock(Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>);
    let task_queue = event_loop.handle();

    let executed = Arc::new(AtomicBool::new(false));
    let ex_clone = Arc::clone(&executed);

    task_queue.post_idle_task(None, move |deadline| {
        assert!(deadline.time_remaining_ms() > 0.0);
        assert!(!deadline.did_timeout());
        ex_clone.store(true, Ordering::SeqCst);
    });

    let count = event_loop.process_idle_tasks(Duration::from_millis(16));
    assert_eq!(count, 1);
    assert!(executed.load(Ordering::SeqCst));
}

#[test]
fn test_idle_task_timeout_forced_execution() {
    let mock_clock = Arc::new(MockClock::new(1000));
    let event_loop = EventLoop::with_clock(Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>);
    let task_queue = event_loop.handle();

    let executed = Arc::new(AtomicBool::new(false));
    let ex_clone = Arc::clone(&executed);

    // Tarefa com timeout de 50ms
    task_queue.post_idle_task(Some(Duration::from_millis(50)), move |deadline| {
        assert!(deadline.did_timeout());
        ex_clone.store(true, Ordering::SeqCst);
    });

    // Avança o relógio além do timeout (100ms)
    mock_clock.advance(Duration::from_millis(100));

    // Mesmo com orçamento 0, a tarefa deve executar pois expirou o timeout
    let count = event_loop.process_idle_tasks(Duration::from_millis(0));
    assert_eq!(count, 1);
    assert!(executed.load(Ordering::SeqCst));
}
