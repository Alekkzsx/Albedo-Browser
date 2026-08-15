use ace_core::gc::{GcHeap, Trace};
use std::thread;

struct DummyTrace;
impl Trace for DummyTrace {
    fn trace(&self) {}
}

#[test]
fn test_chaos_interner() {
    let mut handles = vec![];

    for i in 0..64 {
        handles.push(thread::spawn(move || {
            for j in 0..1000 {
                // Stress Interner
                let string_to_intern = format!("dynamic_string_{}_{}", i, j);
                ace_core::intern::intern(&string_to_intern);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Force flush
    ace_core::intern::flush_dynamic_strings();
}

#[test]
fn test_generational_gc_nursery_sweep() {
    // Validação unitária das Arenas e GcGenerational introduzido na base.
    let mut heap = GcHeap::new();
    {
        let _box1 = heap.allocate(DummyTrace);
        let _box2 = heap.allocate(DummyTrace);
        // Os boxes saem do escopo aqui, ref count vai pra zero e podem ser coletados
    }
    
    // Sweep na nursery
    let freed = heap.collect(&[]);
    assert!(freed >= 2);
}
