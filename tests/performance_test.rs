use albedo::ace::engine::AceEngine;

#[tokio::test]
async fn test_performance_now() {
    let engine = AceEngine::new();
    let rt = albedo::ace::runtime::core::init::init_js_for_url("https://example.com", &engine).unwrap();
    let val = rt.execute_script("performance.now() >= 0").unwrap();
    assert_eq!(val, "true");

    let val2 = rt.execute_script("typeof performance.mark === 'function'").unwrap();
    assert_eq!(val2, "true");
}
