use ace_core::diagnostics::{BreadcrumbBuffer, BreadcrumbEntry, CrashKeyRegistry};

#[test]
fn test_crash_key_registry() {
    let registry = CrashKeyRegistry::new();
    registry.set_key("url", "https://example.com/login");
    registry.set_key("script_id", "42");

    assert_eq!(
        registry.get_key("url"),
        Some("https://example.com/login".to_string())
    );
    assert_eq!(registry.get_key("script_id"), Some("42".to_string()));

    let summary = registry.dump_summary();
    assert!(summary.contains("url: https://example.com/login"));

    registry.remove_key("url");
    assert_eq!(registry.get_key("url"), None);
}

#[test]
fn test_breadcrumb_buffer_rotation() {
    let buffer: BreadcrumbBuffer<3> = BreadcrumbBuffer::new();

    buffer.record("Navigation", "Navigated to home", 100);
    buffer.record("DOM", "Started parse", 110);
    buffer.record("CSS", "Computed styles", 120);

    let snap1 = buffer.snapshot();
    assert_eq!(snap1.len(), 3);
    assert_eq!(snap1[0].category, "Navigation");

    // Adiciona o quarto item, que deve expulsar o mais antigo
    buffer.record("Render", "Frame painted", 130);

    let snap2 = buffer.snapshot();
    assert_eq!(snap2.len(), 3);
    assert_eq!(snap2[0].category, "DOM");
    assert_eq!(snap2[2].category, "Render");
}

#[test]
fn test_breadcrumb_buffer_capacity_zero() {
    let buffer: BreadcrumbBuffer<0> = BreadcrumbBuffer::new();
    assert_eq!(buffer.capacity(), 0);
    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());

    // Gravação não deve entrar em pânico nem armazenar nada
    buffer.record("Test", "No-op event", 1000);
    buffer.push(BreadcrumbEntry {
        category: "Test2",
        message: "No-op entry".to_string(),
        timestamp_ms: 1001,
    });

    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());
    assert_eq!(buffer.snapshot().len(), 0);

    buffer.with_slices(|(s1, s2)| {
        assert_eq!(s1.len(), 0);
        assert_eq!(s2.len(), 0);
    });
}

#[test]
fn test_breadcrumb_buffer_capacity_one() {
    let buffer: BreadcrumbBuffer<1> = BreadcrumbBuffer::new();
    assert_eq!(buffer.capacity(), 1);

    buffer.record("Event1", "Primeiro", 10);
    assert_eq!(buffer.len(), 1);
    assert_eq!(buffer.snapshot()[0].message, "Primeiro");

    buffer.record("Event2", "Segundo", 20);
    assert_eq!(buffer.len(), 1);
    assert_eq!(buffer.snapshot()[0].message, "Segundo");
}

#[test]
fn test_breadcrumb_buffer_capacity_overflow() {
    let buffer: BreadcrumbBuffer<5> = BreadcrumbBuffer::new();

    for i in 1..=10 {
        buffer.record("Iter", format!("Message {}", i), i as u64 * 100);
    }

    assert_eq!(buffer.len(), 5);
    let snap = buffer.snapshot();
    assert_eq!(snap.len(), 5);
    assert_eq!(snap[0].message, "Message 6");
    assert_eq!(snap[4].message, "Message 10");
}

#[test]
fn test_breadcrumb_buffer_with_slices_and_clear() {
    let buffer: BreadcrumbBuffer<4> = BreadcrumbBuffer::new();
    buffer.record("A", "Msg A", 1);
    buffer.record("B", "Msg B", 2);

    let total = buffer.with_slices(|(s1, s2)| s1.len() + s2.len());
    assert_eq!(total, 2);

    buffer.clear();
    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());
    assert_eq!(buffer.snapshot().len(), 0);
}

#[test]
fn test_breadcrumb_buffer_push_and_is_empty() {
    let buffer: BreadcrumbBuffer<2> = BreadcrumbBuffer::new();
    assert!(buffer.is_empty());

    buffer.push(BreadcrumbEntry {
        category: "Manual",
        message: "Pushed".to_string(),
        timestamp_ms: 42,
    });

    assert!(!buffer.is_empty());
    assert_eq!(buffer.len(), 1);
    assert_eq!(buffer.snapshot()[0].category, "Manual");
}
