use ace_core::deque::WorkerDeque;
use std::sync::Arc;
use std::thread;

#[test]
fn test_deque_push_pop_local() {
    let deque = WorkerDeque::new();
    
    assert!(deque.push(10).is_ok());
    assert!(deque.push(20).is_ok());
    assert!(deque.push(30).is_ok());
    
    // Pop é LIFO
    assert_eq!(deque.pop(), Some(30));
    assert_eq!(deque.pop(), Some(20));
    assert_eq!(deque.pop(), Some(10));
    assert_eq!(deque.pop(), None);
}

#[test]
fn test_deque_steal() {
    let deque = WorkerDeque::new();
    
    assert!(deque.push(100).is_ok());
    assert!(deque.push(200).is_ok());
    
    // Steal é FIFO
    assert_eq!(deque.steal(), Some(100));
    
    // Sobrou o 200, que pode ser pego por pop
    assert_eq!(deque.pop(), Some(200));
    assert_eq!(deque.pop(), None);
    assert_eq!(deque.steal(), None);
}

#[test]
fn test_deque_concurrent_steal() {
    let deque = Arc::new(WorkerDeque::new());
    
    // Coloca 10.000 itens (A capacidade é 4096, então o push falha depois disso, 
    // vamos colocar 4000)
    for i in 0..4000 {
        assert!(deque.push(i).is_ok());
    }
    
    let mut handles = vec![];
    
    // 4 threads ladrões
    for _ in 0..4 {
        let deq_clone = Arc::clone(&deque);
        handles.push(thread::spawn(move || {
            let mut sum = 0;
            while let Some(val) = deq_clone.steal() {
                sum += val;
            }
            sum
        }));
    }
    
    // A thread dona faz pop
    let mut owner_sum = 0;
    while let Some(val) = deque.pop() {
        owner_sum += val;
    }
    
    let mut total = owner_sum;
    for h in handles {
        total += h.join().unwrap();
    }
    
    // A soma de 0 a 3999 é (3999 * 4000) / 2 = 7998000
    assert_eq!(total, 7998000);
}
