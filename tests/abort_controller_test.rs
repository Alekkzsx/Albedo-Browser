use albedo::ace::engine::AceEngine;

#[tokio::test]
async fn test_abort_controller() {
    let engine = AceEngine::new();
    let rt = albedo::ace::runtime::core::init::init_js_for_url("https://example.com", &engine).unwrap();

    let is_controller_defined = rt.execute_script("typeof AbortController === 'function'").unwrap();
    assert_eq!(is_controller_defined, "true");

    let is_signal_defined = rt.execute_script("typeof AbortSignal === 'function'").unwrap();
    assert_eq!(is_signal_defined, "true");

    rt.execute_script(r#"
        globalThis.ac = new AbortController();
        globalThis.aborted_initial = ac.signal.aborted;

        globalThis.abort_called = false;
        ac.signal.onabort = function() {
            globalThis.abort_called = true;
        };

        ac.abort();
        globalThis.aborted_after = ac.signal.aborted;
    "#).unwrap();

    let initial = rt.execute_script("globalThis.aborted_initial").unwrap();
    assert_eq!(initial, "false");

    let after = rt.execute_script("globalThis.aborted_after").unwrap();
    assert_eq!(after, "true");

    let called = rt.execute_script("globalThis.abort_called").unwrap();
    assert_eq!(called, "true");
}
