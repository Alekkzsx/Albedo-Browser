use ace_core::event_loop::EventLoop;
use ace_core::time::MockClock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_event_loop_order_and_microtask_checkpoint() {
    let el = EventLoop::new();
    let q = el.handle();
    let counter = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&counter);
    let q1 = q.clone();

    // Submete macrotask 1
    q.queue_user_interaction(move || {
        c1.store(1, Ordering::SeqCst);

        let c1_inner = Arc::clone(&c1);
        let q2 = q1.clone();

        // Enfileira microtask (deve rodar imediatamente ao fim desta macrotask)
        q1.queue_microtask(move || {
            assert_eq!(c1_inner.load(Ordering::SeqCst), 1);
            c1_inner.store(2, Ordering::SeqCst);
        });

        // Enfileira macrotask 2 (só pode rodar DEPOIS de todas as microtasks)
        let c1_inner2 = Arc::clone(&c1);
        q2.queue_network(move || {
            assert_eq!(c1_inner2.load(Ordering::SeqCst), 2);
            c1_inner2.store(3, Ordering::SeqCst);
        });
    });

    // Passo 1: Executa macrotask 1 + drena microtasks
    assert!(el.step());
    assert_eq!(counter.load(Ordering::SeqCst), 2);

    // Passo 2: Executa macrotask 2
    assert!(el.step());
    assert_eq!(counter.load(Ordering::SeqCst), 3);

    // Passo 3: Sem mais tarefas
    assert!(!el.step());
}

#[test]
fn test_unbounded_microtasks_no_drop() {
    let el = EventLoop::new();
    let q = el.handle();
    let counter = Arc::new(AtomicUsize::new(0));

    // Submete 25.000 microtasks simultâneas (o código anterior falhava em 10.001)
    for _ in 0..25_000 {
        let c = Arc::clone(&counter);
        q.queue_microtask(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
    }

    let drained = el.drain_microtasks();
    assert_eq!(drained, 25_000);
    assert_eq!(counter.load(Ordering::SeqCst), 25_000);
}

#[test]
fn test_task_cancellation() {
    let el = EventLoop::new();
    let q = el.handle();
    let was_executed = Arc::new(AtomicBool::new(false));

    let executed_clone = Arc::clone(&was_executed);
    let (_task_id, cancel_handle) = q.queue_timer(move || {
        executed_clone.store(true, Ordering::SeqCst);
    });

    // Cancela a tarefa antes de ela ser processada (ex: clearTimeout)
    cancel_handle.store(true, Ordering::SeqCst);

    assert!(el.step());
    // A tarefa foi descartada e não executou seu corpo
    assert!(!was_executed.load(Ordering::SeqCst));
}

#[test]
fn test_scheduled_timers_with_mock_clock() {
    let mock_clock = Arc::new(MockClock::new(1_000));
    let el = EventLoop::with_clock(Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();

    let timer_executed = Arc::new(AtomicBool::new(false));
    let te_clone = Arc::clone(&timer_executed);

    // Agenda timer para +500ms (disparará em t = 1500ms)
    q.schedule_timer(Duration::from_millis(500), move || {
        te_clone.store(true, Ordering::SeqCst);
    });

    // Em t = 1000ms: timer ainda não expirou
    assert!(!el.step());
    assert!(!timer_executed.load(Ordering::SeqCst));

    // Avança 300ms (t = 1300ms): timer ainda não expirou
    mock_clock.advance(Duration::from_millis(300));
    assert!(!el.step());
    assert!(!timer_executed.load(Ordering::SeqCst));

    // Avança 250ms (t = 1550ms): timer expirou!
    mock_clock.advance(Duration::from_millis(250));
    assert!(el.step());
    assert!(timer_executed.load(Ordering::SeqCst));
}

#[test]
fn test_request_animation_frame() {
    let el = EventLoop::new();
    let q = el.handle();
    let raf_counter = Arc::new(AtomicUsize::new(0));

    let c = Arc::clone(&raf_counter);
    q.request_animation_frame(move || {
        c.fetch_add(1, Ordering::SeqCst);
    });

    let c2 = Arc::clone(&raf_counter);
    q.request_animation_frame(move || {
        c2.fetch_add(1, Ordering::SeqCst);
    });

    let executed = el.process_animation_frame();
    assert_eq!(executed, 2);
    assert_eq!(raf_counter.load(Ordering::SeqCst), 2);
}

#[test]
fn test_task_source_prioritization() {
    let el = EventLoop::new();
    let q = el.handle();
    let execution_order = Arc::new(parking_lot::Mutex::new(Vec::new()));

    let eo1 = Arc::clone(&execution_order);
    let eo2 = Arc::clone(&execution_order);
    let eo3 = Arc::clone(&execution_order);

    // Enfileira networking primeiro, depois DOM, depois UserInteraction
    q.queue_network(move || {
        eo1.lock().push("network");
    });
    q.queue_dom(move || {
        eo2.lock().push("dom");
    });
    q.queue_user_interaction(move || {
        eo3.lock().push("user");
    });

    // O event loop deve executar UserInteraction primeiro, depois DOM, depois Network!
    assert!(el.step());
    assert_eq!(*execution_order.lock(), vec!["user"]);

    assert!(el.step());
    assert_eq!(*execution_order.lock(), vec!["user", "dom"]);

    assert!(el.step());
    assert_eq!(*execution_order.lock(), vec!["user", "dom", "network"]);
}

#[test]
fn test_timer_min_heap_ordering() {
    let mock_clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let fired_timers = Arc::new(parking_lot::Mutex::new(Vec::new()));

    let f1 = Arc::clone(&fired_timers);
    let f2 = Arc::clone(&fired_timers);
    let f3 = Arc::clone(&fired_timers);

    // Agenda 300ms primeiro, depois 100ms, depois 200ms
    q.schedule_timer(Duration::from_millis(300), move || {
        f1.lock().push(300);
    });
    q.schedule_timer(Duration::from_millis(100), move || {
        f2.lock().push(100);
    });
    q.schedule_timer(Duration::from_millis(200), move || {
        f3.lock().push(200);
    });

    // Em t = 150ms: apenas o de 100ms deve ter expirado
    mock_clock.advance_millis(150);
    assert_eq!(el.process_expired_timers(), 1);
    assert_eq!(*fired_timers.lock(), vec![100]);

    // Em t = 350ms: os de 200ms e 300ms expiram na ordem correta
    mock_clock.advance_millis(200);
    assert_eq!(el.process_expired_timers(), 2);
    assert_eq!(*fired_timers.lock(), vec![100, 200, 300]);
}

#[test]
fn test_timer_clamping_and_background_throttling() {
    let mock_clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let fired = Arc::new(AtomicBool::new(false));

    // Nível de aninhamento >= 5 com delay 0ms deve ser clampado para 4ms
    let f1 = Arc::clone(&fired);
    q.schedule_timer_with_nesting(Duration::from_millis(0), 5, move || {
        f1.store(true, Ordering::SeqCst);
    });

    // Em t = 2ms: ainda não disparou por causa do clamp de 4ms
    mock_clock.advance_millis(2);
    assert_eq!(el.process_expired_timers(), 0);
    assert!(!fired.load(Ordering::SeqCst));

    // Em t = 4ms: dispara
    mock_clock.advance_millis(2);
    assert_eq!(el.process_expired_timers(), 1);
    assert!(fired.load(Ordering::SeqCst));

    // Throttling em background (mínimo 1000ms)
    q.set_background_throttling(true);
    let bg_fired = Arc::new(AtomicBool::new(false));
    let bg_clone = Arc::clone(&bg_fired);
    q.schedule_timer(Duration::from_millis(10), move || {
        bg_clone.store(true, Ordering::SeqCst);
    });

    mock_clock.advance_millis(500);
    assert_eq!(el.process_expired_timers(), 0);
    assert!(!bg_fired.load(Ordering::SeqCst));

    mock_clock.advance_millis(500);
    assert_eq!(el.process_expired_timers(), 1);
    assert!(bg_fired.load(Ordering::SeqCst));
}

