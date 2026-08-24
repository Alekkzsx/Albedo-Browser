use ace_core::collections::InlineVec;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct DropTracker {
    id: usize,
    drop_count: Arc<AtomicUsize>,
}

impl DropTracker {
    fn new(id: usize, drop_count: Arc<AtomicUsize>) -> Self {
        Self { id, drop_count }
    }
}

impl Drop for DropTracker {
    fn drop(&mut self) {
        self.drop_count.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_inline_vec_stack_and_spill_to_heap() {
    let mut vec: InlineVec<String, 4> = InlineVec::new();
    assert!(vec.is_empty());
    assert_eq!(vec.len(), 0);
    assert!(vec.is_inline());

    vec.push("item1".to_string());
    vec.push("item2".to_string());
    vec.push("item3".to_string());
    vec.push("item4".to_string());

    assert_eq!(vec.len(), 4);
    assert!(vec.is_inline()); // Ainda na stack
    assert_eq!(vec[0], "item1");
    assert_eq!(vec[3], "item4");

    // 5º item força o spill para o Heap
    vec.push("item5".to_string());
    assert_eq!(vec.len(), 5);
    assert!(!vec.is_inline()); // Agora no Heap!
    assert_eq!(vec[4], "item5");

    // Pop
    assert_eq!(vec.pop(), Some("item5".to_string()));
    assert_eq!(vec.len(), 4);

    // Iteração e fatiamento
    let items: Vec<&str> = vec.iter().map(|s| s.as_str()).collect();
    assert_eq!(items, vec!["item1", "item2", "item3", "item4"]);

    // Clone
    let cloned = vec.clone();
    assert_eq!(cloned, vec);

    vec.clear();
    assert!(vec.is_empty());
}

#[test]
fn test_inline_vec_insert_at_zero_mid_len() {
    let mut vec: InlineVec<String, 6> = InlineVec::new();
    // Inserção em vetor vazio (index = 0 == len)
    vec.insert(0, "c".to_string());
    assert_eq!(vec.as_slice(), &["c"]);
    assert!(vec.is_inline());

    // Inserção no início (index = 0 < len)
    vec.insert(0, "a".to_string());
    assert_eq!(vec.as_slice(), &["a", "c"]);
    assert!(vec.is_inline());

    // Inserção no meio (index = 1)
    vec.insert(1, "b".to_string());
    assert_eq!(vec.as_slice(), &["a", "b", "c"]);
    assert!(vec.is_inline());

    // Inserção no final (index = len)
    vec.insert(3, "e".to_string());
    assert_eq!(vec.as_slice(), &["a", "b", "c", "e"]);
    assert!(vec.is_inline());

    // Inserção antes do final (index = 3)
    vec.insert(3, "d".to_string());
    assert_eq!(vec.as_slice(), &["a", "b", "c", "d", "e"]);
    assert!(vec.is_inline());
}

#[test]
fn test_inline_vec_insert_spill_to_heap() {
    let mut vec: InlineVec<i32, 3> = InlineVec::new();
    vec.push(10);
    vec.push(30);
    vec.push(40);
    assert_eq!(vec.len(), 3);
    assert!(vec.is_inline());

    // Inserção com capacidade cheia causa transição para o Heap
    vec.insert(1, 20);
    assert_eq!(vec.len(), 4);
    assert!(!vec.is_inline());
    assert_eq!(vec.as_slice(), &[10, 20, 30, 40]);

    // Inserções subsequentes no Heap
    vec.insert(0, 0);
    vec.insert(5, 50);
    assert_eq!(vec.as_slice(), &[0, 10, 20, 30, 40, 50]);
}

#[test]
#[should_panic(expected = "índice de inserção fora dos limites")]
fn test_inline_vec_insert_out_of_bounds() {
    let mut vec: InlineVec<i32, 4> = InlineVec::new();
    vec.insert(1, 42);
}

#[test]
fn test_inline_vec_retain_non_copy_drop_count() {
    let drop_counter = Arc::new(AtomicUsize::new(0));

    {
        let mut vec: InlineVec<DropTracker, 6> = InlineVec::new();
        for i in 0..6 {
            vec.push(DropTracker::new(i, Arc::clone(&drop_counter)));
        }
        assert_eq!(vec.len(), 6);
        assert_eq!(drop_counter.load(Ordering::SeqCst), 0);

        // Retém apenas os índices pares (0, 2, 4). Elementos 1, 3, 5 devem ser dropados imediatamente!
        vec.retain(|item| item.id % 2 == 0);

        assert_eq!(vec.len(), 3);
        assert_eq!(drop_counter.load(Ordering::SeqCst), 3); // 3 itens descartados durante retain

        assert_eq!(vec[0].id, 0);
        assert_eq!(vec[1].id, 2);
        assert_eq!(vec[2].id, 4);
    }

    // Ao sair de escopo, os 3 itens restantes são dropados. Total = 6 drops.
    assert_eq!(drop_counter.load(Ordering::SeqCst), 6);
}

#[test]
fn test_inline_vec_retain_heap_mode() {
    let drop_counter = Arc::new(AtomicUsize::new(0));

    {
        let mut vec: InlineVec<DropTracker, 2> = InlineVec::new();
        // 4 itens forçam heap mode
        for i in 0..4 {
            vec.push(DropTracker::new(i, Arc::clone(&drop_counter)));
        }
        assert!(!vec.is_inline());
        assert_eq!(drop_counter.load(Ordering::SeqCst), 0);

        vec.retain(|item| item.id >= 2);
        assert_eq!(vec.len(), 2);
        assert_eq!(drop_counter.load(Ordering::SeqCst), 2); // 0 e 1 dropados

        assert_eq!(vec[0].id, 2);
        assert_eq!(vec[1].id, 3);
    }

    assert_eq!(drop_counter.load(Ordering::SeqCst), 4);
}

#[test]
fn test_inline_vec_retain_panic_safety() {
    let drop_counter = Arc::new(AtomicUsize::new(0));

    let result = std::panic::catch_unwind(|| {
        let mut vec: InlineVec<DropTracker, 6> = InlineVec::new();
        for i in 0..6 {
            vec.push(DropTracker::new(i, Arc::clone(&drop_counter)));
        }

        // Pânico no 3º elemento (i = 2)
        vec.retain(|item| {
            if item.id == 2 {
                panic!("intencional em retain");
            }
            item.id == 0
        });
    });

    assert!(result.is_err());
    // Mesmo com pânico, todos os 6 itens devem ter sido dropados sem Double Free ou vazamento
    assert_eq!(drop_counter.load(Ordering::SeqCst), 6);
}

#[test]
fn test_inline_vec_drain_and_truncate() {
    let mut vec: InlineVec<i32, 4> = InlineVec::new();
    vec.extend([10, 20, 30, 40]);

    assert_eq!(vec.first(), Some(&10));
    assert_eq!(vec.last(), Some(&40));
    assert_eq!(vec.get(2), Some(&30));

    // Drain partial range
    let drained = vec.drain(1..3);
    assert_eq!(drained, vec![20, 30]);
    assert_eq!(vec.as_slice(), &[10, 40]);

    // Truncate
    vec.truncate(1);
    assert_eq!(vec.as_slice(), &[10]);
    assert_eq!(vec.len(), 1);

    // Spill to heap and drain
    vec.extend([20, 30, 40, 50]);
    assert!(!vec.is_inline());
    let drained_heap = vec.drain(1..4);
    assert_eq!(drained_heap, vec![20, 30, 40]);
    assert_eq!(vec.as_slice(), &[10, 50]);
}

#[test]
fn test_inline_vec_remove_and_pop() {
    let mut vec: InlineVec<String, 4> = InlineVec::new();
    vec.push("a".to_string());
    vec.push("b".to_string());
    vec.push("c".to_string());

    assert_eq!(vec.remove(1), "b");
    assert_eq!(vec.as_slice(), &["a", "c"]);
    assert_eq!(vec.pop(), Some("c".to_string()));
    assert_eq!(vec.as_slice(), &["a"]);
    assert_eq!(vec.pop(), Some("a".to_string()));
    assert_eq!(vec.pop(), None);
    assert!(vec.is_empty());
}
