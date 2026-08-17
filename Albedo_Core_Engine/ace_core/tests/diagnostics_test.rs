use ace_core::diagnostics::{BreadcrumbBuffer, CrashKeyRegistry};

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
