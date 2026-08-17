use ace_core::id::{NodeId, TabId, ProcessId, RequestId, FrameId};
use std::mem::size_of;

#[test]
fn test_niche_optimization_size_reduction() {
    // Validação estática fundamental: Option<NodeId> deve ter o mesmo tamanho que NodeId (8 bytes)
    assert_eq!(size_of::<NodeId>(), 8);
    assert_eq!(size_of::<Option<NodeId>>(), 8);

    assert_eq!(size_of::<TabId>(), 8);
    assert_eq!(size_of::<Option<TabId>>(), 8);

    assert_eq!(size_of::<ProcessId>(), 8);
    assert_eq!(size_of::<Option<ProcessId>>(), 8);

    assert_eq!(size_of::<RequestId>(), 8);
    assert_eq!(size_of::<Option<RequestId>>(), 8);

    assert_eq!(size_of::<FrameId>(), 8);
    assert_eq!(size_of::<Option<FrameId>>(), 8);
}

#[test]
fn test_id_conversions_and_uniqueness() {
    let id1 = NodeId::new();
    let id2 = NodeId::new();
    assert_ne!(id1, id2);

    let raw_val = id1.raw();
    assert!(raw_val > 0);

    let restored = NodeId::from_raw(raw_val).expect("deve restaurar ID válido");
    assert_eq!(id1, restored);

    let zero_opt = NodeId::from_raw(0);
    assert_eq!(zero_opt, None);
}
