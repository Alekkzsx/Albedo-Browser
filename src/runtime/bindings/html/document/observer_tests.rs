#[cfg(test)]
mod tests {
    use crate::engine::dom::AceDOM;
    use crate::engine::style;
    use crate::runtime::bindings::html::document::register;
    use crate::runtime::core::runtime::JsRuntime;
    use kuchiki::traits::TendrilSink;
    use std::sync::{Arc, Mutex};

    fn create_test_env(html: &str) -> (JsRuntime, Arc<Mutex<AceDOM>>) {
        let document = kuchiki::parse_html().one(html);
        let dom = Arc::new(Mutex::new(AceDOM::from_kuchiki(document)));
        let rt = JsRuntime::new().unwrap();

        let stylesheet = Arc::new(Mutex::new(style::parse("")));

        register(
            &rt,
            dom.clone(),
            stylesheet,
            Arc::new(Mutex::new(Vec::new())),
            Arc::new(Mutex::new(std::collections::HashMap::new())),
            "http://localhost".to_string(),
            "".to_string(),
            None,
        )
        .unwrap();
        rt.context.lock().unwrap().with(|ctx| {
            let global = ctx.globals();
            rquickjs::Class::<crate::runtime::bindings::html::mutation_observer::MutationObserver>::define(&global).unwrap();
        });

        (rt, dom)
    }

    #[test]
    fn test_mutation_observer_attributes() {
        let html = r#"<div id="target"></div>"#;
        let (rt, _dom) = create_test_env(html);

        let result = rt
            .execute_script(
                r#"
            var target = document.getElementById('target');
            var result = "";
            var observer = new MutationObserver(function(mutations) {
                mutations.forEach(function(mutation) {
                    result = mutation.type + ":" + mutation.attributeName + ":" + mutation.oldValue;
                });
            });
            observer.observe(target, { attributes: true, attributeOldValue: true });
            target.setAttribute('class', 'active');
            
            // Pulse to trigger observer (microtasks/mutations)
            // No Albedo, run_pending processa mutações
            "OK"
        "#,
            )
            .unwrap();

        // Manual pulse
        rt.run_pending();

        let final_result = rt.execute_script("result").unwrap();
        assert_eq!(final_result, "attributes:class:null");

        rt.execute_script("target.setAttribute('class', 'inactive');")
            .unwrap();
        rt.run_pending();
        let final_result2 = rt.execute_script("result").unwrap();
        assert_eq!(final_result2, "attributes:class:active");
    }

    #[test]
    fn test_mutation_observer_child_list() {
        let html = r#"<div id="parent"></div>"#;
        let (rt, _dom) = create_test_env(html);

        rt.execute_script(
            r#"
            var parent = document.getElementById('parent');
            var log = "";
            var observer = new MutationObserver(function(mutations) {
                mutations.forEach(function(mutation) {
                    log += "added:" + mutation.addedNodes.length + ";";
                    if (mutation.addedNodes.length > 0) {
                        log += "tagName:" + mutation.addedNodes[0].tagName + ";";
                    }
                });
            });
            observer.observe(parent, { childList: true });
            var child = document.createElement('span');
            parent.appendChild(child);
        "#,
        )
        .unwrap();

        rt.run_pending();

        let log = rt.execute_script("log").unwrap();
        assert!(log.contains("added:1"));
        assert!(log.contains("tagName:SPAN"));
    }

    #[test]
    fn test_mutation_observer_subtree() {
        let html = r#"<div id="root"><div id="child"></div></div>"#;
        let (rt, _dom) = create_test_env(html);

        rt.execute_script(
            r#"
            var root = document.getElementById('root');
            var child = document.getElementById('child');
            var count = 0;
            var observer = new MutationObserver(function(mutations) {
                count += mutations.length;
            });
            observer.observe(root, { attributes: true, subtree: true });
            child.setAttribute('data-test', 'v1');
        "#,
        )
        .unwrap();

        rt.run_pending();

        let count = rt.execute_script("count").unwrap();
        assert_eq!(count, "1");
    }

    #[test]
    fn test_mutation_observer_take_records() {
        let html = r#"<div id="target"></div>"#;
        let (rt, _dom) = create_test_env(html);

        let result = rt
            .execute_script(
                r#"
            var target = document.getElementById('target');
            var observer = new MutationObserver(function(mutations) {});
            observer.observe(target, { attributes: true });
            target.setAttribute('id', 'new-id');
            var records = observer.takeRecords();
            records.length + ":" + records[0].type + ":" + records[0].attributeName
        "#,
            )
            .unwrap();

        assert_eq!(result, "1:attributes:id");

        // After takeRecords, callback shouldn't be called for those records
        rt.run_pending();
        // Since the callback is empty, we just verify it doesn't crash
    }
}
