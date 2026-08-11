use ace_core::thread_pool::ThreadPool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn test_thread_pool_basic_execution() {
    let pool = ThreadPool::new(4);
    let counter = Arc::new(AtomicUsize::new(0));

    // Enfileira 10.000 microtasks instantâneas
    for _ in 0..10_000 {
        let clone = Arc::clone(&counter);
        pool.execute(move || {
            clone.fetch_add(1, Ordering::Relaxed);
        });
    }

    // O Drop do pool invocará `join()` automaticamente aqui, forçando a espera.
    drop(pool);

    // Se o motor for perfeito, exatamente 10.000 tarefas alteraram a memória concorrente.
    assert_eq!(counter.load(Ordering::Relaxed), 10_000);
}

#[test]
fn test_thread_pool_panic_survival() {
    let pool = ThreadPool::new(2);
    let counter = Arc::new(AtomicUsize::new(0));

    // Injetamos um pânico nuclear!
    pool.execute(|| {
        panic!("Boom! Testando resiliência da VM!");
    });

    // Garantimos que a thread não morreu e continuou operando o próximo job
    let c2 = Arc::clone(&counter);
    pool.execute(move || {
        c2.fetch_add(1, Ordering::Relaxed);
    });

    drop(pool);
    assert_eq!(counter.load(Ordering::Relaxed), 1); // 1 Sucesso, 1 Falha tratada
}
