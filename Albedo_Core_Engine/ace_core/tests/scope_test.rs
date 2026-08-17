use ace_core::event_loop::{EventLoop, TaskScope};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[test]
fn test_task_scope_invalidation_prevents_execution() {
    let event_loop = EventLoop::new();
    let task_queue = event_loop.handle();

    let counter = Arc::new(AtomicU32::new(0));

    let scope = TaskScope::new();
    let scoped_queue = scope.wrap_queue(task_queue.clone());

    let c1 = Arc::clone(&counter);
    scoped_queue.queue_user_interaction(move || {
        c1.fetch_add(1, Ordering::SeqCst);
    });

    let c2 = Arc::clone(&counter);
    scoped_queue.queue_network(move || {
        c2.fetch_add(10, Ordering::SeqCst);
    });

    // Invalida o escopo antes do step do Event Loop (simula fechamento de aba)
    scope.invalidate();

    // Tarefa após invalidação não deve nem ser agendada
    let c3 = Arc::clone(&counter);
    let post_invalidation_id = scoped_queue.queue_user_interaction(move || {
        c3.fetch_add(100, Ordering::SeqCst);
    });
    assert!(post_invalidation_id.is_none());

    // Executa os passos do Event Loop
    while event_loop.step() {}

    // Nenhuma das tarefas associadas ao escopo invalidado deve ter executado
    assert_eq!(counter.load(Ordering::SeqCst), 0);
}

#[test]
fn test_task_scope_normal_execution() {
    let event_loop = EventLoop::new();
    let task_queue = event_loop.handle();

    let counter = Arc::new(AtomicU32::new(0));
    let (scope, scoped_queue) = task_queue.create_scope();

    let c1 = Arc::clone(&counter);
    scoped_queue.queue_user_interaction(move || {
        c1.fetch_add(5, Ordering::SeqCst);
    });

    let c2 = Arc::clone(&counter);
    scoped_queue.queue_microtask(move || {
        c2.fetch_add(2, Ordering::SeqCst);
    });

    assert!(scope.is_active());
    event_loop.step();

    assert_eq!(counter.load(Ordering::SeqCst), 7);
}
