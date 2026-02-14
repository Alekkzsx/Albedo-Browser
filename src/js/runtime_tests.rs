use super::*;
use std::sync::{Arc, Mutex};
use crate::engine::style::Stylesheet;

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
    rt.init_stdlib("http://test.com").unwrap();
    
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
    rt.init_stdlib("http://test.com").unwrap();
    
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
    rt.init_stdlib("http://test.com").unwrap();
    document::register(&rt, dom.clone(), engine.stylesheet.clone()).unwrap();

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

#[test]
fn test_fetch_registration() {
    let rt = JsRuntime::new().unwrap();
    rt.init_stdlib("http://test.com").unwrap();
    
    // Check if fetch is defined and is a function
    let result = rt.execute_script("typeof fetch").unwrap();
    assert_eq!(result, "function");
    
    // Check if Response class is defined
    let result = rt.execute_script("typeof Response").unwrap();
    assert_eq!(result, "function");
}

#[test]
fn test_computed_style() {
    use crate::engine::AceEngine;
    use crate::js::bindings::document;

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
    rt.init_stdlib("http://test.com").unwrap();
    document::register(&rt, dom.clone(), engine.stylesheet.clone()).unwrap();

    let result = rt.execute_script(r#"
        var el = document.getElementById("target");
        var style = getComputedStyle(el);
        style.getPropertyValue('color') + '|' + style.getPropertyValue('font-size')
    "#).unwrap();

    assert_eq!(result, "red|20px");
}

#[test]
fn test_dom_traversal() {
    use crate::engine::AceEngine;
    use crate::js::bindings::document;

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
    rt.init_stdlib("http://test.com").unwrap();
    document::register(&rt, dom.clone(), engine.stylesheet.clone()).unwrap();

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
fn test_dynamic_css() {
    use crate::engine::AceEngine;
    use crate::js::bindings::document;

    let mut engine = AceEngine::new();
    let html = r#"
        <div id="target">Hello</div>
        <style id="style-tag">#target { color: red; }</style>
    "#;
    engine.load_html(html);
    let dom = engine.dom.as_ref().unwrap().clone();

    let rt = JsRuntime::new().unwrap();
    rt.init_stdlib("http://test.com").unwrap();
    document::register(&rt, dom.clone(), engine.stylesheet.clone()).unwrap();

    // 1. Initial color should be red
    let result1 = rt.execute_script(r#"
        const el = document.getElementById("target");
        getComputedStyle(el).color
    "#).unwrap();
    assert_eq!(result1, "red");

    // 2. Change style tag content via JS
    rt.execute_script(r##"
        const style = document.getElementById("style-tag");
        style.textContent = "#target { color: blue; }";
    "##).unwrap();

    // In a real browser, the mutation would trigger run_pending which calls update_stylesheet.
    // Here we simulate that if run_pending returns true.
    let (mutated, stylesheet_dirty) = rt.run_pending();
    if mutated || stylesheet_dirty {
        engine.update_stylesheet();
    }

    let result2 = rt.execute_script(r#"
        getComputedStyle(document.getElementById("target")).color
    "#).unwrap();
    assert_eq!(result2, "blue");

    // 3. Add new style tag via innerHTML
    rt.execute_script(r##"
        const div = document.createElement("div");
        div.innerHTML = "<style>#target { font-size: 50px; }</style>";
        document.body.appendChild(div);
    "##).unwrap();

    let (mutated, stylesheet_dirty) = rt.run_pending();
    if mutated || stylesheet_dirty {
        engine.update_stylesheet();
    }

    let result3 = rt.execute_script(r#"
        getComputedStyle(document.getElementById("target")).fontSize
    "#).unwrap();
    assert_eq!(result3, "50px");
    
    rt.run_gc();
}

#[test]
fn test_css_specificity() {
    use crate::engine::AceEngine;
    use crate::js::bindings::document;

    let mut engine = AceEngine::new();
    let html = r#"
        <style>
            div { color: green; }
            .content { color: yellow; }
            #target { color: red; }
            
            p.important { color: purple; }
            p { color: orange; }
        </style>
        <div id="target" class="content">Specificity Test</div>
        <p class="important">P Test</p>
    "#;
    engine.load_html(html);
    let dom = engine.dom.as_ref().unwrap().clone();

    let rt = JsRuntime::new().unwrap();
    rt.init_stdlib("http://test.com").unwrap();
    document::register(&rt, dom.clone(), engine.stylesheet.clone()).unwrap();

    let result = rt.execute_script(r#"
        const div = document.getElementById("target");
        const p = document.querySelector("p");
        getComputedStyle(div).color + "|" + getComputedStyle(p).color
    "#).unwrap();

    assert_eq!(result, "red|purple");
    
    rt.run_gc();
}

#[test]
fn test_event_complex_propagation() {
    use crate::engine::AceEngine;
    use crate::js::bindings::document;

    let mut engine = AceEngine::new();
    let html = r#"
        <div id="outer">
            <div id="inner">
                <button id="btn">Click me</button>
            </div>
        </div>
    "#;
    engine.load_html(html);
    let dom = engine.dom.as_ref().unwrap().clone();

    let rt = JsRuntime::new().unwrap();
    rt.init_stdlib("http://test.com").unwrap();
    rt.register_events().unwrap();
    document::register(&rt, dom.clone(), engine.stylesheet.clone()).unwrap();

    let result = rt.execute_script(r#"
        const outer = document.getElementById("outer");
        const inner = document.getElementById("inner");
        const btn = document.getElementById("btn");
        
        let path = [];
        outer.addEventListener("click", () => path.push("outer"));
        inner.addEventListener("click", (e) => {
            path.push("inner");
            e.stopPropagation();
        });
        btn.addEventListener("click", () => path.push("btn"));
        
        btn.dispatchEvent(new MouseEvent("click", { bubbles: true }));
        path.join("|")
    "#).unwrap();

    assert_eq!(result, "btn|inner");
    
    rt.run_gc();
}

#[test]
fn test_dom_attribute_manipulation() {
    use crate::engine::AceEngine;
    use crate::js::bindings::document;

    let mut engine = AceEngine::new();
    let html = r#"<div id="target" class="foo"></div>"#;
    engine.load_html(html);
    let dom = engine.dom.as_ref().unwrap().clone();

    let rt = JsRuntime::new().unwrap();
    rt.init_stdlib("http://test.com").unwrap();
    document::register(&rt, dom.clone(), engine.stylesheet.clone()).unwrap();

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
    
    // Verify sync to native DOM
    let el_data = dom.root.select_first("#target").unwrap();
    let attrs = el_data.attributes.borrow();
    assert_eq!(attrs.get("title").unwrap(), "hello");
    assert!(attrs.get("class").is_none());
}

#[test]
fn test_computed_style_extended() {
    use crate::engine::AceEngine;
    use crate::js::bindings::document;

    let mut engine = AceEngine::new();
    let html = r#"
        <style>
            #target { 
                margin: 10px;
                padding: 20px;
                border: 1px solid black;
                width: 100px;
                height: 50px;
            }
        </style>
        <div id="target"></div>
    "#;
    engine.load_html(html);
    let dom = engine.dom.as_ref().unwrap().clone();

    let rt = JsRuntime::new().unwrap();
    rt.init_stdlib("http://test.com").unwrap();
    document::register(&rt, dom.clone(), engine.stylesheet.clone()).unwrap();

    let result = rt.execute_script(r#"
        const el = document.getElementById("target");
        const style = getComputedStyle(el);
        [
            style.getPropertyValue('margin-top'),
            style.getPropertyValue('padding-left'),
            style.getPropertyValue('width'),
            style.getPropertyValue('height')
        ].join('|')
    "#).unwrap();

    assert_eq!(result, "10px|20px|100px|50px");
    
    rt.run_gc();
}

#[test]
fn test_inner_html_extended() {
    use crate::engine::AceEngine;
    use crate::js::bindings::document;

    let mut engine = AceEngine::new();
    let html = r#"<div id="container"></div>"#;
    engine.load_html(html);
    let dom = engine.dom.as_ref().unwrap().clone();

    let rt = JsRuntime::new().unwrap();
    rt.init_stdlib("http://test.com").unwrap();
    document::register(&rt, dom.clone(), engine.stylesheet.clone()).unwrap();

    rt.execute_script(r##"
        const container = document.getElementById("container");
        container.innerHTML = `
            <style>#dynamic { color: cyan; }</style>
            <div class="wrapper">
                <span id="dynamic">Hello</span>
            </div>
        `;
    "##).unwrap();

    let (mutated, stylesheet_dirty) = rt.run_pending();
    if mutated || stylesheet_dirty {
        engine.update_stylesheet();
    }

    let result = rt.execute_script(r#"
        const span = document.getElementById("dynamic");
        getComputedStyle(span).color
    "#).unwrap();

    assert_eq!(result, "cyan");
    
    // Check DOM structure
    let span_count = dom.root.select("span#dynamic").unwrap().count();
    assert_eq!(span_count, 1);
    
    rt.run_gc();
}
