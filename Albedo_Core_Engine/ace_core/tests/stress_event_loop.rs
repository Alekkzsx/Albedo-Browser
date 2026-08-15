// ============================================================================
// Albedo Core Engine (ACE)
// File: stress_event_loop.rs
// Description: Teste de Stress para o Circuit Breaker do Event Loop.
// Author: Albedo Browser Engineering Team
// ============================================================================

use ace_core::event_loop::EventLoop;
use ace_core::io::NativeMultiplexer;
use ace_core::thread_pool::ThreadPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Instant, Duration};

#[test]
fn test_stress_event_loop_circuit_breaker() {
    let pool = Arc::new(ThreadPool::new(1));
    let mut event_loop = EventLoop::new(NativeMultiplexer::new(), pool.clone());

    let execution_count = Arc::new(AtomicUsize::new(0));

    // Simulamos um loop de Microtasks infinito/gigantesco (ex: Promise que se resolve recursivamente)
    for _ in 0..1_000_000 {
        let count = execution_count.clone();
        event_loop.queue_microtask(Box::new(move || {
            count.fetch_add(1, Ordering::Relaxed);
            // Gasta um tempo artificial na microtask
            let start = Instant::now();
            while start.elapsed() < Duration::from_micros(10) {
                core::hint::spin_loop();
            }
        }));
    }

    // Marca o início
    let start_time = Instant::now();

    // Rodamos UM único ciclo do event loop
    event_loop.run_once().unwrap();

    let elapsed = start_time.elapsed();
    let executed = execution_count.load(Ordering::Relaxed);
    
    println!("EventLoop: Abortou o ciclo após executar {} microtasks em {:?}", executed, elapsed);
    
    // O Circuit Breaker do motor deve interromper o loop em aprox. 5ms!
    // Sem ele, o teste rodaria por >10 segundos (1 milhão * 10 micros).
    assert!(executed < 1_000_000, "Circuit Breaker falhou! Drenou toda a fila infinita.");
    assert!(elapsed.as_millis() < 50, "O tempo de quebra estourou os limites do VSync!");
}
