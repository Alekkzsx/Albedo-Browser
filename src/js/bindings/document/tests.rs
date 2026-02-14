#[cfg(test)]
mod tests {
    use super::super::*; // Access Document from mod.rs
    use crate::engine::parser;
    use crate::js::JsRuntime;
    use std::sync::{Arc, Mutex};
    use crate::engine::style::Stylesheet;

    #[test]
    fn test_get_element_by_id() {
        let html = r#"<div id="test_div">Hello World</div>"#;
        let dom = parser::parse_html(html);
        
        let rt = JsRuntime::new().unwrap();
        register(&rt, dom, Arc::new(Mutex::new(Stylesheet::parse("")))).unwrap();
        
        let result = rt.execute_script("
            var el = document.getElementById('test_div');
            el.tagName + ':' + el.textContent
        ").unwrap();
        
        assert_eq!(result, "DIV:Hello World");
    }
    
    #[test]
    fn test_get_element_by_id_null() {
        let html = r#"<div></div>"#;
        let dom = parser::parse_html(html);
        
        let rt = JsRuntime::new().unwrap();
        register(&rt, dom, Arc::new(Mutex::new(Stylesheet::parse("")))).unwrap();
        
        let result = rt.execute_script("
            var el = document.getElementById('non_existent');
            el === null ? 'null' : 'not null'
        ").unwrap();
        
        assert_eq!(result, "null");
    }

    #[test]
    fn test_create_element() {
        let html = "";
        let dom = parser::parse_html(html);
        let rt = JsRuntime::new().unwrap();
        register(&rt, dom, Arc::new(Mutex::new(Stylesheet::parse("")))).unwrap();
        
        let result = rt.execute_script("
            var el = document.createElement('span');
            el.textContent = 'Works';
            el.tagName + ':' + el.textContent
        ").unwrap();
        
        assert_eq!(result, "SPAN:Works");
    }

    #[test]
    fn test_document_body() {
        let html = "<html><body><h1>Hello</h1></body></html>";
        let dom = parser::parse_html(html);
        let rt = JsRuntime::new().unwrap();
        register(&rt, dom, Arc::new(Mutex::new(Stylesheet::parse("")))).unwrap();
        
        let result = rt.execute_script("
            document.body.tagName
        ").unwrap();
        
        assert_eq!(result, "BODY");
    }

    #[test]
    fn test_append_child() {
        let html = "<html><body></body></html>";
        let dom = parser::parse_html(html);
        let rt = JsRuntime::new().unwrap();
        register(&rt, dom, Arc::new(Mutex::new(Stylesheet::parse("")))).unwrap();
        
        let result = rt.execute_script("
            var p = document.createElement('p');
            p.textContent = 'Appended';
            document.body.appendChild(p);
            // document.body.children unsupported properties.
            // Check innerHTML or textContent aproximation
            document.body.textContent
        ").unwrap();
        
        // kuchiki text_contents should include appended text
        assert_eq!(result.trim(), "Appended");
    }

    #[test]
    fn test_attributes() {
        let html = "<html><body><div id='mydiv'></div></body></html>";
        let dom = parser::parse_html(html);
        let rt = JsRuntime::new().unwrap();
        register(&rt, dom, Arc::new(Mutex::new(Stylesheet::parse("")))).unwrap();
        
        let result = rt.execute_script("
            var div = document.getElementById('mydiv');
            div.setAttribute('class', 'container');
            div.setAttribute('data-test', '123');
            div.getAttribute('class') + '|' + div.getAttribute('data-test')
        ").unwrap();
        
        assert_eq!(result, "container|123");
    }

    #[test]
    fn test_remove_child() {
        let html = "<html><body><div id='toremove'>Remove Me</div></body></html>";
        let dom = parser::parse_html(html);
        let rt = JsRuntime::new().unwrap();
        register(&rt, dom, Arc::new(Mutex::new(Stylesheet::parse("")))).unwrap();
        
        let result = rt.execute_script("
            var div = document.getElementById('toremove');
            var removed = document.body.removeChild(div);
            (document.getElementById('toremove') === null) + '|' + removed.tagName
        ").unwrap();
        
        // When removed from DOM, getElementById might still find it if we don't clear from index?
        // Wait, DomTree.find_by_id uses selector which traverses the tree.
        // If detached, it shouldn't be found starting from root.
        assert_eq!(result, "true|DIV");
    }

    #[test]
    fn test_inner_html_set() {
        let html = "<html><body><div id='container'></div></body></html>";
        let dom = parser::parse_html(html);
        let rt = JsRuntime::new().unwrap();
        register(&rt, dom, Arc::new(Mutex::new(Stylesheet::parse("")))).unwrap();
        
        let result = rt.execute_script("
            var div = document.getElementById('container');
            div.innerHTML = '<p>Hello</p><span>World</span>';
            div.children // Not implemented yet
            // Check text content or query
            var p = div.querySelector('p');
            var span = div.querySelector('span');
            p.textContent + '|' + span.textContent
        ").unwrap();
        
        assert_eq!(result, "Hello|World");
    }

    #[test]
    fn test_query_selector() {
        let html = "<html><body><div class='foo'>A</div><div class='foo'>B</div></body></html>";
        let dom = parser::parse_html(html);
        let rt = JsRuntime::new().unwrap();
        register(&rt, dom, Arc::new(Mutex::new(Stylesheet::parse("")))).unwrap();
        
        let result = rt.execute_script("
            var first = document.querySelector('.foo');
            var all = document.querySelectorAll('.foo');
            first.textContent + '|' + all.length + '|' + all[1].textContent
        ").unwrap();
        
        assert_eq!(result, "A|2|B");
    }
    #[test]
    fn test_class_list() {
        let html = "<html><body><div id='btn' class='btn'></div></body></html>";
        let dom = parser::parse_html(html);
        let rt = JsRuntime::new().unwrap();
        register(&rt, dom, Arc::new(Mutex::new(Stylesheet::parse("")))).unwrap();
        
        let result = rt.execute_script("
            var el = document.getElementById('btn');
            el.classList.add('primary');
            var hasPrimary = el.classList.contains('primary');
            el.classList.remove('btn');
            var hasBtn = el.classList.contains('btn');
            el.classList.toggle('active');
            
            // Check final class attribute
            var attr = el.getAttribute('class');
            hasPrimary + '|' + hasBtn + '|' + attr
        ").unwrap();
        
        let s = result;
        assert!(s.starts_with("true|false|"));
        assert!(s.contains("primary"));
        assert!(s.contains("active"));
        assert!(!s.contains("btn"));
    }

    #[test]
    fn test_style() {
        let html = "<html><body><div id='box'></div></body></html>";
        let dom = parser::parse_html(html);
        let rt = JsRuntime::new().unwrap();
        register(&rt, dom, Arc::new(Mutex::new(Stylesheet::parse("")))).unwrap();
        
        let result = rt.execute_script("
            var el = document.getElementById('box');
            el.style.setProperty('color', 'red');
            el.style.setProperty('width', '100px');
            
            var color = el.style.getPropertyValue('color');
            var cssText = el.getAttribute('style'); 
            
            el.style.removeProperty('color');
            var color2 = el.style.getPropertyValue('color');
            
            color + '|' + (color2 === '') + '|' + cssText.includes('width: 100px')
        ").unwrap();
        
        let s = result;
        assert_eq!(s, "red|true|true");
    }

    #[test]
    fn test_event_listener() {
        let html = "<div id='btn'>Click me</div>";
        let dom = parser::parse_html(html);
        let rt = JsRuntime::new().unwrap();
        register(&rt, dom, Arc::new(Mutex::new(Stylesheet::parse("")))).unwrap();
        
        let result = rt.execute_script("
            var btn = document.getElementById('btn');
            var clicked = 0;
            
            function onClick(e) {
                clicked++;
            }
            
            btn.addEventListener('click', onClick);
            
            var event = new Event('click');
            btn.dispatchEvent(event);
            
            // Test remove
            btn.removeEventListener('click', onClick);
            btn.dispatchEvent(event); // Should not trigger
            
            // Test document event
            var docClicked = false;
            document.addEventListener('custom', function() { docClicked = true; });
            var docEvent = new Event('custom');
            document.dispatchEvent(docEvent);
            
            clicked + '|' + docClicked
        ").unwrap();
        
        crate::js::bindings::event::EventTargetImpl::clear_all();
        assert_eq!(result, "1|true");
    }

    #[test]
    fn test_event_bubbling() {
        let html = "<div id='parent'><div id='child'></div></div>";
        let dom = parser::parse_html(html);
        let rt = JsRuntime::new().unwrap();
        register(&rt, dom, Arc::new(Mutex::new(Stylesheet::parse("")))).unwrap();
        
        let result = rt.execute_script("
            var parent = document.getElementById('parent');
            var child = document.getElementById('child');
            var log = [];
            
            function onChild(e) {
                log.push('child');
                // Check target is correct by ID (identity check fails as wrappers are not cached yet)
                if (e.target.getAttribute('id') !== 'child') log.push('wrong_target');
            }
            
            function onParent(e) {
                log.push('parent');
                if (e.target.getAttribute('id') !== 'child') log.push('wrong_target_in_parent');
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
        ").unwrap();
        
        crate::js::bindings::event::EventTargetImpl::clear_all();
        assert_eq!(result, "child|parent|document");
    }
}
