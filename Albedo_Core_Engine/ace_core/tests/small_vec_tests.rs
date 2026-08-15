use ace_core::small_vec::SmallVec;

#[test]
fn test_small_vec_inline() {
    let mut vec: SmallVec<i32, 4> = SmallVec::new();
    assert!(vec.is_empty());

    vec.push(10);
    vec.push(20);
    vec.push(30);
    vec.push(40);

    assert_eq!(vec.len(), 4);
    assert_eq!(&vec[..], &[10, 20, 30, 40]);
}

#[test]
fn test_small_vec_spill_to_heap() {
    let mut vec: SmallVec<i32, 2> = SmallVec::new();

    vec.push(1);
    vec.push(2);
    // Deve transicionar para Heap agora
    vec.push(3);
    vec.push(4);

    assert_eq!(vec.len(), 4);
    assert_eq!(&vec[..], &[1, 2, 3, 4]);
}

#[test]
fn test_small_vec_drop_strings() {
    // String no Rust aloca no Heap. Precisamos garantir que elas são derrubadas
    // mesmo quando o SmallVec morre enquanto ainda está na Stack (inline state).
    use std::sync::atomic::{AtomicUsize, Ordering};

    static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

    struct DropTracker(String);
    impl Drop for DropTracker {
        fn drop(&mut self) {
            let _ = &self.0; // Evita warning de field never read
            DROP_COUNT.fetch_add(1, Ordering::SeqCst);
        }
    }

    {
        let mut vec: SmallVec<DropTracker, 4> = SmallVec::new();
        vec.push(DropTracker("a".to_string()));
        vec.push(DropTracker("b".to_string()));
        // Vec morre e deve rodar 2 drops.
    }

    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 2);
}
