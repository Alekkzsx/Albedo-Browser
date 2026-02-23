use std::sync::{Arc, Mutex};
use albedo::engine::mod::*; // Ajustar os imports conforme o projeto, ou usar os paths diretos
use albedo::engine::dom::{AceDOM, AceNode, AceNodeType};
use albedo::engine::{AceEngine, ElementGeometry};

#[test]
fn test_collect_subframe_geometries() {
    let mut parent_engine = AceEngine::new();
    
    // Configurar o DOM pai
    let mut parent_dom = AceDOM::new();
    
    // Adicionar um iframe no DOM pai (node_idx = 1)
    let iframe_idx = 1;
    parent_dom.nodes.push(AceNode {
        node_type: AceNodeType::Element(albedo::engine::dom::AceElement {
            tag: "iframe".to_string(),
            attributes: std::collections::HashMap::new(),
        }),
        parent: Some(0),
        children: vec![],
        prev_sibling: None,
        next_sibling: None,
        shadow_root: None,
    });
    
    // Configurar o subframe (AceEngine filho)
    let sub_engine = Arc::new(Mutex::new(AceEngine::new()));
    {
        let mut sub = sub_engine.lock().unwrap();
        let mut sub_dom = AceDOM::new();
        sub_dom.iframe_node_idx = Some(iframe_idx); // Marcamos este DOM como sendo o iframe 1
        
        // Elemento dentro do subframe (node_idx = 1 no subframe)
        sub_dom.nodes.push(AceNode {
            node_type: AceNodeType::Element(albedo::engine::dom::AceElement {
                tag: "div".to_string(),
                attributes: std::collections::HashMap::new(),
            }),
            parent: Some(0),
            children: vec![],
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
        });
        
        sub.dom = Some(Arc::new(Mutex::new(sub_dom)));
        
        // Simular a geometria do elemento no subframe: x=50, y=50, w=100, h=100
        let mut sub_geom = sub.element_geometry.lock().unwrap();
        sub_geom.insert(1, ElementGeometry {
            x: 50.0,
            y: 50.0,
            width: 100.0,
            height: 100.0,
            content_width: 100.0,
            content_height: 100.0,
        });
    }
    
    // Registrar o subframe no DOM pai
    let mut subframes_map = std::collections::HashMap::new();
    subframes_map.insert(iframe_idx, sub_engine.clone());
    parent_dom.subframes = Some(Arc::new(Mutex::new(subframes_map)));
    
    parent_engine.dom = Some(Arc::new(Mutex::new(parent_dom)));
    
    // Simular a geometria do iframe no frame pai: x=200, y=300, w=500, h=400
    {
        let mut parent_geom = parent_engine.element_geometry.lock().unwrap();
        parent_geom.insert(iframe_idx, ElementGeometry {
            x: 200.0,
            y: 300.0,
            width: 500.0,
            height: 400.0,
            content_width: 500.0,
            content_height: 400.0,
        });
    }
    
    // Executar a coleta de geometrias dos subframes
    parent_engine.collect_subframe_geometries();
    
    // Verificar se a projeção foi calculada corretamente
    let projected = parent_engine.iframe_projected_geometry.lock().unwrap();
    
    // Chave: iframe_idx (1) * 1_000_000 + elem_idx (1) = 1000001
    let key = 1_000_001;
    assert!(projected.contains_key(&key), "O elemento projetado deve existir");
    
    let geom = projected.get(&key).unwrap();
    // Elemento interno x=50, y=50. Iframe x=200, y=300 => Projetado: x=250, y=350
    assert_eq!(geom.x, 250.0);
    assert_eq!(geom.y, 350.0);
    assert_eq!(geom.width, 100.0);
    assert_eq!(geom.height, 100.0);
}
