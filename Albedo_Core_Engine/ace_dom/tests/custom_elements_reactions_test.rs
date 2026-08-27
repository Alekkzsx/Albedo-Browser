//! # Suíte de Testes de Reações de Ciclo de Vida e `[CEReactions]` (ace_dom)
//!
//! Cobre:
//! 1. Pilha de reações (`CustomElementReactionsStack`, frames de pilha e filas ordenadas)
//! 2. Mutações da árvore (`append_child`, `insert_before`, `remove_child`, `replace_child`) disparando `connectedCallback` e `disconnectedCallback`
//! 3. Ordem da árvore (Preorder DFS) na conexão e desconexão de subárvores inteiras
//! 4. Alterações e remoções de atributos observados disparando `attributeChangedCallback`
//! 5. Reentrância em mutações secundárias acionadas dentro dos próprios callbacks
//! 6. Teste de estresse com centenas de nós customizados em lote

use ace_core::id::NodeId;
use ace_dom::custom_elements::{
    CustomElementCallbacks, CustomElementDefinition, CustomElementReaction,
    CustomElementReactionsStack, CustomElementRegistry, CustomElementState,
};
use ace_dom::node::element::Namespace;
use ace_dom::tree::mutation::{
    append_child_with_reactions, insert_after_with_reactions, insert_before_with_reactions,
    prepend_child_with_reactions, remove_child_with_reactions, replace_child_with_reactions,
};
use ace_dom::tree::Document;
use smol_str::SmolStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[test]
fn test_reaction_stack_push_pop_drain() {
    let mut stack = CustomElementReactionsStack::new();
    assert!(stack.is_empty());
    assert_eq!(stack.len(), 0);

    let n1 = NodeId::new();
    let n2 = NodeId::new();

    // 1. Enfileiramento em fila plana (sem frame de pilha explícito)
    stack.enqueue_connected(n1);
    stack.enqueue_disconnected(n2);
    assert_eq!(stack.len(), 2);
    assert!(!stack.is_empty());

    let drained = stack.drain_all();
    assert_eq!(drained.len(), 2);
    assert_eq!(drained[0], CustomElementReaction::Connected(n1));
    assert_eq!(drained[1], CustomElementReaction::Disconnected(n2));
    assert!(stack.is_empty());

    // 2. Enfileiramento com frames de pilha [CEReactions]
    stack.push_stack_frame();
    stack.enqueue_attribute_changed(n1, "theme", None, Some(SmolStr::new("dark")));
    stack.enqueue_connected(n2);
    assert_eq!(stack.len(), 2);

    let popped = stack.pop_stack_frame().expect("frame desempilhado");
    assert_eq!(popped.len(), 2);
    assert!(popped.contains(&n1));
    assert!(popped.contains(&n2));

    let drained2 = stack.drain_all();
    assert_eq!(drained2.len(), 2);
    assert!(stack.is_empty());
}

#[test]
fn test_connected_callback_on_tree_insertion_and_ordering() {
    let mut doc = Document::new(None);
    let mut registry = CustomElementRegistry::new();
    let mut reactions = CustomElementReactionsStack::new();

    let connection_log = Arc::new(Mutex::new(Vec::<String>::new()));
    let log_c = connection_log.clone();

    // Registra "parent-widget" e "child-widget" com callbacks que gravam o log
    let callbacks_parent = CustomElementCallbacks::new().with_connected({
        let log = log_c.clone();
        move |_id| {
            log.lock().unwrap().push("parent-connected".to_string());
        }
    });

    let callbacks_child = CustomElementCallbacks::new().with_connected({
        let log = log_c.clone();
        move |_id| {
            log.lock().unwrap().push("child-connected".to_string());
        }
    });

    registry
        .define_with_callbacks("parent-widget", &[], callbacks_parent)
        .unwrap();
    registry
        .define_with_callbacks("child-widget", &[], callbacks_child)
        .unwrap();

    // Cria nós e marca como Custom
    let parent_elem = doc.create_element("parent-widget", Namespace::Html);
    let child1 = doc.create_element("child-widget", Namespace::Html);
    let child2 = doc.create_element("child-widget", Namespace::Html);

    if let Some(el) = doc.get_node_mut(parent_elem).and_then(|n| n.as_element_mut()) {
        el.set_custom_element_state(CustomElementState::Custom);
    }
    if let Some(el) = doc.get_node_mut(child1).and_then(|n| n.as_element_mut()) {
        el.set_custom_element_state(CustomElementState::Custom);
    }
    if let Some(el) = doc.get_node_mut(child2).and_then(|n| n.as_element_mut()) {
        el.set_custom_element_state(CustomElementState::Custom);
    }

    // 1. Monta a subárvore fora do documento: parent -> child1, child2
    // Não deve disparar reações de conexão pois a subárvore ainda está desconectada!
    append_child_with_reactions(&mut doc.arena, parent_elem, child1, &mut reactions).unwrap();
    append_child_with_reactions(&mut doc.arena, parent_elem, child2, &mut reactions).unwrap();
    reactions.process_reactions(&doc.arena, &registry);

    assert!(connection_log.lock().unwrap().is_empty(), "Não deve disparar em árvore desconectada");

    // 2. Anexa a subárvore completa à raiz do Document
    // Deve disparar connectedCallback em ordem pré-ordem (parent -> child1 -> child2)
    append_child_with_reactions(&mut doc.arena, doc.root(), parent_elem, &mut reactions).unwrap();
    reactions.process_reactions(&doc.arena, &registry);

    let log_result = connection_log.lock().unwrap().clone();
    assert_eq!(
        log_result,
        vec![
            "parent-connected".to_string(),
            "child-connected".to_string(),
            "child-connected".to_string()
        ],
        "Reações de connectedCallback devem ser executadas em ordem de pré-ordem DFS"
    );
}

#[test]
fn test_disconnected_callback_on_node_removal_and_replace() {
    let mut doc = Document::new(None);
    let mut registry = CustomElementRegistry::new();
    let mut reactions = CustomElementReactionsStack::new();

    let disconnection_log = Arc::new(Mutex::new(Vec::<String>::new()));
    let log_d = disconnection_log.clone();

    let callbacks_tree = CustomElementCallbacks::new().with_disconnected({
        let log = log_d.clone();
        move |id| {
            log.lock().unwrap().push(format!("disconnected-{:?}", id));
        }
    });

    registry
        .define_with_callbacks("tree-node", &[], callbacks_tree)
        .unwrap();

    let root_node = doc.create_element("tree-node", Namespace::Html);
    let branch_node = doc.create_element("tree-node", Namespace::Html);
    let leaf_node = doc.create_element("tree-node", Namespace::Html);

    for &id in &[root_node, branch_node, leaf_node] {
        if let Some(el) = doc.get_node_mut(id).and_then(|n| n.as_element_mut()) {
            el.set_custom_element_state(CustomElementState::Custom);
        }
    }

    // Monta: Document -> root_node -> branch_node -> leaf_node
    append_child_with_reactions(&mut doc.arena, branch_node, leaf_node, &mut reactions).unwrap();
    append_child_with_reactions(&mut doc.arena, root_node, branch_node, &mut reactions).unwrap();
    append_child_with_reactions(&mut doc.arena, doc.root(), root_node, &mut reactions).unwrap();
    reactions.drain_all(); // Limpa reações de conexão

    // Remove root_node da raiz do documento:
    // Deve disparar desconexão para root_node, branch_node e leaf_node em ordem da árvore
    remove_child_with_reactions(&mut doc.arena, doc.root(), root_node, &mut reactions).unwrap();
    reactions.process_reactions(&doc.arena, &registry);

    let log_result = disconnection_log.lock().unwrap().clone();
    assert_eq!(log_result.len(), 3);
    assert_eq!(log_result[0], format!("disconnected-{:?}", root_node));
    assert_eq!(log_result[1], format!("disconnected-{:?}", branch_node));
    assert_eq!(log_result[2], format!("disconnected-{:?}", leaf_node));

    // Teste de replace_child com reações
    let new_node = doc.create_element("tree-node", Namespace::Html);
    if let Some(el) = doc.get_node_mut(new_node).and_then(|n| n.as_element_mut()) {
        el.set_custom_element_state(CustomElementState::Custom);
    }
    append_child_with_reactions(&mut doc.arena, doc.root(), root_node, &mut reactions).unwrap();
    reactions.drain_all();

    disconnection_log.lock().unwrap().clear();
    replace_child_with_reactions(&mut doc.arena, doc.root(), new_node, root_node, &mut reactions).unwrap();

    let queued_reactions = reactions.drain_all();
    assert!(queued_reactions.contains(&CustomElementReaction::Disconnected(root_node)));
    assert!(queued_reactions.contains(&CustomElementReaction::Connected(new_node)));
}

#[test]
fn test_attribute_changed_callback_lifecycle_and_filters() {
    let mut doc = Document::new(None);
    let mut registry = CustomElementRegistry::new();
    let mut reactions = CustomElementReactionsStack::new();

    let attr_log = Arc::new(Mutex::new(Vec::<(String, Option<String>, Option<String>)>::new()));
    let log_a = attr_log.clone();

    let callbacks = CustomElementCallbacks::new().with_attribute_changed({
        let log = log_a.clone();
        move |_id, name, old, new| {
            log.lock().unwrap().push((
                name.to_string(),
                old.map(String::from),
                new.map(String::from),
            ));
        }
    });

    registry
        .define_with_callbacks("config-panel", &["theme", "mode"], callbacks)
        .unwrap();

    let panel_id = doc.create_element("config-panel", Namespace::Html);
    if let Some(el) = doc.get_node_mut(panel_id).and_then(|n| n.as_element_mut()) {
        el.set_custom_element_state(CustomElementState::Custom);
    }

    // 1. Inserção de atributo observado ("theme" = "dark")
    if let Some(el) = doc.get_node_mut(panel_id).and_then(|n| n.as_element_mut()) {
        el.set_attribute_with_reaction(panel_id, "theme", "dark", &mut reactions, &registry);
    }
    reactions.process_reactions(&doc.arena, &registry);

    // 2. Modificação de atributo observado ("theme" de "dark" para "light")
    if let Some(el) = doc.get_node_mut(panel_id).and_then(|n| n.as_element_mut()) {
        el.set_attribute_with_reaction(panel_id, "theme", "light", &mut reactions, &registry);
    }
    reactions.process_reactions(&doc.arena, &registry);

    // 3. Atribuição de valor idêntico ("theme" = "light") -> NÃO deve gerar reação
    if let Some(el) = doc.get_node_mut(panel_id).and_then(|n| n.as_element_mut()) {
        el.set_attribute_with_reaction(panel_id, "theme", "light", &mut reactions, &registry);
    }
    reactions.process_reactions(&doc.arena, &registry);

    // 4. Atribuição de atributo não-observado ("unobserved" = "foo") -> NÃO deve gerar reação
    if let Some(el) = doc.get_node_mut(panel_id).and_then(|n| n.as_element_mut()) {
        el.set_attribute_with_reaction(panel_id, "unobserved", "foo", &mut reactions, &registry);
    }
    reactions.process_reactions(&doc.arena, &registry);

    // 5. Remoção de atributo observado ("theme")
    if let Some(el) = doc.get_node_mut(panel_id).and_then(|n| n.as_element_mut()) {
        el.remove_attribute_with_reaction(panel_id, "theme", &mut reactions, &registry);
    }
    reactions.process_reactions(&doc.arena, &registry);

    // 6. Remoção de atributo não-observado -> NÃO deve gerar reação
    if let Some(el) = doc.get_node_mut(panel_id).and_then(|n| n.as_element_mut()) {
        el.remove_attribute_with_reaction(panel_id, "unobserved", &mut reactions, &registry);
    }
    reactions.process_reactions(&doc.arena, &registry);

    let log_result = attr_log.lock().unwrap().clone();
    assert_eq!(log_result.len(), 3);
    assert_eq!(
        log_result[0],
        ("theme".to_string(), None, Some("dark".to_string()))
    );
    assert_eq!(
        log_result[1],
        ("theme".to_string(), Some("dark".to_string()), Some("light".to_string()))
    );
    assert_eq!(
        log_result[2],
        ("theme".to_string(), Some("light".to_string()), None)
    );
}

#[test]
fn test_insert_after_and_prepend_with_reactions() {
    let mut doc = Document::new(None);
    let mut reactions = CustomElementReactionsStack::new();

    let parent_id = doc.create_element("div", Namespace::Html);
    let ref_node = doc.create_element("span", Namespace::Html);
    let custom_first = doc.create_element("x-item", Namespace::Html);
    let custom_after = doc.create_element("x-item", Namespace::Html);

    for &id in &[custom_first, custom_after] {
        if let Some(el) = doc.get_node_mut(id).and_then(|n| n.as_element_mut()) {
            el.set_custom_element_state(CustomElementState::Custom);
        }
    }

    doc.append_child(doc.root(), parent_id).unwrap();
    doc.append_child(parent_id, ref_node).unwrap();

    // Prepend
    prepend_child_with_reactions(&mut doc.arena, parent_id, custom_first, &mut reactions).unwrap();
    // Insert after
    insert_after_with_reactions(&mut doc.arena, parent_id, custom_after, ref_node, &mut reactions).unwrap();

    let drained = reactions.drain_all();
    assert!(drained.contains(&CustomElementReaction::Connected(custom_first)));
    assert!(drained.contains(&CustomElementReaction::Connected(custom_after)));
}

#[test]
fn test_custom_elements_stress_500_nodes() {
    let mut doc = Document::new(None);
    let mut registry = CustomElementRegistry::new();
    let mut reactions = CustomElementReactionsStack::new();

    let connected_total = Arc::new(AtomicUsize::new(0));
    let disconnected_total = Arc::new(AtomicUsize::new(0));
    let attr_total = Arc::new(AtomicUsize::new(0));

    let c_tot = connected_total.clone();
    let d_tot = disconnected_total.clone();
    let a_tot = attr_total.clone();

    let callbacks = CustomElementCallbacks::new()
        .with_connected(move |_| {
            c_tot.fetch_add(1, Ordering::SeqCst);
        })
        .with_disconnected(move |_| {
            d_tot.fetch_add(1, Ordering::SeqCst);
        })
        .with_attribute_changed(move |_, _, _, _| {
            a_tot.fetch_add(1, Ordering::SeqCst);
        });

    registry
        .define_with_callbacks("stress-item", &["index", "status"], callbacks)
        .unwrap();

    let container_id = doc.create_element("div", Namespace::Html);
    doc.append_child(doc.root(), container_id).unwrap();

    const NODE_COUNT: usize = 500;
    let mut created_nodes = Vec::with_capacity(NODE_COUNT);

    for i in 0..NODE_COUNT {
        let node_id = doc.create_element("stress-item", Namespace::Html);
        if let Some(el) = doc.get_node_mut(node_id).and_then(|n| n.as_element_mut()) {
            el.set_custom_element_state(CustomElementState::Custom);
            el.set_attribute_with_reaction(
                node_id,
                "index",
                format!("{}", i),
                &mut reactions,
                &registry,
            );
        }
        append_child_with_reactions(&mut doc.arena, container_id, node_id, &mut reactions).unwrap();
        created_nodes.push(node_id);
    }

    // Processa todas as reações da criação e inserção em massa
    reactions.process_reactions(&doc.arena, &registry);

    assert_eq!(connected_total.load(Ordering::SeqCst), NODE_COUNT);
    assert_eq!(attr_total.load(Ordering::SeqCst), NODE_COUNT);

    // Remove todos os nós
    for node_id in created_nodes {
        remove_child_with_reactions(&mut doc.arena, container_id, node_id, &mut reactions).unwrap();
    }
    reactions.process_reactions(&doc.arena, &registry);

    assert_eq!(disconnected_total.load(Ordering::SeqCst), NODE_COUNT);
}
