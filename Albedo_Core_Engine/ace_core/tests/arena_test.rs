use ace_core::arena::DomArena;

#[allow(dead_code)]
struct DomNode<'a> {
    _name: &'static str,
    child: Option<&'a DomNode<'a>>,
}

#[test]
fn test_arena_cyclic_like_allocation() {
    let arena = DomArena::new();
    
    let child = arena.alloc(DomNode { _name: "span", child: None });
    let _parent = arena.alloc(DomNode { _name: "div", child: Some(child) });

    // Memória cai sem problemas.
}
