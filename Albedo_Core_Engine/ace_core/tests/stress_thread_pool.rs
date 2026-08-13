use ace_core::thread_pool::ThreadPool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn test_stress_thread_pool_millions_of_tasks() {
    let pool = ThreadPool::new(4);
    let counter = Arc::new(AtomicUsize::new(0));

    // Injeta 1 Milhão de micro-tarefas para testar contenção e work-stealing
    const TASK_COUNT: usize = 1_000_000;

    for _ in 0..TASK_COUNT {
        let c = Arc::clone(&counter);
        pool.execute(move || {
            c.fetch_add(1, Ordering::Relaxed);
        });
    }

    pool.wait_for_all();

    assert_eq!(counter.load(Ordering::SeqCst), TASK_COUNT);
}

#[test]
fn test_stress_thread_pool_layout_phases() {
    let pool = ThreadPool::new(8); // Máximo stress de threads
    let counter = Arc::new(AtomicUsize::new(0));

    // Simula 100 frames (Layout Phases)
    for _ in 0..100 {
        // Cada phase lança 10.000 tarefas
        for _ in 0..10_000 {
            let c = Arc::clone(&counter);
            pool.execute(move || {
                // Matemática simples para simular processamento
                let mut x = 0.0;
                for i in 0..10 {
                    x += (i as f64).sin();
                }
                if x > 1000.0 {
                    c.fetch_add(1, Ordering::Relaxed);
                }

                c.fetch_add(1, Ordering::Relaxed);
            });
        }

        // Barreira absoluta: garante que as 10.000 terminaram antes do próximo frame
        pool.wait_for_all();
    }

    assert_eq!(counter.load(Ordering::SeqCst), 1_000_000); // 100 * 10_000
}
