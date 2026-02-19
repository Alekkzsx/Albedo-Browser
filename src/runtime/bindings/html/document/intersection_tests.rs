#[cfg(test)]
mod tests {
    use crate::runtime::core::runtime::JsRuntime;
    use crate::engine::AceEngine;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    fn test_intersection_observer_basic() {
        let mut engine = AceEngine::new();
        let rt = crate::runtime::core::init::init_js_for_url("http://test.com", &engine, None, None, None, None).unwrap();
        engine.js_runtime = Some(rt.clone());

        // 1. Setup HTML with target element outside viewport
        let html = r#"
            <div id="target" style="width: 100px; height: 100px; position: absolute; top: 1000px; left: 0;"></div>
            <div id="spacer" style="height: 2000px;"></div>
        "#;
        engine.load_html(html, "http://test.com");
        engine.layout(800.0, 600.0);

        // 2. Register Observer
        println!("DEBUG: Executing script...");
        let script = r#"
            globalThis.entries = [];
            globalThis.observer = new IntersectionObserver((ent) => {
                globalThis.entries.push(...ent);
            }, { threshold: [0.1] });
            
            let target = document.getElementById('target');
            observer.observe(target);
        "#;
        rt.execute_script(script).unwrap();
        println!("DEBUG: Script executed.");

        // 3. Initial Check (Should not intersect)
        engine.tick(0.0);
        println!("DEBUG: First tick done.");
        crate::runtime::core::executor::run_pending(&rt);
        println!("DEBUG: First run_pending done.");
        
        let result = rt.execute_script("globalThis.entries.length").unwrap();
        println!("DEBUG: First check done. Result: {:?}", result);
        assert_eq!(result, "0"); 

        // Let's scroll to make it intersect
        engine.viewport_y = -500.0; 
        engine.tick(0.0); // Triggers check_intersections
        println!("DEBUG: Second tick done.");
        crate::runtime::core::executor::run_pending(&rt);
        println!("DEBUG: Second run_pending done.");

        let result = rt.execute_script("globalThis.entries.length").unwrap();
        assert_eq!(result, "1");
        
        let is_intersecting = rt.execute_script("globalThis.entries[0].isIntersecting").unwrap();
        assert_eq!(is_intersecting, "true");
        
        let ratio = rt.execute_script("globalThis.entries[0].intersectionRatio > 0").unwrap();
        assert_eq!(ratio, "true");
    }

    #[test]
    fn test_intersection_observer_disconnect() {
        let mut engine = AceEngine::new();
        let rt = crate::runtime::core::init::init_js_for_url("http://test.com", &engine, None, None, None, None).unwrap();
        engine.js_runtime = Some(rt.clone());

        let html = r#"<div id="target" style="width:100px; height:100px; top:10px;"></div>"#;
        engine.load_html(html, "http://test.com");
        engine.layout(800.0, 600.0);

        let script = r#"
            globalThis.count = 0;
            let obs = new IntersectionObserver(() => { globalThis.count++; });
            obs.observe(document.getElementById('target'));
            obs.disconnect();
        "#;
        rt.execute_script(script).unwrap();

        engine.tick(0.0);
        crate::runtime::core::executor::run_pending(&rt); // Flush any pending from observe before disconnect?
        // Disconnect clears pending? Spec says "empties observer's record queue".
        // Use 'takeRecords()' to clear or disconnect clears it.
        
        // We implemented disconnect as clearing targets.
        // But pending notifications in registry might remain if queued before disconnect?
        // Our registry::disconnect only clears targets.
        // Ideally it should clear pending too.
        
        let count = rt.execute_script("globalThis.count").unwrap();
        assert_eq!(count, "0");
    }
}
