use ace_core::id::NodeId;
use ace_core::memory::{GCRoot, RootSet, Traceable, Visitor};
use std::sync::Arc;

struct MockVisitor {
    visited_nodes: Vec<NodeId>,
    visited_opaque: Vec<(&'static str, u64)>,
}

impl Visitor for MockVisitor {
    fn visit_node(&mut self, id: NodeId) {
        self.visited_nodes.push(id);
    }

    fn visit_root(&mut self, root: &dyn Traceable) {
        root.trace(self);
    }

    fn visit_opaque(&mut self, tag: &'static str, id: u64) {
        self.visited_opaque.push((tag, id));
    }
}

struct DomElement {
    node_id: NodeId,
    js_object_id: Option<u64>,
}

impl Traceable for DomElement {
    fn trace(&self, visitor: &mut dyn Visitor) {
        visitor.visit_node(self.node_id);
        if let Some(id) = self.js_object_id {
            visitor.visit_opaque("JSObject", id);
        }
    }
}

#[test]
fn test_root_set_and_gc_root_lifecycle() {
    let root_set = Arc::new(RootSet::new());
    assert_eq!(root_set.len(), 0);
    assert!(root_set.is_empty());

    let node1_id = NodeId::new();
    let node2_id = NodeId::new();

    {
        let root1 = GCRoot::new(
            DomElement {
                node_id: node1_id,
                js_object_id: Some(101),
            },
            Arc::clone(&root_set),
        );
        assert_eq!(root_set.len(), 1);

        let root2 = GCRoot::new(
            DomElement {
                node_id: node2_id,
                js_object_id: None,
            },
            Arc::clone(&root_set),
        );
        assert_eq!(root_set.len(), 2);

        let mut visitor = MockVisitor {
            visited_nodes: Vec::new(),
            visited_opaque: Vec::new(),
        };

        root_set.trace_all(&mut visitor);
        assert_eq!(visitor.visited_nodes.len(), 2);
        assert!(visitor.visited_nodes.contains(&node1_id));
        assert!(visitor.visited_nodes.contains(&node2_id));
        assert_eq!(visitor.visited_opaque.len(), 1);
        assert_eq!(visitor.visited_opaque[0], ("JSObject", 101));
    }

    // Após o drop dos GCRoot, o RootSet deve estar vazio
    assert_eq!(root_set.len(), 0);
    assert!(root_set.is_empty());
}
