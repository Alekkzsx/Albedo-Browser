use ace_core::task::CancellationToken;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[test]
fn test_cancellation_token_basic_and_callbacks() {
    let token = CancellationToken::new();
    assert!(!token.is_cancelled());

    let fired = Arc::new(AtomicBool::new(false));
    let fired_clone = Arc::clone(&fired);

    token.on_cancel(move || {
        fired_clone.store(true, Ordering::SeqCst);
    });

    assert!(!fired.load(Ordering::SeqCst));
    token.cancel();
    assert!(token.is_cancelled());
    assert!(fired.load(Ordering::SeqCst));

    // Callback registrado após cancelamento executa imediatamente
    let immediate_fired = Arc::new(AtomicBool::new(false));
    let imm_clone = Arc::clone(&immediate_fired);
    token.on_cancel(move || {
        imm_clone.store(true, Ordering::SeqCst);
    });
    assert!(immediate_fired.load(Ordering::SeqCst));
}

#[test]
fn test_cancellation_token_any_composition() {
    let t1 = CancellationToken::new();
    let t2 = CancellationToken::new();
    let t3 = CancellationToken::new();

    let composite = CancellationToken::any(&[&t1, &t2, &t3]);
    assert!(!composite.is_cancelled());

    // Cancela apenas o t2
    t2.cancel();
    assert!(composite.is_cancelled());
    assert!(!t1.is_cancelled());
    assert!(!t3.is_cancelled());
}

#[test]
fn test_cancellation_token_child_hierarchy() {
    let parent = CancellationToken::new();
    let child = parent.child();

    assert!(!parent.is_cancelled());
    assert!(!child.is_cancelled());

    parent.cancel();
    assert!(parent.is_cancelled());
    assert!(child.is_cancelled());
}
