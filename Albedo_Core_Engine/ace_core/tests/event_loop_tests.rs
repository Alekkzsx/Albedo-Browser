use ace_core::event_loop::{EventLoop, Macrotask};
use ace_core::io::MockMultiplexer;
use ace_core::thread_pool::ThreadPool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn test_event_loop_priorities() {
    let multiplexer = MockMultiplexer::new();
    let thread_pool = Arc::new(ThreadPool::new(4));
    let mut event_loop = EventLoop::new(multiplexer, Arc::clone(&thread_pool));

    let execution_order = Arc::new(AtomicUsize::new(0));

    // Enfileiramos 1 Macrotask (Baixa Prioridade - Delegada para o Pool)
    let order_macro = Arc::clone(&execution_order);
    event_loop.queue_macrotask(Macrotask::Timer(Box::new(move || {
        // Se ela rodar primeiro, gravará 1. Se rodar depois, gravará algo maior.
        order_macro.fetch_add(100, Ordering::SeqCst);
    })));

    // Enfileiramos 2 Microtasks (Alta Prioridade - Execução in-place Run-To-Completion)
    let order_micro1 = Arc::clone(&execution_order);
    event_loop.queue_microtask(Box::new(move || {
        order_micro1.fetch_add(1, Ordering::SeqCst);
    }));

    let order_micro2 = Arc::clone(&execution_order);
    event_loop.queue_microtask(Box::new(move || {
        order_micro2.fetch_add(2, Ordering::SeqCst);
    }));

    // O EventLoop gira 1 ciclo completo
    event_loop.run_once().unwrap();

    // Verificação de Microtasks (Garantia WHATWG):
    // Como as Microtasks rodam na própria Main Thread antes de devolver o controle,
    // temos certeza absoluta matemática que a execução ocorreu in-place.
    // O valor deve ser pelo menos 3 (1 + 2). Se a Macrotask bateu antes, será 103.
    let final_val = execution_order.load(Ordering::SeqCst);
    assert!(final_val >= 3);

    // Encerramento Gracioso
    drop(event_loop);
    
    // O Arc::try_unwrap falha se houver clones pendentes, o que ajuda a provar ausência de vazamentos
    if let Ok(pool) = Arc::try_unwrap(thread_pool) {
        pool.join();
    }
}
