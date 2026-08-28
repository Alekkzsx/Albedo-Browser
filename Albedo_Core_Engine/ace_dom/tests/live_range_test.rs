use ace_dom::{parse_html, BoundaryPoint, LiveRangeHandle, LiveRangeRegistry, Range};

#[test]
fn test_live_range_auto_adjustment_on_split_text() {
    let html = r#"<div id="container"><p id="txt">Hello World, Albedo Engine!</p></div>"#;
    let mut doc = parse_html(html);

    let p_id = doc.get_element_by_id("txt").unwrap();
    let text_node_id = doc.first_child(p_id).unwrap();

    // Range cobrindo "Albedo" (offset 13 a 19)
    let range = Range::from_points(
        BoundaryPoint::new(text_node_id, 13),
        BoundaryPoint::new(text_node_id, 19),
    );

    let handle = LiveRangeHandle::new(range);
    let mut registry = LiveRangeRegistry::new();
    registry.register(&handle);

    // Divide o texto no offset 6 ("Hello " | "World, Albedo Engine!")
    let new_text_node_id = doc.split_text(text_node_id, 6).expect("split_text success");
    registry.notify_split_text(text_node_id, new_text_node_id, 6);

    let updated_range = handle.get_range();
    // O nó deve ter sido atualizado para o novo nó e os offsets deslocados em 6
    assert_eq!(updated_range.start.node, new_text_node_id);
    assert_eq!(updated_range.start.offset, 7); // 13 - 6 = 7
    assert_eq!(updated_range.end.node, new_text_node_id);
    assert_eq!(updated_range.end.offset, 13); // 19 - 6 = 13
}

#[test]
fn test_live_range_auto_adjustment_on_node_removal_and_insertion() {
    let html = r#"<ul id="list"><li id="i1">1</li><li id="i2">2</li><li id="i3">3</li></ul>"#;
    let mut doc = parse_html(html);

    let list_id = doc.get_element_by_id("list").unwrap();
    let i2_id = doc.get_element_by_id("i2").unwrap();

    // Range apontando para o li#i2
    let range = Range::from_points(
        BoundaryPoint::new(i2_id, 0),
        BoundaryPoint::new(i2_id, 1),
    );

    let handle = LiveRangeHandle::new(range);
    let mut registry = LiveRangeRegistry::new();
    registry.register(&handle);

    // Remove i2 (índice 1 no pai)
    doc.remove_child(list_id, i2_id).unwrap();
    registry.notify_node_removal(i2_id, list_id, 1);

    let updated_range = handle.get_range();
    assert_eq!(updated_range.start.node, list_id);
    assert_eq!(updated_range.start.offset, 1);
    assert_eq!(updated_range.end.node, list_id);
    assert_eq!(updated_range.end.offset, 1);
}
