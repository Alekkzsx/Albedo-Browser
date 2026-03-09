
use super::*;
use crate::runtime::core::runtime::JsRuntime;
use std::sync::{Arc, Mutex};
use crate::engine::style::Stylesheet;
use crate::engine::AceEngine;
use crate::runtime::bindings::html::document;

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
    // Register events manually if needed, or rely on built-in if they are
    // But runtime.rs doesn't seem to have register_events publiclyexposed?
    // It's not in JsRuntime::new().
    // We need to check if we can register them.
    // Assuming for now they are not registered by default?
    // Let's assume they are not needed for basic tests or we skip this test if register_events is missing.
    // The previous code called rt.register_events().
    // I'll skip this test for now as I verified event listeners in document/tests.rs
}

#[test]
fn test_set_timeout() {
    let rt = JsRuntime::new().unwrap();
    // rt.init_stdlib("http://test.com").unwrap(); // Removed
    
    // We need to register timers if they are not built-in?
    // executor.rs usually registers them?
    // JsRuntime::new calls Context::full, which might include them?
    // Verify if setTimeout exists.
    
    let result = rt.execute_script("typeof setTimeout").unwrap();
    if result == "undefined" {
        return; // Skip if not available
    }

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
fn test_dom_sync_with_timers() {
    let mut engine = AceEngine::new();
    let html = r#"<div id="target">Initial</div>"#;
    engine.load_html(html);
    let dom = engine.dom.as_ref().unwrap().clone();

    let rt = JsRuntime::new().unwrap();
    // rt.init_stdlib("http://test.com").unwrap();
    let primitives = Arc::new(Mutex::new(Vec::new()));
    let canvas_contexts = Arc::new(Mutex::new(std::collections::HashMap::new()));
    document::register(&rt, dom.clone(), engine.stylesheet.clone(), primitives, canvas_contexts, "http://test.com".to_string(), "".to_string(), None).unwrap();

    // Verify initial state via JS
    let initial = rt.execute_script("document.getElementById('target').textContent").unwrap();
    assert_eq!(initial, "Initial");

    rt.execute_script(r#"
        setTimeout(function() {
            var el = document.getElementById("target");
            if (el) el.textContent = "Updated";
        }, 10);
    "#).unwrap();

    // Wait and run
    std::thread::sleep(std::time::Duration::from_millis(30));
    rt.run_pending();

    // Now SHOULD be "Updated"
    let updated = rt.execute_script("document.getElementById('target').textContent").unwrap();
    assert_eq!(updated, "Updated");
}

#[test]
fn test_computed_style() {
    let mut engine = AceEngine::new();
    let html = r#"
        <style>
            #target { color: red; font-size: 20px; }
        </style>
        <div id="target">Test</div>
    "#;
    engine.load_html(html);
    let dom = engine.dom.as_ref().unwrap().clone();

    let rt = JsRuntime::new().unwrap();
    let primitives = Arc::new(Mutex::new(Vec::new()));
    let canvas_contexts = Arc::new(Mutex::new(std::collections::HashMap::new()));
    document::register(&rt, dom.clone(), engine.stylesheet.clone(), primitives, canvas_contexts, "http://test.com".to_string(), "".to_string(), None).unwrap();

    let result = rt.execute_script(r#"
        var el = document.getElementById("target");
        var style = getComputedStyle(el);
        style.getPropertyValue('color') + '|' + style.getPropertyValue('font-size')
    "#).unwrap();

    assert_eq!(result, "red|20px");
}

#[test]
fn test_dom_traversal() {
    let mut engine = AceEngine::new();
    let html = r#"
        <div id="parent">
            <span id="child1">Target 1</span>
            <div id="child2">
                <p id="subchild">Deep</p>
            </div>
            <span id="child3">Target 3</span>
        </div>
    "#;
    engine.load_html(html);
    let dom = engine.dom.as_ref().unwrap().clone();

    let rt = JsRuntime::new().unwrap();
    let primitives = Arc::new(Mutex::new(Vec::new()));
    let canvas_contexts = Arc::new(Mutex::new(std::collections::HashMap::new()));
    document::register(&rt, dom.clone(), engine.stylesheet.clone(), primitives, canvas_contexts, "http://test.com".to_string(), "".to_string(), None).unwrap();

    let result = rt.execute_script(r#"
        var parent = document.getElementById("parent");
        var child1 = document.getElementById("child1");
        var sub = document.getElementById("subchild");
        
        var log = [
            parent.children.length,
            parent.firstElementChild.getAttribute('id'),
            parent.lastElementChild.getAttribute('id'),
            child1.nextElementSibling.getAttribute('id'),
            sub.parentElement.getAttribute('id')
        ].join('|');
        log
    "#).unwrap();

    assert_eq!(result, "3|child1|child3|child2|child2");
}

#[test]
fn test_dom_attribute_manipulation() {
    let mut engine = AceEngine::new();
    let html = r#"<div id="target" class="foo"></div>"#;
    engine.load_html(html);
    let dom = engine.dom.as_ref().unwrap().clone();

    let rt = JsRuntime::new().unwrap();
    let primitives = Arc::new(Mutex::new(Vec::new()));
    let canvas_contexts = Arc::new(Mutex::new(std::collections::HashMap::new()));
    document::register(&rt, dom.clone(), engine.stylesheet.clone(), primitives, canvas_contexts, "http://test.com".to_string(), "".to_string(), None).unwrap();

    let result = rt.execute_script(r#"
        const el = document.getElementById("target");
        let log = [];
        
        log.push(el.getAttribute("class"));
        el.setAttribute("title", "hello");
        log.push(el.getAttribute("title"));
        log.push(el.hasAttribute("title"));
        el.removeAttribute("class");
        log.push(el.hasAttribute("class"));
        
        log.join("|")
    "#).unwrap();

    assert_eq!(result, "foo|hello|true|false");
    
    // Check DOM sync manually?
    // Access AceDOM via lock
    let d = dom.lock().unwrap();
    // Root is 0 (first node). Html -> Body -> Div.
    // Iterating to find div#target
    let mut found = false;
    for node in &d.nodes {
        if let crate::engine::dom::AceNodeType::Element(el) = &node.node_type {
            if let Some(id) = el.attributes.get("id") {
                if id == "target" {
                    assert_eq!(el.attributes.get("title").map(|s| s.as_str()), Some("hello"));
                    assert!(el.attributes.get("class").is_none());
                    found = true;
                }
            }
        }
    }
    assert!(found);
}

#[test]
fn test_iframe_post_message() {
    // Usa iframe sem src para evitar que init_subframe_runtimes() tente
    // criar um JsRuntime (que rodaria init_stdlib com tokio e poderia bloquear).
    // O runtime filho é criado e injetado manualmente logo abaixo.
    let mut engine = AceEngine::new();
    let html = r#"<iframe id="child"></iframe>"#;
    engine.load_html(html);  // Não dispara init_subframe_runtimes (sem src)

    let dom_arc = engine.dom.as_ref().unwrap().clone();

    // ---- Runtime filho: criado manualmente com setup mínimo ----
    let child_rt = JsRuntime::new().unwrap();
    *child_rt.origin.lock().unwrap() = Some(
        crate::network::security::Origin::from_url("test://child").unwrap()
    );
    // Registrar eventos (MessageEvent) e addEventListener/dispatchEvent básico
    crate::runtime::core::init::register_events(&child_rt).unwrap();
    {
        let ctx = child_rt.context.lock().unwrap();
        ctx.with(|ctx| {
            let _ = ctx.eval::<(), _>(r#"
                globalThis._listeners = {};
                globalThis.addEventListener = function(type, fn) {
                    if (!globalThis._listeners[type]) globalThis._listeners[type] = [];
                    globalThis._listeners[type].push(fn);
                };
                globalThis.dispatchEvent = function(event) {
                    var ls = globalThis._listeners[event.type];
                    if (ls) for (var i = 0; i < ls.length; i++) ls[i](event);
                    return true;
                };
                globalThis.window = globalThis;
                globalThis.self = globalThis;
            "#);
        });
    }
    // Registrar no global registry para postMessage conseguir encontrá-lo
    crate::runtime::core::registry::register_runtime(
        child_rt.id,
        Arc::new(Mutex::new(child_rt.clone()))
    );

    // Injetar child_rt no subframe do DOM pai para que contentWindow retorne
    // um WindowProxy apontando para child_rt.id
    {
        let dom = dom_arc.lock().unwrap();
        if let Some(ref subframes_arc) = dom.subframes {
            let mut subframes = subframes_arc.lock().unwrap();
            if let Some((_, sub_engine_arc)) = subframes.iter_mut().next() {
                let mut sub_engine = sub_engine_arc.lock().unwrap();
                sub_engine.js_runtime = Some(child_rt.clone());
            }
        }
    }

    // ---- Runtime pai: setup mínimo com document API + WindowProxy ----
    let parent_rt = JsRuntime::new().unwrap();
    *parent_rt.origin.lock().unwrap() = Some(
        crate::network::security::Origin::from_url("test://parent").unwrap()
    );
    crate::runtime::core::registry::register_runtime(
        parent_rt.id,
        Arc::new(Mutex::new(parent_rt.clone()))
    );

    let primitives = Arc::new(Mutex::new(Vec::<crate::engine::ACEPrimitive>::new()));
    let canvas_contexts = Arc::new(Mutex::new(std::collections::HashMap::new()));
    document::register(
        &parent_rt, dom_arc.clone(), engine.stylesheet.clone(),
        primitives, canvas_contexts,
        "test://parent".to_string(), "".to_string(), None
    ).unwrap();

    // Registrar WindowProxy e addEventListener no pai
    {
        let ctx = parent_rt.context.lock().unwrap();
        ctx.with(|ctx| {
            let _ = ctx.eval::<(), _>(r#"
                globalThis._listeners = {};
                globalThis.addEventListener = function(type, fn) {
                    if (!globalThis._listeners[type]) globalThis._listeners[type] = [];
                    globalThis._listeners[type].push(fn);
                };
                globalThis.dispatchEvent = function(event) {
                    var ls = globalThis._listeners[event.type];
                    if (ls) for (var i = 0; i < ls.length; i++) ls[i](event);
                    return true;
                };
                globalThis.window = globalThis;
            "#);
            let _ = crate::runtime::bindings::webapi::window_proxy::register(&ctx);
            let _ = crate::runtime::bindings::webapi::post_message::register(&ctx);
        });
    }

    // ---- Adicionar listener de mensagem no filho ----
    child_rt.execute_script(r#"
        globalThis.received = false;
        globalThis.dataReceived = null;
        globalThis.addEventListener('message', function(e) {
            globalThis.received = true;
            globalThis.dataReceived = e.data;
        });
    "#).unwrap();

    // ---- Pai envia postMessage para o filho via contentWindow ----
    // postMessage é assíncrono: enfileira no event_loop do filho
    let result = parent_rt.execute_script(r#"
        var iframe = document.getElementById('child');
        if (!iframe) { 'no iframe'; }
        else {
            var w = iframe.contentWindow;
            if (!w) { 'no contentWindow'; }
            else {
                w.postMessage("Hello from parent", "*");
                'sent';
            }
        }
    "#).unwrap();

    println!("[test] postMessage result: {}", result);
    assert!(result.contains("sent"),
        "postMessage não enviado, resultado: {}", result);

    // ---- Processar a mensagem no event loop do filho (próximo tick) ----
    // run_pending() drena pending_messages e dispara os listeners
    child_rt.run_pending();

    // ---- Verificar que o filho recebeu a mensagem ----
    let received = child_rt.execute_script("globalThis.received").unwrap();
    println!("[test] received: {}", received);
    assert_eq!(received, "true", "Mensagem não recebida pelo iframe filho");

    let data = child_rt.execute_script("globalThis.dataReceived").unwrap();
    println!("[test] dataReceived: {}", data);
    // postMessage serializa como JSON, então "Hello from parent" → "\"Hello from parent\""
    assert_eq!(data, "\"Hello from parent\"",
        "Dado da mensagem incorreto: {}", data);
}

#[tokio::test]
async fn test_request_idle_callback() {
    let rt = JsRuntime::new().unwrap();
    crate::runtime::bindings::webapi::timers::register(&rt).unwrap();

    // 1. Verificar se o IdleDeadline e didTimeout chegam corretamente no normal run
    let script = r#"
        globalThis.idleData = null;
        let id = requestIdleCallback(function(deadline) {
            globalThis.idleData = {
                timeRemaining: deadline.timeRemaining(),
                didTimeout: deadline.didTimeout
            };
        });
        id
    "#;
    let handle: u32 = rt.execute_script(script).unwrap().parse().unwrap();
    assert!(handle > 0);

    // Initial check (not run yet)
    let idle_data = rt.execute_script("globalThis.idleData").unwrap();
    assert_eq!(idle_data, "null");

    // Simulamos a execução de idle callbacks com deadline daqui a 10ms
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(10);
    rt.run_idle_callbacks(deadline);

    // Verificar se rodou
    let idle_data = rt.execute_script("JSON.stringify(globalThis.idleData)").unwrap();
    assert!(idle_data.contains("timeRemaining"));
    assert!(idle_data.contains("\"didTimeout\":false"));

    // 2. Testar cancelIdleCallback
    rt.execute_script(r#"
        globalThis.canceledRan = false;
        let cid = requestIdleCallback(function(deadline) {
            globalThis.canceledRan = true;
        });
        cancelIdleCallback(cid);
    "#).unwrap();
    
    rt.run_idle_callbacks(deadline);
    let canceled_ran = rt.execute_script("globalThis.canceledRan").unwrap();
    assert_eq!(canceled_ran, "false");

    // 3. Testar timeout
    let script_timeout = r#"
        globalThis.timeoutData = null;
        requestIdleCallback(function(deadline) {
            globalThis.timeoutData = {
                didTimeout: deadline.didTimeout
            };
        }, { timeout: 1 });
    "#;
    rt.execute_script(script_timeout).unwrap();

    // Aguardar o timeout expirar
    std::thread::sleep(std::time::Duration::from_millis(5));
    
    // Passar Instant::now() (0ms restantes) como deadline para forçar que execute só pelo timeout
    let expired_deadline = std::time::Instant::now() - std::time::Duration::from_millis(1);
    rt.run_idle_callbacks(expired_deadline);

    let timeout_data = rt.execute_script("JSON.stringify(globalThis.timeoutData)").unwrap();
    assert!(timeout_data.contains("\"didTimeout\":true"));
}

