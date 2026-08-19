use ace_core::event_loop::{current_timer_nesting, EventLoop, TaskQueue, TaskSource};
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

    // Submete 25.000 microtasks simultâneas
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

    // Em t = 150ms: apenas o de 100ms deve ter expirado e executado
    mock_clock.advance_millis(150);
    assert!(el.step());
    assert_eq!(*fired_timers.lock(), vec![100]);

    // Em t = 350ms: os de 200ms e 300ms expiram na ordem correta
    mock_clock.advance_millis(200);
    assert!(el.step());
    assert_eq!(*fired_timers.lock(), vec![100, 200]);

    assert!(el.step());
    assert_eq!(*fired_timers.lock(), vec![100, 200, 300]);
    assert!(!el.step());
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
    assert!(!el.step());
    assert!(!fired.load(Ordering::SeqCst));

    // Em t = 4ms: dispara
    mock_clock.advance_millis(2);
    assert!(el.step());
    assert!(fired.load(Ordering::SeqCst));

    // Throttling em background (mínimo 1000ms)
    q.set_background_throttling(true);
    let bg_fired = Arc::new(AtomicBool::new(false));
    let bg_clone = Arc::clone(&bg_fired);
    q.schedule_timer(Duration::from_millis(10), move || {
        bg_clone.store(true, Ordering::SeqCst);
    });

    mock_clock.advance_millis(500);
    assert!(!el.step());
    assert!(!bg_fired.load(Ordering::SeqCst));

    mock_clock.advance_millis(500);
    assert!(el.step());
    assert!(bg_fired.load(Ordering::SeqCst));
}

/// WHATWG §8.1.6: Prevenção de Starvation sob fluxo contínuo de tarefas de alta prioridade.
#[test]
fn test_starvation_prevention_under_continuous_ui_load() {
    let mock_clock = Arc::new(MockClock::new(0));
    // Limite de starvation = 4 para validação rápida determinística
    let el = EventLoop::with_clock_and_starvation_limit(
        Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>,
        4,
    );
    let q = el.handle();

    let network_executed = Arc::new(AtomicBool::new(false));
    let net_clone = Arc::clone(&network_executed);

    // Enfileira 1 tarefa de rede (menor prioridade que UserInteraction)
    q.queue_network(move || {
        net_clone.store(true, Ordering::SeqCst);
    });

    let mut ui_executed_count = 0;

    // Simula saturação contínua de UserInteraction
    // O escalonador com Starvation Limit = 4 deve despachar a tarefa de rede em no máximo 5 passos
    for _ in 0..10 {
        let (task_id, _cancel) = q.queue_user_interaction(|| {});
        let _ = task_id;

        if el.step() {
            if network_executed.load(Ordering::SeqCst) {
                break;
            }
            ui_executed_count += 1;
        }
    }

    // A tarefa de rede DEVE ter executado antes de sofrer inanição
    assert!(
        network_executed.load(Ordering::SeqCst),
        "A tarefa de rede sofreu starvation sob fluxo contínuo de UI!"
    );
    // Verificamos que a tarefa de rede foi despachada dentro do threshold (<= 4 tarefas de UI)
    assert!(
        ui_executed_count <= 4,
        "Tarefas de UI executaram mais vezes ({ui_executed_count}) do que o limite de starvation (4)"
    );
}

/// WHATWG §8.1.6.3: Microtask Checkpoint Reentrancy Guard contra recursão infinita.
#[test]
fn test_microtask_checkpoint_reentrancy_guard() {
    let el = Arc::new(EventLoop::new());
    let q = el.handle();

    let inner_drain_result = Arc::new(AtomicUsize::new(999));
    let microtask2_ran = Arc::new(AtomicBool::new(false));

    let el_clone = Arc::clone(&el);
    let inner_res = Arc::clone(&inner_drain_result);
    let m2_clone = Arc::clone(&microtask2_ran);
    let q_clone = q.clone();

    // Microtask 1 tenta invocar reentrantemente drain_microtasks()
    q.queue_microtask(move || {
        // Enfileira Microtask 2 dentro da execução do microtask checkpoint
        let m2_inner = Arc::clone(&m2_clone);
        q_clone.queue_microtask(move || {
            m2_inner.store(true, Ordering::SeqCst);
        });

        // O Reentrancy Guard deve detectar que performing_microtask_checkpoint == true
        // e retornar 0 imediatamente (Passo 1 da WHATWG §8.1.6.3)
        let reentrant_drained = el_clone.drain_microtasks();
        inner_res.store(reentrant_drained, Ordering::SeqCst);
    });

    assert!(!el.is_performing_microtask_checkpoint());
    let total_drained = el.drain_microtasks();

    // Reentrância retornou 0
    assert_eq!(inner_drain_result.load(Ordering::SeqCst), 0);
    // O checkpoint externo drenou ambas as microtasks (M1 e M2)
    assert_eq!(total_drained, 2);
    assert!(microtask2_ran.load(Ordering::SeqCst));
    assert!(!el.is_performing_microtask_checkpoint());
}

/// WHATWG §8.1.6: Despacho atômico de temporizadores com Microtask Checkpoint intercalado.
#[test]
fn test_interleaved_timer_macrotasks_and_microtasks() {
    let mock_clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();

    let execution_log = Arc::new(parking_lot::Mutex::new(Vec::new()));

    let log1 = Arc::clone(&execution_log);
    let q1 = q.clone();
    // Timer 1 dispara em t=100ms e enfileira uma Microtask
    q.schedule_timer(Duration::from_millis(100), move || {
        log1.lock().push("timer1");
        let log_micro = Arc::clone(&log1);
        q1.queue_microtask(move || {
            log_micro.lock().push("microtask_from_timer1");
        });
    });

    let log2 = Arc::clone(&execution_log);
    // Timer 2 dispara em t=100ms (mesmo instante de Timer 1)
    q.schedule_timer(Duration::from_millis(100), move || {
        log2.lock().push("timer2");
    });

    // Avança relógio para expirar ambos os timers
    mock_clock.advance_millis(100);

    // Passo 1: Executa Timer 1 Macrotask + Microtask Checkpoint (executa microtask de Timer 1)
    assert!(el.step());
    assert_eq!(
        *execution_log.lock(),
        vec!["timer1", "microtask_from_timer1"]
    );

    // Passo 2: Executa Timer 2 Macrotask
    assert!(el.step());
    assert_eq!(
        *execution_log.lock(),
        vec!["timer1", "microtask_from_timer1", "timer2"]
    );

    assert!(!el.step());
}

/// WHATWG §8.5.2: Clamping dinâmico de temporizadores recursivos aninhados (>= 5 -> min 4ms).
#[test]
fn test_dynamic_recursive_timer_nesting_clamping() {
    let mock_clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();

    let nesting_records = Arc::new(parking_lot::Mutex::new(Vec::new()));

    // Função recursiva simulando setTimeout(..., 0) aninhado 6 vezes
    fn schedule_recursive_level(
        q: TaskQueue,
        current_depth: usize,
        max_depth: usize,
        records: Arc<parking_lot::Mutex<Vec<(usize, usize)>>>,
    ) {
        let q_clone = q.clone();
        let records_clone = Arc::clone(&records);

        q.schedule_timer(Duration::from_millis(0), move || {
            let active_nesting = current_timer_nesting();
            records_clone.lock().push((current_depth, active_nesting));

            if current_depth < max_depth {
                schedule_recursive_level(
                    q_clone,
                    current_depth + 1,
                    max_depth,
                    records_clone,
                );
            }
        });
    }

    // Dispara cadeia recursiva até profundidade 6
    schedule_recursive_level(q.clone(), 1, 6, Arc::clone(&nesting_records));

    // Nível 1: t=0ms
    assert!(el.step());
    assert_eq!(*nesting_records.lock(), vec![(1, 1)]);

    // Nível 2: t=0ms
    assert!(el.step());
    assert_eq!(*nesting_records.lock(), vec![(1, 1), (2, 2)]);

    // Nível 3: t=0ms
    assert!(el.step());
    assert_eq!(*nesting_records.lock(), vec![(1, 1), (2, 2), (3, 3)]);

    // Nível 4: t=0ms
    assert!(el.step());
    assert_eq!(
        *nesting_records.lock(),
        vec![(1, 1), (2, 2), (3, 3), (4, 4)]
    );

    // Nível 5: foi agendado a partir do callback de Nível 4 (nesting=4 -> novo timer tem nesting=5 >= 5)
    // O delay 0ms foi clampado para 4ms!
    // Em t=0ms, step() não deve executar o timer de nível 5
    assert!(!el.step());

    // Avança 2ms (t=2ms): ainda não deve disparar
    mock_clock.advance_millis(2);
    assert!(!el.step());

    // Avança mais 2ms (t=4ms): Nível 5 dispara!
    mock_clock.advance_millis(2);
    assert!(el.step());
    assert_eq!(
        *nesting_records.lock(),
        vec![(1, 1), (2, 2), (3, 3), (4, 4), (5, 5)]
    );

    // Nível 6: agendado a partir de Nível 5 (nesting=5 -> novo timer tem nesting=6 >= 5, delay=4ms)
    assert!(!el.step());
    mock_clock.advance_millis(4);
    assert!(el.step());
    assert_eq!(
        *nesting_records.lock(),
        vec![(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)]
    );

    // Fora do callback do timer, o nesting na thread retorna para 0
    assert_eq!(current_timer_nesting(), 0);
}

/// Starvation Prevention e Fair Queuing com todas as 7 fontes de tarefas do WHATWG.
#[test]
fn test_fair_queuing_all_task_sources() {
    let mock_clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock_and_starvation_limit(
        Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>,
        3,
    );
    let q = el.handle();

    let executed_sources = Arc::new(parking_lot::Mutex::new(Vec::new()));

    for &src in &TaskSource::ALL_SOURCES {
        let exec = Arc::clone(&executed_sources);
        q.queue_task(src, move || {
            exec.lock().push(src);
        });
    }

    // Executa continuamente até drenar todas as 7 fontes
    let mut steps = 0;
    while el.step() && steps < 20 {
        steps += 1;
    }

    let recorded = executed_sources.lock().clone();
    assert_eq!(recorded.len(), 7);
    // Todas as 7 fontes executaram sem perdas
    for &src in &TaskSource::ALL_SOURCES {
        assert!(recorded.contains(&src), "Fonte {:?} não foi executada!", src);
    }
}

/// WHATWG §8.1.6.3: Microtasks encadeadas em profundidade são todas drenadas no mesmo checkpoint.
#[test]
fn test_chained_microtasks_in_single_checkpoint() {
    let el = EventLoop::new();
    let q = el.handle();
    let steps_log = Arc::new(parking_lot::Mutex::new(Vec::new()));

    let log1 = Arc::clone(&steps_log);
    let q1 = q.clone();

    // Macrotask inicial
    q.queue_user_interaction(move || {
        log1.lock().push("macrotask_start");

        let log2 = Arc::clone(&log1);
        let q2 = q1.clone();
        // Microtask Nível 1
        q1.queue_microtask(move || {
            log2.lock().push("microtask_lvl1");

            let log3 = Arc::clone(&log2);
            let q3 = q2.clone();
            // Microtask Nível 2
            q2.queue_microtask(move || {
                log3.lock().push("microtask_lvl2");

                let log4 = Arc::clone(&log3);
                // Microtask Nível 3
                q3.queue_microtask(move || {
                    log4.lock().push("microtask_lvl3");
                });
            });
        });
    });

    // Passo 1: Macrotask + drenagem completa de toda a cadeia de microtasks (níveis 1, 2 e 3)
    assert!(el.step());
    assert_eq!(
        *steps_log.lock(),
        vec![
            "macrotask_start",
            "microtask_lvl1",
            "microtask_lvl2",
            "microtask_lvl3"
        ]
    );

    // Passo 2: Sem mais tarefas
    assert!(!el.step());
}

/// WHATWG §8.1.6.3: RAII guard restaura performing_microtask_checkpoint mesmo se a microtask sofrer panic.
#[test]
fn test_microtask_panic_recovers_checkpoint_flag() {
    let el = Arc::new(EventLoop::new());
    let q = el.handle();

    let after_panic_executed = Arc::new(AtomicBool::new(false));
    let after_panic_clone = Arc::clone(&after_panic_executed);

    // Microtask que causa pânico
    q.queue_microtask(|| {
        panic!("Erro intencional de teste na microtask");
    });

    // Captura o pânico
    let el_clone = Arc::clone(&el);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        el_clone.drain_microtasks();
    }));
    assert!(result.is_err());

    // Flag DEVE ter sido restaurada pelo RAII guard
    assert!(!el.is_performing_microtask_checkpoint());

    // Novas microtasks conseguem ser processadas normalmente
    q.queue_microtask(move || {
        after_panic_clone.store(true, Ordering::SeqCst);
    });

    assert_eq!(el.drain_microtasks(), 1);
    assert!(after_panic_executed.load(Ordering::SeqCst));
    assert!(!el.is_performing_microtask_checkpoint());
}

/// Cancelamento de temporizador após expirar no relógio, mas antes do step() executar.
#[test]
fn test_expired_timer_cancellation_before_execution() {
    let mock_clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();

    let executed = Arc::new(AtomicBool::new(false));
    let exec_clone = Arc::clone(&executed);

    let (_id, cancel_handle) = q.schedule_timer(Duration::from_millis(50), move || {
        exec_clone.store(true, Ordering::SeqCst);
    });

    // Avança relógio para expirar o timer
    mock_clock.advance_millis(60);

    // Cancela o timer antes do Event Loop processá-lo
    cancel_handle.store(true, Ordering::SeqCst);

    // Executa step: o timer é promovido e descartado por cancelamento
    assert!(el.step());
    assert!(!executed.load(Ordering::SeqCst));
    assert!(!el.step());
}

/// Concorrência e thread-safety: múltiplas threads submetendo tarefas simultaneamente.
#[test]
fn test_concurrent_multithreaded_task_queuing() {
    let el = Arc::new(EventLoop::new());
    let counter = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..8)
        .map(|i| {
            let q = el.handle();
            let c = Arc::clone(&counter);
            std::thread::spawn(move || {
                for _ in 0..100 {
                    let c_clone = Arc::clone(&c);
                    match i % 4 {
                        0 => {
                            q.queue_user_interaction(move || {
                                c_clone.fetch_add(1, Ordering::SeqCst);
                            });
                        }
                        1 => {
                            q.queue_dom(move || {
                                c_clone.fetch_add(1, Ordering::SeqCst);
                            });
                        }
                        2 => {
                            q.queue_network(move || {
                                c_clone.fetch_add(1, Ordering::SeqCst);
                            });
                        }
                        _ => {
                            q.queue_microtask(move || {
                                c_clone.fetch_add(1, Ordering::SeqCst);
                            });
                        }
                    }
                }
            })
        })
        .collect();

    for h in handles {
        h.join().expect("Thread join falhou");
    }

    // Drena tudo pelo Event Loop
    let mut total_steps = 0;
    while el.step() && total_steps < 5000 {
        total_steps += 1;
    }

    // 8 threads * 100 tarefas = 800 tarefas executadas
    assert_eq!(counter.load(Ordering::SeqCst), 800);
}

