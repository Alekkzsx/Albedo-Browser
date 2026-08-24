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
    assert_eq!(arena.len(), 0);
    assert_eq!(arena.get(a), None);
    assert_eq!(arena.get(b), None);

    // Reutilizar após o clear não ressuscita ids antigos.
    let c = arena.alloc(3);
    assert_eq!(arena.get(a), None);
    assert_eq!(arena.get(c), Some(&3));
    assert_eq!(arena.len(), 1);
}

#[test]
fn clear_idempotent_multiple_calls_no_duplicates() {
    let mut arena = Arena::new();
    let _a = arena.alloc(10);
    let _b = arena.alloc(20);
    let _c = arena.alloc(30);

    assert_eq!(arena.len(), 3);

    // Primeiro clear
    arena.clear();
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());

    // Segundo clear consecutivo em arena vazia
    arena.clear();
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());

    // Terceiro clear
    arena.clear();
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());

    // Re-aloca 3 itens e confirma que nenhum slot foi duplicado
    let id1 = arena.alloc(100);
    let id2 = arena.alloc(200);
    let id3 = arena.alloc(300);

    assert_eq!(arena.len(), 3);
    assert_eq!(arena.get(id1), Some(&100));
    assert_eq!(arena.get(id2), Some(&200));
    assert_eq!(arena.get(id3), Some(&300));
}

#[test]
fn clear_with_interleaved_removals_and_reallocs() {
    let mut arena = Arena::new();
    let mut ids = Vec::new();
    for i in 0..10 {
        ids.push(arena.alloc(i));
    }
    assert_eq!(arena.len(), 10);

    // Remove alguns itens antes do clear
    arena.remove(ids[1]);
    arena.remove(ids[4]);
    arena.remove(ids[8]);
    assert_eq!(arena.len(), 7);

    // Limpa a arena
    arena.clear();
    assert_eq!(arena.len(), 0);

    // Todos os IDs anteriores devem estar inválidos
    for id in ids {
        assert_eq!(arena.get(id), None);
    }

    // Aloca novos itens
    let mut new_ids = Vec::new();
    for i in 0..5 {
        new_ids.push(arena.alloc(i * 10));
    }
    assert_eq!(arena.len(), 5);

    for (i, &id) in new_ids.iter().enumerate() {
        assert_eq!(arena.get(id), Some(&(i * 10)));
    }
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
