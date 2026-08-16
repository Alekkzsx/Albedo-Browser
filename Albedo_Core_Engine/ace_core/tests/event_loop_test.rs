use ace_core::event_loop::EventLoop;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn test_event_loop_order() {
    let el = EventLoop::new();
    let q = el.handle();
    let counter = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&counter);

    // Submetemos uma macrotask que, ao rodar, agenda uma microtask e outra macrotask.
    let q1 = q.clone();
    q.queue_macrotask(move || {
        c1.store(1, Ordering::SeqCst);
        
        let c1_inner = Arc::clone(&c1);
        let q2 = q1.clone();
        q1.queue_microtask(move || {
            // A microtask DEVE rodar logo em seguida, antes de qualquer outra macrotask.
            assert_eq!(c1_inner.load(Ordering::SeqCst), 1);
            c1_inner.store(2, Ordering::SeqCst);
        });

        let c1_inner2 = Arc::clone(&c1);
        q2.queue_macrotask(move || {
            // A segunda macrotask DEVE rodar após a microtask alterar pra 2.
            assert_eq!(c1_inner2.load(Ordering::SeqCst), 2);
            c1_inner2.store(3, Ordering::SeqCst);
        });
    });
}
