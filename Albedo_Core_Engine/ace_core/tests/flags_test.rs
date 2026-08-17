use ace_core::flags::{NodeFlags, RenderFlags, StyleChangeHint};

#[test]
fn test_node_flags_operations() {
    let mut flags = NodeFlags::IS_ELEMENT | NodeFlags::IS_CONNECTED;
    assert!(flags.contains(NodeFlags::IS_ELEMENT));
    assert!(flags.contains(NodeFlags::IS_CONNECTED));
    assert!(!flags.contains(NodeFlags::DIRTY_STYLE));

    flags.insert(NodeFlags::DIRTY_STYLE | NodeFlags::SUBTREE_DIRTY);
    assert!(flags.contains(NodeFlags::DIRTY_STYLE));
    assert!(flags.contains(NodeFlags::SUBTREE_DIRTY));

    flags.remove(NodeFlags::DIRTY_STYLE);
    assert!(!flags.contains(NodeFlags::DIRTY_STYLE));
}

#[test]
fn test_style_change_hint_and_render_flags() {
    let hint = StyleChangeHint::REPAINT | StyleChangeHint::REFLOW_LAYOUT;
    assert!(hint.intersects(StyleChangeHint::REFLOW_LAYOUT));
    assert!(!hint.intersects(StyleChangeHint::RECONSTRUCT_FRAME));

    let render = RenderFlags::VISIBLE | RenderFlags::COMPOSITED_LAYER;
    assert!(render.contains(RenderFlags::VISIBLE));
    assert!(render.contains(RenderFlags::COMPOSITED_LAYER));
}
