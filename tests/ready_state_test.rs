use albedo::ace::engine::AceEngine;

#[tokio::test]
async fn test_ready_state_transitions() {
    let mut engine = AceEngine::new();
    let rt = albedo::ace::runtime::core::init::init_js_for_url("https://example.com", &engine).unwrap();
    engine.js_runtime = Some(rt.clone());

    let rs = rt.ready_state.lock().unwrap().clone();
    assert_eq!(rs, "interactive");

    let res = albedo::network::resources::ResourceResponse {
        url: "https://example.com/style.css".into(),
        data: b"body {}".to_vec(),
        resource_type: albedo::network::resources::ResourceType::Css,
        etag: None,
        cache_control: None,
        last_modified: None,
        expires: None,
        decoded_image: None,
        timestamp: std::time::SystemTime::now(),
        content_type: "text/css".to_string(),
        status_code: 200,
        original_size: 7,
        compressed_with: albedo::network::cache::CompressionMethod::None,
    };
    engine.pending_resources.lock().unwrap().insert(res.url.clone());
    engine.handle_resource_response(res);

    let rs2 = rt.ready_state.lock().unwrap().clone();
    assert_eq!(rs2, "complete");
}
