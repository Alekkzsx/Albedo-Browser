#[cfg(test)]
mod tests {
    use crate::engine::AceEngine;
    use crate::engine::layout::Dimensions;

    #[test]
    fn test_flexbox_demo_layout() {
        let mut engine = AceEngine::new();
        let html = r#"
            <div style="display: flex; flex-direction: row; justify-content: space-around; width: 300px; height: 100px;">
                <div style="width: 50px; height: 50px; background-color: red;"></div>
                <div style="width: 50px; height: 50px; background-color: blue;"></div>
            </div>
        "#;
        engine.load_html(html);
        
        if let Some(dom) = &engine.dom {
            let mut layout_root = crate::engine::layout::build_layout_tree(&dom.root, &engine.stylesheet.lock().unwrap()).unwrap();
            let mut viewport = Dimensions::default();
            viewport.content.width = 300.0;
            viewport.content.height = 100.0;
            
            layout_root.layout(viewport);
            
            let debug_output = layout_root.render_debug(0);
            println!("{}", debug_output);
            
            // The layout root is <html>. We need to find the container div.
            // Structure: html -> body -> div
            let container = &layout_root.children[1].children[0];
            assert_eq!(container.children.len(), 2);
            
            let child1 = &container.children[0];
            let child2 = &container.children[1];
            
            assert!(child1.dimensions.content.x >= 49.0 && child1.dimensions.content.x <= 51.0);
            assert!(child2.dimensions.content.x >= 199.0 && child2.dimensions.content.x <= 201.0);
        }
    }
}
