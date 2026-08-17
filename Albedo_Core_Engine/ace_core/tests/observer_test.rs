use ace_core::observer::ObserverList;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn test_observer_list_basic_notifications() {
    let list = ObserverList::<String>::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let c = Arc::clone(&counter);
    let id1 = list.register(move |_msg| {
        c.fetch_add(1, Ordering::SeqCst);
    });

    let c2 = Arc::clone(&counter);
    let id2 = list.register(move |_msg| {
        c2.fetch_add(10, Ordering::SeqCst);
    });

    list.notify(&"evento 1".to_string());
    assert_eq!(counter.load(Ordering::SeqCst), 11);

    // Remove 1 observador
    assert!(list.unregister(id1));
    list.notify(&"evento 2".to_string());
    assert_eq!(counter.load(Ordering::SeqCst), 21); // Apenas +10

    assert!(list.unregister(id2));
    assert_eq!(list.len(), 0);
}

#[test]
fn test_observer_list_reentrant_mutation() {
    let list = Arc::new(ObserverList::<usize>::new());
    let triggered = Arc::new(AtomicUsize::new(0));

    let list_clone = Arc::clone(&list);
    let tr_clone = Arc::clone(&triggered);

    // Observador 1 adiciona o Observador 2 DURANTE a notificação (Reentrância)
    list.register(move |val| {
        tr_clone.fetch_add(*val, Ordering::SeqCst);

        let tr_inner = Arc::clone(&tr_clone);
        list_clone.register(move |v2| {
            tr_inner.fetch_add(*v2 * 100, Ordering::SeqCst);
        });
    });

    // Primeira notificação: executa o observador 1 (adicionando o 2)
    list.notify(&1);
    assert_eq!(triggered.load(Ordering::SeqCst), 1);
    assert_eq!(list.len(), 2);

    // Segunda notificação: executa ambos (1 e 2) sem deadlock
    list.notify(&1);
    // +1 (do obs 1) + 100 (do obs 2 criado no passo 1) = +101
    assert_eq!(triggered.load(Ordering::SeqCst), 102);
}
