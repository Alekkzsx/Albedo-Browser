use ace_core::gc::{mark, GcBox, GcHeap, Trace};
use std::cell::RefCell;

struct JsObject {
    value: i32,
    child: RefCell<Option<GcBox<JsObject>>>,
}

impl Trace for JsObject {
    fn trace(&self) {
        if let Some(child_box) = self.child.borrow().as_ref() {
            mark(child_box);
        }
    }
}

// Wrapper para usar o Trace no Root
struct Root(Option<GcBox<JsObject>>);
impl Trace for Root {
    fn trace(&self) {
        if let Some(b) = &self.0 {
            mark(b);
        }
    }
}

#[test]
fn test_gc_mark_and_sweep() {
    let mut heap = GcHeap::new();

    let a = heap.allocate(JsObject {
        value: 1,
        child: RefCell::new(None),
    });
    let b = heap.allocate(JsObject {
        value: 2,
        child: RefCell::new(None),
    });
    let _c = heap.allocate(JsObject {
        value: 3,
        child: RefCell::new(None),
    });

    // A aponta para B.
    *a.child.borrow_mut() = Some(b.clone());
    // C fica isolado e inatingível a partir do root (A).

    let root = Root(Some(a.clone()));
    let roots: &[&dyn Trace] = &[&root];

    let freed = heap.collect(roots);

    // O objeto C deve ser coletado (freed = 1)
    assert_eq!(freed, 1);

    // Na próxima passada sem raízes, tudo deve morrer (A e B = 2).
    let empty_roots: &[&dyn Trace] = &[];
    let freed_all = heap.collect(empty_roots);
    assert_eq!(freed_all, 2);
}
