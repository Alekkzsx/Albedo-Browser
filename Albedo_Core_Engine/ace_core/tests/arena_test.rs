use ace_core::arena::{Arena, ArenaId};
use std::mem;

#[test]
fn alloc_and_get() {
    let mut arena = Arena::new();
    let id = arena.alloc(42);
    assert_eq!(arena.get(id), Some(&42));
    assert_eq!(arena.len(), 1);
}

#[test]
fn get_mut_updates_value() {
    let mut arena = Arena::new();
    let id = arena.alloc(1);
    *arena.get_mut(id).unwrap() = 99;
    assert_eq!(arena.get(id), Some(&99));
}

#[test]
fn removed_id_is_invalid() {
    let mut arena = Arena::new();
    let id = arena.alloc("vivo");
    assert_eq!(arena.remove(id), Some("vivo"));
    assert_eq!(arena.get(id), None);
    assert!(arena.is_empty());
}

#[test]
fn double_remove_is_safe() {
    let mut arena = Arena::new();
    let id = arena.alloc(1);
    assert_eq!(arena.remove(id), Some(1));
    assert_eq!(arena.remove(id), None); // sem double free
}

#[test]
fn clear_invalidates_all_ids() {
    let mut arena = Arena::new();
    let a = arena.alloc(1);
    let b = arena.alloc(2);
    arena.clear();

    assert!(arena.is_empty());
    assert_eq!(arena.get(a), None);
    assert_eq!(arena.get(b), None);

    // Reutilizar após o clear não ressuscita ids antigos.
    let c = arena.alloc(3);
    assert_eq!(arena.get(a), None);
    assert_eq!(arena.get(c), Some(&3));
}

#[test]
fn iteration_yields_only_live_values() {
    let mut arena = Arena::new();
    let a = arena.alloc(1);
    let _b = arena.alloc(2);
    arena.remove(a);

    let mut values: Vec<_> = arena.iter().map(|(_, &v)| v).collect();
    values.sort();
    assert_eq!(values, vec![2]);
}

#[test]
fn stats_are_consistent() {
    let mut arena = Arena::new();
    let a = arena.alloc(10u64);
    let _b = arena.alloc(20u64);
    arena.remove(a);
    let _c = arena.alloc(30u64); // reutiliza o slot de `a`

    let stats = arena.stats();
    assert_eq!(stats.live, 2);
    assert_eq!(stats.total_allocated, 3);
    assert_eq!(stats.total_freed, 1);
    assert_eq!(stats.slot_reuses, 1);
    assert_eq!(stats.bytes_allocated, 2 * mem::size_of::<u64>());
}

#[test]
fn cyclic_references_are_safe() {
    #[derive(Default)]
    struct Node {
        parent: Option<ArenaId<Node>>,
        first_child: Option<ArenaId<Node>>,
    }

    let mut arena: Arena<Node> = Arena::new();
    let parent = arena.alloc(Node::default());
    let child = arena.alloc(Node {
        parent: Some(parent),
        ..Default::default()
    });
    arena.get_mut(parent).unwrap().first_child = Some(child);

    // Ciclo pai ⇄ filho resolvido sem Rc/RefCell.
    let c = arena.get(child).unwrap();
    let p = arena.get(c.parent.unwrap()).unwrap();
    assert_eq!(p.first_child, Some(child));
}

#[test]
fn test_arena_id_node_id_conversion() {
    let mut arena: Arena<u32> = Arena::new();
    let id = arena.alloc(1234);

    let node_id = id.to_node_id();
    assert_eq!(node_id.raw(), id.raw());

    let restored_id = ArenaId::<u32>::from_node_id(node_id).unwrap();
    assert_eq!(restored_id, id);
    assert_eq!(arena.get(restored_id), Some(&1234));
}
