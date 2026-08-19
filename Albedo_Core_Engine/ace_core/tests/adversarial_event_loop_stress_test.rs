//! # Adversarial Event Loop Stress Tests (Milestone M3 Challenge Suite)
//!
//! Comprehensive empirical stress-testing suite for WHATWG Event Loop alignment:
//! 1. Starvation Prevention & Fair Queuing under extreme adversarial workloads
//! 2. Microtask Reentrancy, Nested Checkpoint Safety & Circuit Breaker Limits
//! 3. Interleaved Timer Macrotasks & Microtask Checkpoint Ordering
//! 4. Dynamic Timer Nesting Depth Clamping (depth >= 5 -> min 4ms)
//! 5. Multithreaded concurrent assault with Scope Invalidation

use ace_core::event_loop::{
    current_timer_nesting, EventLoop, TaskQueue, TaskSource,
};
use ace_core::time::MockClock;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

// =========================================================================
// 1. STARVATION PREVENTION & FAIR QUEUING ADVERSARIAL TESTS
// =========================================================================

#[test]
fn test_adversarial_starvation_multiple_low_priority_queues() {
    let mock_clock = Arc::new(MockClock::new(0));
    // Starvation limit = 3
    let el = EventLoop::with_clock_and_starvation_limit(
        Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>,
        3,
    );
    let q = el.handle();

    let execution_log = Arc::new(parking_lot::Mutex::new(Vec::new()));

    // Enfileira 1 tarefa em Networking (prio 4), 1 em Timer (prio 5), 1 em Internal (prio 6)
    let l_net = Arc::clone(&execution_log);
    q.queue_network(move || {
        l_net.lock().push("network");
    });

    let l_tim = Arc::clone(&execution_log);
    q.queue_timer(move || {
        l_tim.lock().push("timer");
    });

    let l_int = Arc::clone(&execution_log);
    q.queue_task(TaskSource::Internal, move || {
        l_int.lock().push("internal");
    });

    // Enfileira 50 tarefas de UI contínuas (prio 0)
    for i in 0..50 {
        let l_ui = Arc::clone(&execution_log);
        q.queue_user_interaction(move || {
            l_ui.lock().push("ui");
        });
        let _ = i;
    }

    // Executa 30 passos
    for _ in 0..30 {
        el.step();
    }

    let log = execution_log.lock().clone();

    // Verifica que todas as 3 fontes de baixa prioridade foram executadas!
    assert!(
        log.contains(&"network"),
        "Networking queue was starved by UI flood!"
    );
    assert!(
        log.contains(&"timer"),
        "Timer queue was starved by UI flood!"
    );
    assert!(
        log.contains(&"internal"),
        "Internal queue was starved by UI flood!"
    );

    // Verifica que tarefas de baixa prioridade foram intercaladas a cada <= 3 UI tasks
    let mut consecutive_ui = 0;
    let mut max_consecutive_ui = 0;
    for &entry in &log {
        if entry == "ui" {
            consecutive_ui += 1;
            if consecutive_ui > max_consecutive_ui {
                max_consecutive_ui = consecutive_ui;
            }
        } else {
            consecutive_ui = 0;
        }
    }

    assert!(
        max_consecutive_ui <= 3,
        "Starvation limit violated: UI executed {} times consecutively (limit was 3)",
        max_consecutive_ui
    );
}

#[test]
fn test_adversarial_starvation_dynamic_flood() {
    let mock_clock = Arc::new(MockClock::new(0));
    // Starvation limit = 2
    let el = EventLoop::with_clock_and_starvation_limit(
        Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>,
        2,
    );
    let q = el.handle();

    let net_ran = Arc::new(AtomicBool::new(false));
    let net_clone = Arc::clone(&net_ran);

    q.queue_network(move || {
        net_clone.store(true, Ordering::SeqCst);
    });

    let mut ui_steps = 0;
    // Injeta continuamente UI antes de cada step
    for _ in 0..100 {
        q.queue_user_interaction(|| {});
        el.step();
        if net_ran.load(Ordering::SeqCst) {
            break;
        }
        ui_steps += 1;
    }

    assert!(net_ran.load(Ordering::SeqCst), "Network task never ran under dynamic flood!");
    assert!(
        ui_steps <= 2,
        "Network task took {} UI steps to execute (starvation limit is 2)",
        ui_steps
    );
}

#[test]
fn test_adversarial_starvation_counter_reset_on_empty() {
    let el = EventLoop::with_clock_and_starvation_limit(
        Arc::new(MockClock::new(0)) as Arc<dyn ace_core::time::Clock>,
        5,
    );
    let q = el.handle();

    // Push and pop immediately
    q.queue_network(|| {});
    el.step();

    // Now run 10 UI tasks with Network queue EMPTY
    for _ in 0..10 {
        q.queue_user_interaction(|| {});
        el.step();
    }

    // Now enqueue Network task: it should start with counter 0, not have accumulated starvation while empty
    let net_counter = el.task_queues.starvation_counter(TaskSource::Networking);
    assert_eq!(net_counter, 0, "Empty queue accumulated starvation counters!");
}

// =========================================================================
// 2. MICROTASK REENTRANCY & SAFETY ADVERSARIAL TESTS
// =========================================================================

#[test]
fn test_adversarial_microtask_deep_recursion_circuit_breaker() {
    let el = EventLoop::new();
    let q = el.handle();

    let executed_count = Arc::new(AtomicUsize::new(0));

    // Recursive microtask generator
    fn queue_recursive(q: TaskQueue, count: Arc<AtomicUsize>, max: usize) {
        let q_clone = q.clone();
        let count_clone = Arc::clone(&count);
        q.queue_microtask(move || {
            let current = count_clone.fetch_add(1, Ordering::SeqCst) + 1;
            if current < max {
                queue_recursive(q_clone, count_clone, max);
            }
        });
    }

    // Attempt to queue 120,000 recursive microtasks (exceeds 100,000 circuit breaker)
    queue_recursive(q.clone(), Arc::clone(&executed_count), 120_000);

    let drained = el.drain_microtasks();

    // The circuit breaker stops at 100,001
    assert!(
        drained > 100_000,
        "Circuit breaker should have allowed 100,001 tasks, got {}",
        drained
    );
    assert_eq!(executed_count.load(Ordering::SeqCst), drained);

    // Guard was released after circuit breaker tripped
    assert!(!el.is_performing_microtask_checkpoint());

    // Second drain completes the remainder
    let remaining_drained = el.drain_microtasks();
    assert!(
        remaining_drained > 0,
        "Remaining tasks should be drainable in subsequent checkpoint"
    );
    assert_eq!(
        executed_count.load(Ordering::SeqCst),
        drained + remaining_drained
    );
}

#[test]
fn test_adversarial_microtask_reentrant_step_and_nested_calls() {
    let el = Arc::new(EventLoop::new());
    let q = el.handle();

    let reentrant_drain_result = Arc::new(AtomicUsize::new(999));
    let reentrant_step_ran = Arc::new(AtomicBool::new(false));
    let inner_microtask_ran = Arc::new(AtomicBool::new(false));

    let el_clone = Arc::clone(&el);
    let r_drain = Arc::clone(&reentrant_drain_result);
    let r_step = Arc::clone(&reentrant_step_ran);
    let i_micro = Arc::clone(&inner_microtask_ran);
    let q_clone = q.clone();

    // Macrotask
    q.queue_user_interaction(move || {
        let q_inner = q_clone.clone();
        let el_inner = Arc::clone(&el_clone);

        // Microtask
        q_clone.queue_microtask(move || {
            let i_clone = Arc::clone(&i_micro);
            q_inner.queue_microtask(move || {
                i_clone.store(true, Ordering::SeqCst);
            });

            // Call drain_microtasks reentrantly
            let res = el_inner.drain_microtasks();
            r_drain.store(res, Ordering::SeqCst);

            // Reentrancy checkpoint is protected
            assert!(el_inner.is_performing_microtask_checkpoint());

            // Queue a UI macrotask from inside microtask
            let r_step_clone = Arc::clone(&r_step);
            q_inner.queue_user_interaction(move || {
                r_step_clone.store(true, Ordering::SeqCst);
            });
        });
    });

    // Step 1: Runs macrotask + drains outer microtasks
    assert!(el.step());

    // Reentrant drain returned 0 per Step 1 of WHATWG spec
    assert_eq!(reentrant_drain_result.load(Ordering::SeqCst), 0);
    // The nested microtask ran in the outer drain loop
    assert!(inner_microtask_ran.load(Ordering::SeqCst));
    // Checkpoint flag is false afterwards
    assert!(!el.is_performing_microtask_checkpoint());
    // Step has not run the newly queued macrotask yet
    assert!(!reentrant_step_ran.load(Ordering::SeqCst));

    // Step 2: Runs the newly queued UI macrotask
    assert!(el.step());
    assert!(reentrant_step_ran.load(Ordering::SeqCst));
}

#[test]
fn test_adversarial_microtask_panic_recovery() {
    let el = Arc::new(EventLoop::new());
    let q = el.handle();

    let ran_before_panic = Arc::new(AtomicBool::new(false));
    let ran_after_panic = Arc::new(AtomicBool::new(false));

    let b_clone = Arc::clone(&ran_before_panic);
    q.queue_microtask(move || {
        b_clone.store(true, Ordering::SeqCst);
    });

    // Panicking microtask
    q.queue_microtask(|| {
        panic!("Adversarial microtask panic injection!");
    });

    let a_clone = Arc::clone(&ran_after_panic);
    q.queue_microtask(move || {
        a_clone.store(true, Ordering::SeqCst);
    });

    let el_clone = Arc::clone(&el);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        el_clone.drain_microtasks();
    }));
    assert!(result.is_err());

    // RAII guard MUST have reset the flag
    assert!(!el.is_performing_microtask_checkpoint());
    assert!(ran_before_panic.load(Ordering::SeqCst));

    // Remaining microtask can now be drained cleanly
    assert_eq!(el.drain_microtasks(), 1);
    assert!(ran_after_panic.load(Ordering::SeqCst));
    assert!(!el.is_performing_microtask_checkpoint());
}

// =========================================================================
// 3. INTERLEAVED TIMER MACROTASKS & MICROTASK ORDERING ADVERSARIAL TESTS
// =========================================================================

#[test]
fn test_adversarial_interleaved_timer_macrotasks_multi() {
    let mock_clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();

    let execution_order = Arc::new(parking_lot::Mutex::new(Vec::new()));

    // Schedule 3 timers for t=100ms
    for i in 1..=3 {
        let log = Arc::clone(&execution_order);
        let q_clone = q.clone();
        q.schedule_timer(Duration::from_millis(100), move || {
            log.lock().push(format!("timer_{}", i));

            // Each timer schedules 2 microtasks
            let log_m1 = Arc::clone(&log);
            let log_m2 = Arc::clone(&log);
            q_clone.queue_microtask(move || {
                log_m1.lock().push(format!("micro_{}_a", i));
            });
            q_clone.queue_microtask(move || {
                log_m2.lock().push(format!("micro_{}_b", i));
            });
        });
    }

    // Advance clock to t=100ms
    mock_clock.advance_millis(100);

    // Step 1: Timer 1 + Microtasks for Timer 1
    assert!(el.step());
    assert_eq!(
        *execution_order.lock(),
        vec!["timer_1", "micro_1_a", "micro_1_b"]
    );

    // Step 2: Timer 2 + Microtasks for Timer 2
    assert!(el.step());
    assert_eq!(
        *execution_order.lock(),
        vec![
            "timer_1", "micro_1_a", "micro_1_b",
            "timer_2", "micro_2_a", "micro_2_b"
        ]
    );

    // Step 3: Timer 3 + Microtasks for Timer 3
    assert!(el.step());
    assert_eq!(
        *execution_order.lock(),
        vec![
            "timer_1", "micro_1_a", "micro_1_b",
            "timer_2", "micro_2_a", "micro_2_b",
            "timer_3", "micro_3_a", "micro_3_b"
        ]
    );

    // No more tasks
    assert!(!el.step());
}

#[test]
fn test_adversarial_timer_spawns_timer_and_microtask() {
    let mock_clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();

    let history = Arc::new(parking_lot::Mutex::new(Vec::new()));

    let h1 = Arc::clone(&history);
    let q1 = q.clone();

    // Timer A at t=50ms
    q.schedule_timer(Duration::from_millis(50), move || {
        h1.lock().push("timer_a");

        let h_micro = Arc::clone(&h1);
        q1.queue_microtask(move || {
            h_micro.lock().push("micro_from_timer_a");
        });

        let h_inner_timer = Arc::clone(&h1);
        // Spawns Timer B at t=50ms (delay 0ms)
        q1.schedule_timer(Duration::from_millis(0), move || {
            h_inner_timer.lock().push("timer_b_nested");
        });
    });

    mock_clock.advance_millis(50);

    // Step 1: Executes Timer A, then its microtask. Timer B is scheduled in min-heap.
    assert!(el.step());
    assert_eq!(
        *history.lock(),
        vec!["timer_a", "micro_from_timer_a"]
    );

    // Step 2: Executes Timer B (delay 0ms, target_ms = 50 <= 50)
    assert!(el.step());
    assert_eq!(
        *history.lock(),
        vec!["timer_a", "micro_from_timer_a", "timer_b_nested"]
    );

    assert!(!el.step());
}

// =========================================================================
// 4. DYNAMIC TIMER NESTING CLAMPING ADVERSARIAL TESTS
// =========================================================================

#[test]
fn test_adversarial_dynamic_timer_nesting_deep_chain_timestamps() {
    let mock_clock = Arc::new(MockClock::new(1000));
    let el = EventLoop::with_clock(Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();

    let log = Arc::new(parking_lot::Mutex::new(Vec::new()));

    fn schedule_chain(
        q: TaskQueue,
        depth: usize,
        max_depth: usize,
        mock_clock: Arc<MockClock>,
        log: Arc<parking_lot::Mutex<Vec<(usize, u64)>>>,
    ) {
        let q_clone = q.clone();
        let clock_clone = Arc::clone(&mock_clock);
        let log_clone = Arc::clone(&log);

        q.schedule_timer(Duration::from_millis(0), move || {
            let now = clock_clone.now_ms();
            log_clone.lock().push((depth, now));

            if depth < max_depth {
                schedule_chain(q_clone, depth + 1, max_depth, clock_clone, log_clone);
            }
        });
    }

    // Schedule chain of depth 10 starting at t=1000ms
    schedule_chain(q.clone(), 1, 10, Arc::clone(&mock_clock), Arc::clone(&log));

    // Depth 1: t=1000ms
    assert!(el.step());
    // Depth 2: t=1000ms
    assert!(el.step());
    // Depth 3: t=1000ms
    assert!(el.step());
    // Depth 4: t=1000ms
    assert!(el.step());

    // Depth 5: Scheduled from Depth 4 (nesting=4 -> timer nesting=5 >= 5 -> clamped to 4ms)
    // Target is 1000 + 4 = 1004ms
    assert!(!el.step());

    // Advance 3ms -> t=1003ms (still not ready)
    mock_clock.advance_millis(3);
    assert!(!el.step());

    // Advance 1ms -> t=1004ms (fires!)
    mock_clock.advance_millis(1);
    assert!(el.step());

    // Depth 6: Scheduled from Depth 5 (nesting=5 -> timer nesting=6 >= 5 -> clamped to 4ms)
    // Target is 1004 + 4 = 1008ms
    assert!(!el.step());
    mock_clock.advance_millis(4);
    assert!(el.step());

    // Depth 7: Target 1008 + 4 = 1012ms
    assert!(!el.step());
    mock_clock.advance_millis(4);
    assert!(el.step());

    // Depth 8: Target 1012 + 4 = 1016ms
    assert!(!el.step());
    mock_clock.advance_millis(4);
    assert!(el.step());

    // Depth 9: Target 1016 + 4 = 1020ms
    assert!(!el.step());
    mock_clock.advance_millis(4);
    assert!(el.step());

    // Depth 10: Target 1020 + 4 = 1024ms
    assert!(!el.step());
    mock_clock.advance_millis(4);
    assert!(el.step());

    let entries = log.lock().clone();
    assert_eq!(entries.len(), 10);
    assert_eq!(
        entries,
        vec![
            (1, 1000),
            (2, 1000),
            (3, 1000),
            (4, 1000),
            (5, 1004),
            (6, 1008),
            (7, 1012),
            (8, 1016),
            (9, 1020),
            (10, 1024),
        ]
    );

    // Outside timers, nesting level is 0
    assert_eq!(current_timer_nesting(), 0);
}

#[test]
fn test_adversarial_timer_explicit_large_delay_at_depth_5() {
    let mock_clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();

    let ran = Arc::new(AtomicBool::new(false));
    let ran_clone = Arc::clone(&ran);

    // Schedule timer with nesting 5 and explicit delay of 50ms
    q.schedule_timer_with_nesting(Duration::from_millis(50), 5, move || {
        ran_clone.store(true, Ordering::SeqCst);
    });

    // Advance 4ms: should NOT run because delay was 50ms (not clamped down to 4ms)
    mock_clock.advance_millis(4);
    assert!(!el.step());
    assert!(!ran.load(Ordering::SeqCst));

    // Advance 46ms -> 50ms: now it runs!
    mock_clock.advance_millis(46);
    assert!(el.step());
    assert!(ran.load(Ordering::SeqCst));
}

// =========================================================================
// 5. CONCURRENT MULTITHREADED STRESS & SCOPE INVALIDATION
// =========================================================================

#[test]
fn test_adversarial_multithreaded_stress_with_cancellation_and_scopes() {
    let mock_clock = Arc::new(MockClock::new(0));
    let el = Arc::new(EventLoop::with_clock(
        Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>,
    ));
    let q = el.handle();

    let completed_tasks = Arc::new(AtomicUsize::new(0));
    let cancelled_tasks = Arc::new(AtomicUsize::new(0));

    let num_threads = 8;
    let iterations_per_thread = 200;

    let handles: Vec<_> = (0..num_threads)
        .map(|tid| {
            let q = q.clone();
            let c_comp = Arc::clone(&completed_tasks);
            let c_canc = Arc::clone(&cancelled_tasks);
            std::thread::spawn(move || {
                for i in 0..iterations_per_thread {
                    let task_type = (tid * 13 + i) % 6;
                    match task_type {
                        0 => {
                            let c = Arc::clone(&c_comp);
                            q.queue_user_interaction(move || {
                                c.fetch_add(1, Ordering::SeqCst);
                            });
                        }
                        1 => {
                            let c = Arc::clone(&c_comp);
                            q.queue_network(move || {
                                c.fetch_add(1, Ordering::SeqCst);
                            });
                        }
                        2 => {
                            let c = Arc::clone(&c_comp);
                            q.queue_microtask(move || {
                                c.fetch_add(1, Ordering::SeqCst);
                            });
                        }
                        3 => {
                            let (scope, scoped_q) = q.create_scope();
                            let c = Arc::clone(&c_comp);
                            let sc_res = scoped_q.queue_dom(move || {
                                c.fetch_add(1, Ordering::SeqCst);
                            });
                            if i % 2 == 0 {
                                scope.invalidate();
                            }
                            let _ = sc_res;
                        }
                        4 => {
                            let c = Arc::clone(&c_comp);
                            let (_id, cancel) = q.queue_timer(move || {
                                c.fetch_add(1, Ordering::SeqCst);
                            });
                            if i % 3 == 0 {
                                cancel.store(true, Ordering::SeqCst);
                                c_canc.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                        _ => {
                            let c = Arc::clone(&c_comp);
                            q.schedule_timer(Duration::from_millis(0), move || {
                                c.fetch_add(1, Ordering::SeqCst);
                            });
                        }
                    }
                }
            })
        })
        .collect();

    for h in handles {
        h.join().expect("Worker thread panicked!");
    }

    // Advance clock to trigger all timers
    mock_clock.advance_millis(100);

    // Run the event loop until drained
    let mut steps = 0;
    while el.step() && steps < 50_000 {
        steps += 1;
    }

    let completed = completed_tasks.load(Ordering::SeqCst);
    let cancelled = cancelled_tasks.load(Ordering::SeqCst);

    assert!(
        completed > 0,
        "No tasks completed during multithreaded stress!"
    );
    assert!(
        completed + cancelled <= num_threads * iterations_per_thread,
        "Completed ({}) + cancelled ({}) exceeded total spawned tasks ({})",
        completed,
        cancelled,
        num_threads * iterations_per_thread
    );

    // Invariant: Microtask checkpoint flag must be false
    assert!(!el.is_performing_microtask_checkpoint());
    // Invariant: current timer nesting must be 0
    assert_eq!(current_timer_nesting(), 0);
}
