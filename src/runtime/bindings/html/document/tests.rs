use super::*;
use crate::engine::dom::AceDOM;
use crate::engine::style;
use crate::runtime::core::runtime::JsRuntime;
use kuchiki::traits::TendrilSink;
use std::sync::{Arc, Mutex};

fn create_test_env(html: &str) -> (JsRuntime, Arc<Mutex<AceDOM>>) {
    let document = kuchiki::parse_html().one(html);
    let dom = Arc::new(Mutex::new(AceDOM::from_kuchiki(document)));
    let rt = JsRuntime::new().unwrap();

    let stylesheet = Arc::new(Mutex::new(style::parse(""))); // Empty stylesheet

    // register expects 8 arguments
    register(
        &rt,
        dom.clone(),
        stylesheet,
        Arc::new(Mutex::new(Vec::new())),
        Arc::new(Mutex::new(std::collections::HashMap::new())),
        "http://test.com".to_string(),
        "".to_string(),
        None,
    )
    .unwrap();

    (rt, dom)
}

#[test]
fn test_get_element_by_id() {
    let html = r#"<div id="test_div">Hello World</div>"#;
    let (rt, _dom) = create_test_env(html);

    let result = rt
        .execute_script(
            "
        var el = document.getElementById('test_div');
        el.tagName + ':' + el.textContent
    ",
        )
        .unwrap();

    assert_eq!(result, "div:Hello World");
}

#[test]
fn test_get_element_by_id_null() {
    let html = r#"<div></div>"#;
    let (rt, _dom) = create_test_env(html);

    let result = rt
        .execute_script(
            "
        var el = document.getElementById('non_existent');
        el === null ? 'null' : 'not null'
    ",
        )
        .unwrap();

    assert_eq!(result, "null");
}

#[test]
fn test_create_element() {
    let html = "";
    let (rt, _dom) = create_test_env(html);

    let result = rt
        .execute_script(
            "
        var el = document.createElement('span');
        el.textContent = 'Works';
        el.tagName + ':' + el.textContent
    ",
        )
        .unwrap();

    assert_eq!(result, "span:Works");
}

#[test]
fn test_document_body() {
    let html = "<html><body><h1>Hello</h1></body></html>";
    let (rt, _dom) = create_test_env(html);

    let result = rt
        .execute_script(
            "
        document.body.tagName
    ",
        )
        .unwrap();

    assert_eq!(result, "body");
}

#[test]
fn test_append_child() {
    let html = "<html><body></body></html>";
    let (rt, _dom) = create_test_env(html);

    let result = rt
        .execute_script(
            "
        var p = document.createElement('p');
        p.textContent = 'Appended';
        document.body.appendChild(p);
        document.body.textContent
    ",
        )
        .unwrap();

    assert_eq!(result.trim(), "Appended");
}

#[test]
fn test_attributes() {
    let html = "<html><body><div id='mydiv'></div></body></html>";
    let (rt, _dom) = create_test_env(html);

    let result = rt
        .execute_script(
            "
        var div = document.getElementById('mydiv');
        div.setAttribute('class', 'container');
        div.setAttribute('data-test', '123');
        div.getAttribute('class') + '|' + div.getAttribute('data-test')
    ",
        )
        .unwrap();

    assert_eq!(result, "container|123");
}

#[test]
fn test_remove_child() {
    let html = "<html><body><div id='toremove'>Remove Me</div></body></html>";
    let (rt, _dom) = create_test_env(html);

    let result = rt
        .execute_script(
            "
        var div = document.getElementById('toremove');
        var removed = document.body.removeChild(div);
        (document.getElementById('toremove') === null) + '|' + removed.tagName
    ",
        )
        .unwrap();

    assert_eq!(result, "true|div");
}

#[test]
fn test_query_selector() {
    let html = "<html><body><div class='foo'>A</div><div class='foo'>B</div></body></html>";
    let (rt, _dom) = create_test_env(html);

    let result = rt
        .execute_script(
            "
        var first = document.querySelector('.foo');
        var all = document.querySelectorAll('.foo');
        first.textContent + '|' + all.length + '|' + all[1].textContent
    ",
        )
        .unwrap();

    assert_eq!(result, "A|2|B");
}

#[test]
fn test_class_list() {
    let html = "<html><body><div id='btn' class='btn'></div></body></html>";
    let (rt, _dom) = create_test_env(html);

    let result = rt
        .execute_script(
            "
        var el = document.getElementById('btn');
        el.classList.add('primary');
        var hasPrimary = el.classList.contains('primary');
        el.classList.remove('btn');
        var hasBtn = el.classList.contains('btn');
        el.classList.toggle('active');
        
        var attr = el.getAttribute('class');
        hasPrimary + '|' + hasBtn + '|' + attr
    ",
        )
        .unwrap();

    let s = result;
    assert!(s.starts_with("true|false|"));
    assert!(s.contains("primary"));
    assert!(s.contains("active"));
    assert!(!s.contains("btn"));
}

#[test]
fn test_style() {
    let html = "<html><body><div id='box'></div></body></html>";
    let (rt, _dom) = create_test_env(html);

    let result = rt
        .execute_script(
            "
        var el = document.getElementById('box');
        el.style.setProperty('color', 'red');
        el.style.setProperty('width', '100px');
        
        var color = el.style.getPropertyValue('color');
        var cssText = el.getAttribute('style'); 
        
        el.style.removeProperty('color');
        var color2 = el.style.getPropertyValue('color');
        
        color + '|' + (color2 === '') + '|' + cssText.includes('width: 100px')
    ",
        )
        .unwrap();

    let s = result;
    assert_eq!(s, "red|true|true");
}

#[test]
fn test_event_listener() {
    let html = "<div id='btn'>Click me</div>";
    let (rt, _dom) = create_test_env(html);

    crate::runtime::bindings::html::event::EventTargetImpl::clear_all();

    let result = rt
        .execute_script(
            "
        var btn = document.getElementById('btn');
        var clicked = 0;
        
        function onClick(e) {
            clicked++;
        }
        
        btn.addEventListener('click', onClick);
        
        var event = new Event('click');
        btn.dispatchEvent(event);
        
        btn.removeEventListener('click', onClick);
        btn.dispatchEvent(event); 
        
        var docClicked = false;
        document.addEventListener('custom', function() { docClicked = true; });
        var docEvent = new Event('custom');
        document.dispatchEvent(docEvent);
        
        clicked + '|' + docClicked
    ",
        )
        .unwrap();

    assert_eq!(result, "1|true");
}

#[test]
fn test_event_bubbling() {
    let html = "<div id='parent'><div id='child'></div></div>";
    let (rt, _dom) = create_test_env(html);
    crate::runtime::bindings::html::event::EventTargetImpl::clear_all();

    let result = rt
        .execute_script(
            "
        var parent = document.getElementById('parent');
        var child = document.getElementById('child');
        var log = [];
        
        function onChild(e) {
            log.push('child');
        }
        
        function onParent(e) {
            log.push('parent');
        }
        
        function onDoc(e) {
            log.push('document');
        }
        
        child.addEventListener('click', onChild);
        parent.addEventListener('click', onParent);
        document.addEventListener('click', onDoc);
        
        var event = new Event('click', { bubbles: true });
        child.dispatchEvent(event);
        
        log.join('|')
    ",
        )
        .unwrap();

    assert_eq!(result, "child|parent|document");
}
