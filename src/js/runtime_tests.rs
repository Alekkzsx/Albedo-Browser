use super::*;

#[test]
fn test_basic_execution() {
    let rt = JsRuntime::new().unwrap();
    let _ = rt.execute_script("2 + 2").unwrap();
}

#[test]
fn test_variables() {
    let rt = JsRuntime::new().unwrap();
    rt.execute_script("var x = 10; var y = 20;").unwrap();
    let _ = rt.execute_script("x + y").unwrap();
}

#[test]
fn test_function_definition() {
    let rt = JsRuntime::new().unwrap();
    rt.execute_script("function multiply(a, b) { return a * b; }").unwrap();
}

#[test]
fn test_event_subclasses() {
    let rt = JsRuntime::new().unwrap();
    rt.register_events().unwrap();
    
    let script = "
        try {
            var m = new MouseEvent('click', { clientX: 10, clientY: 20 });
            var k = new KeyboardEvent('keydown', { key: 'A', ctrlKey: true });
            
            var result = [
                // m instanceof Event,
                m instanceof MouseEvent,
                m.type,
                m.clientX,
                // k instanceof Event,
                k.key,
                k.ctrlKey
            ].join('|');
            result
        } catch(e) {
            'Error: ' + e.toString()
        }
    ";
    
    let result = rt.execute_script(script).unwrap();
    assert_eq!(result, "true|click|10|A|true");
}

#[test]
fn test_set_timeout() {
    let rt = JsRuntime::new().unwrap();
    rt.init_stdlib().unwrap();
    
    let script = "
        var called = 'false';
        setTimeout(function() {
            called = 'true';
        }, 10);
    ";
    rt.execute_script(script).unwrap();
    
    // Initial check
    let result = rt.execute_script("called").unwrap();
    assert_eq!(result, "false");
    
    // Wait and run loop
    std::thread::sleep(std::time::Duration::from_millis(20));
    rt.run_pending();
    
    // Final check
    let result = rt.execute_script("called").unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_set_interval() {
    let rt = JsRuntime::new().unwrap();
    rt.init_stdlib().unwrap();
    
    let script = "
        var counter = 0;
        var id = setInterval(function() {
            counter++;
        }, 20);
    ";
    rt.execute_script(script).unwrap();
    
    // Run loop a few times
    std::thread::sleep(std::time::Duration::from_millis(25)); // 1st tick
    rt.run_pending();
    
    std::thread::sleep(std::time::Duration::from_millis(25)); // 2nd tick
    rt.run_pending();
    
    // Stop it
    let result = rt.execute_script("clearInterval(id); counter").unwrap();
    // Should be at least 2
    let count: i32 = result.parse().unwrap_or(0);
    assert!(count >= 2, "Counter should be at least 2, got {}", count);
}

#[test]
fn test_dom_sync_with_timers() {
    use crate::engine::AceEngine;
    use crate::js::bindings::document;

    let mut engine = AceEngine::new();
    let html = r#"<div id="target">Initial</div>"#;
    engine.load_html(html);
    let dom = engine.dom.as_ref().unwrap().clone();

    let rt = JsRuntime::new().unwrap();
    rt.init_stdlib().unwrap();
    document::register(&rt, dom.clone()).unwrap();

    rt.execute_script(r#"
        setTimeout(function() {
            document.getElementById("target").textContent = "Updated";
        }, 10);
    "#).unwrap();

    // Initially still "Initial"
    assert_eq!(dom.root.select_first("#target").unwrap().text_contents(), "Initial");

    // Wait and run
    std::thread::sleep(std::time::Duration::from_millis(30));
    rt.run_pending();

    // Now SHOULD be "Updated"
    assert_eq!(dom.root.select_first("#target").unwrap().text_contents(), "Updated");
}
