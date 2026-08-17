use ace_core::memory::{MemoryPressureLevel, MemoryPressureListener};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn test_memory_pressure_notifications() {
    let listener = MemoryPressureListener::new();
    assert_eq!(listener.current_level(), MemoryPressureLevel::None);

    let counter = Arc::new(AtomicUsize::new(0));
    let c_clone = Arc::clone(&counter);

    let id = listener.register(move |level| {
        if level == MemoryPressureLevel::Moderate {
            c_clone.fetch_add(1, Ordering::SeqCst);
        } else if level == MemoryPressureLevel::Critical {
            c_clone.fetch_add(10, Ordering::SeqCst);
        }
    });

    listener.notify(MemoryPressureLevel::Moderate);
    assert_eq!(listener.current_level(), MemoryPressureLevel::Moderate);
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    listener.notify(MemoryPressureLevel::Critical);
    assert_eq!(listener.current_level(), MemoryPressureLevel::Critical);
    assert_eq!(counter.load(Ordering::SeqCst), 11);

    assert!(listener.unregister(id));
    listener.notify(MemoryPressureLevel::Moderate);
    assert_eq!(counter.load(Ordering::SeqCst), 11);
}
