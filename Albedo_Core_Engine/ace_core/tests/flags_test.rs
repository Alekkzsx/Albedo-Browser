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

#[test]
fn test_style_hint_to_node_flags_subtree_recalc() {
    use ace_core::flags::{is_node_dirty, style_hint_to_node_flags};

    // NONE
    assert_eq!(style_hint_to_node_flags(StyleChangeHint::NONE), NodeFlags::empty());

    // REPAINT
    let repaint_flags = style_hint_to_node_flags(StyleChangeHint::REPAINT);
    assert_eq!(repaint_flags, NodeFlags::DIRTY_PAINT);
    assert!(is_node_dirty(repaint_flags));

    // REFLOW_LAYOUT
    let reflow_flags = style_hint_to_node_flags(StyleChangeHint::REFLOW_LAYOUT);
    assert_eq!(reflow_flags, NodeFlags::DIRTY_LAYOUT | NodeFlags::DIRTY_PAINT);

    // RECALC_STYLE
    let recalc_flags = style_hint_to_node_flags(StyleChangeHint::RECALC_STYLE);
    assert_eq!(
        recalc_flags,
        NodeFlags::DIRTY_STYLE | NodeFlags::DIRTY_LAYOUT | NodeFlags::DIRTY_PAINT
    );

    // SUBTREE_RECALC
    let subtree_flags = style_hint_to_node_flags(StyleChangeHint::SUBTREE_RECALC);
    assert_eq!(
        subtree_flags,
        NodeFlags::SUBTREE_DIRTY | NodeFlags::DIRTY_STYLE
    );
    assert!(subtree_flags.contains(NodeFlags::SUBTREE_DIRTY));
    assert!(subtree_flags.contains(NodeFlags::DIRTY_STYLE));
    assert!(is_node_dirty(subtree_flags));

    // Combined SUBTREE_RECALC + REPAINT
    let combined = style_hint_to_node_flags(StyleChangeHint::SUBTREE_RECALC | StyleChangeHint::REPAINT);
    assert_eq!(
        combined,
        NodeFlags::SUBTREE_DIRTY | NodeFlags::DIRTY_STYLE | NodeFlags::DIRTY_PAINT
    );
}
