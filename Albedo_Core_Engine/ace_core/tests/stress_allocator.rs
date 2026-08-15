// ============================================================================
// Albedo Core Engine (ACE)
// File: stress_allocator.rs
// Description: Teste de Stress para o TLAC (Thread-Local Allocation Caching).
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

#[test]
fn test_stress_tlac_multithread() {
    let num_threads = 32;
    let allocations_per_thread = 50_000;

    let start_signal = Arc::new(AtomicBool::new(false));
    let mut handles = vec![];

    let start = Instant::now();

    for _ in 0..num_threads {
        let signal = start_signal.clone();
        handles.push(thread::spawn(move || {
            // Spin-wait for simultaneous start
            while !signal.load(Ordering::Acquire) {
                core::hint::spin_loop();
            }

            let mut v = Vec::new();
            for i in 0..allocations_per_thread {
                // Alocando no Heap (ativa o malloc interceptado)
                v.push(vec![i; 64]);
            }
            // Deallocate all
            drop(v);
        }));
    }

    // Fire!
    start_signal.store(true, Ordering::Release);

    for handle in handles {
        handle.join().unwrap();
    }

    let elapsed = start.elapsed();
    let total_allocations = num_threads * allocations_per_thread;

    // Mostra o throughput. Com o TLAC, isso deve ser absurdamente rápido
    // pois reduzimos a contenção no Atômico Global do `alloc.rs`
    println!(
        "TLAC: {} alocações paralelas completadas em {:?}",
        total_allocations, elapsed
    );
    assert!(
        elapsed.as_millis() < 5000,
        "Alocação massiva demorou demais. TLAC falhou em escalar."
    );
}
