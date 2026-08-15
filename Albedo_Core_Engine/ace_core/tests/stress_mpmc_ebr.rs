// ============================================================================
// Albedo Core Engine (ACE)
// File: stress_mpmc_ebr.rs
// Description: Testes extremos de contenção para MPMC Queue e EBR Lock-Free.
// Author: Albedo Browser Engineering Team
// ============================================================================

use ace_core::mpmc::ArrayQueue;
use ace_core::ebr::{AtomicEbr, Guard};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

const NUM_PRODUCERS: usize = 16;
const NUM_CONSUMERS: usize = 16;
const ITEMS_PER_PRODUCER: usize = 200_000;

#[test]
fn test_stress_mpmc_extreme_contention() {
    let queue = Arc::new(ArrayQueue::<usize>::new(65536));
    let barrier = Arc::new(Barrier::new(NUM_PRODUCERS + NUM_CONSUMERS));
    let consumed_count = Arc::new(AtomicUsize::new(0));
    
    let mut handles = vec![];

    let start = Instant::now();

    // Spawn consumers
    for _ in 0..NUM_CONSUMERS {
        let q = queue.clone();
        let b = barrier.clone();
        let count = consumed_count.clone();
        handles.push(thread::spawn(move || {
            b.wait();
            let mut local_count = 0;
            // Espera receber itens até o total esperado (NUM_PRODUCERS * ITEMS_PER_PRODUCER)
            let total_expected = NUM_PRODUCERS * ITEMS_PER_PRODUCER;
            while count.load(Ordering::Relaxed) < total_expected {
                if let Some(_) = q.pop() {
                    count.fetch_add(1, Ordering::Relaxed);
                    local_count += 1;
                }
            }
            local_count
        }));
    }

    // Spawn producers
    for p in 0..NUM_PRODUCERS {
        let q = queue.clone();
        let b = barrier.clone();
        handles.push(thread::spawn(move || {
            b.wait();
            for i in 0..ITEMS_PER_PRODUCER {
                let item = (p * ITEMS_PER_PRODUCER) + i;
                while q.push(item).is_err() {
                    // Queue full, spin
                    core::hint::spin_loop();
                }
            }
            0
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let elapsed = start.elapsed();
    let total_processed = consumed_count.load(Ordering::SeqCst);
    let expected = NUM_PRODUCERS * ITEMS_PER_PRODUCER;
    
    assert_eq!(total_processed, expected, "A Fila MPMC perdeu dados!");
    println!("MPMC: Processou {} itens em {:?} ({} M/s)", 
        total_processed, 
        elapsed, 
        (total_processed as f64 / elapsed.as_secs_f64()) / 1_000_000.0
    );
}

#[test]
fn test_stress_ebr_memory_leak() {
    // Testa se o EBR realmente liberta memória ou se causa vazamento/deadlock
    static DROPPED_COUNT: AtomicUsize = AtomicUsize::new(0);

    struct Node {
        _data: [u8; 1024], // 1KB de dados
    }

    impl Drop for Node {
        fn drop(&mut self) {
            DROPPED_COUNT.fetch_add(1, Ordering::Relaxed);
        }
    }

    let pointer = Arc::new(AtomicEbr::new(Node { _data: [0; 1024] }));
    let barrier = Arc::new(Barrier::new(16));
    
    let mut handles = vec![];

    let start = Instant::now();

    for i in 0..16 {
        let ptr = pointer.clone();
        let b = barrier.clone();
        handles.push(thread::spawn(move || {
            b.wait();
            for _ in 0..5000 {
                let guard = Guard::pin();
                // Lê o valor atual
                let _node = ptr.load(&guard);
                
                // Algumas threads tentam substituir o ponteiro por um novo
                if i % 2 == 0 {
                    ptr.swap(Some(Node { _data: [1; 1024] }), &guard);
                }
                
                // Força chamadas de flush ocasionais para tentar coletar lixo local
                if i % 100 == 0 {
                    guard.flush();
                }
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // A thread principal deve tentar forçar um flush final para limpar tudo
    let guard = Guard::pin();
    pointer.defer_destroy(&guard);
    drop(guard);
    
    // Força o avanço global lendo e pinando repetidamente em uma thread isolada 
    // ou apenas chamando flush várias vezes.
    for _ in 0..10 {
        let guard = Guard::pin();
        guard.flush();
        thread::sleep(Duration::from_millis(1));
    }

    let dropped = DROPPED_COUNT.load(Ordering::SeqCst);
    let elapsed = start.elapsed();
    
    println!("EBR: Limpou {} nós com sucesso em {:?}", dropped, elapsed);
    assert!(dropped > 10000, "O EBR não descartou memória suficiente! Possível Leak.");
}
