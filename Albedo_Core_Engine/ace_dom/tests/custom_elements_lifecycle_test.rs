//! # Suíte de Testes de Ciclo de Vida e Validação de Custom Elements (ace_dom)
//!
//! Cobre exaustivamente:
//! 1. Validador WHATWG §4.13.1.2 PCENChar (ASCII, Unicode, caracteres reservados, maiúsculas, pontuação)
//! 2. `CustomElementRegistry` (`define`, `get`, `is_defined`, `when_defined`, duplicatas, erros sintáticos)
//! 3. `CustomElementState` em `ElementData` (Uncustomized, Undefined, Custom, Precustomized, Failed)
//! 4. Operações de `upgrade` em subárvores com elementos customizados e atributos observados pré-existentes

use ace_core::intern::Atom;
use ace_dom::custom_elements::{
    is_pcen_char, is_valid_custom_element_name, CustomElementCallbacks, CustomElementReaction,
    CustomElementReactionsStack, CustomElementRegistry, CustomElementState,
};
use ace_dom::node::element::Namespace;
use ace_dom::tree::Document;
use smol_str::SmolStr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn test_pcen_char_unit_checks() {
    // Caracteres básicos
    assert!(is_pcen_char('-'));
    assert!(is_pcen_char('.'));
    assert!(is_pcen_char('_'));
    assert!(is_pcen_char('a'));
    assert!(is_pcen_char('z'));
    assert!(is_pcen_char('0'));
    assert!(is_pcen_char('9'));

    // Caracteres Unicode normativos WHATWG §4.13.1.2
    assert!(is_pcen_char('\u{00B7}')); // Middle dot
    assert!(is_pcen_char('\u{00C0}')); // À
    assert!(is_pcen_char('\u{00D6}')); // Ö
    assert!(is_pcen_char('\u{00D8}')); // Ø
    assert!(is_pcen_char('\u{00F6}')); // ö
    assert!(is_pcen_char('\u{00E9}')); // é
    assert!(is_pcen_char('\u{00F8}')); // ø
    assert!(is_pcen_char('\u{037D}')); // Greek question mark
    assert!(is_pcen_char('\u{037F}')); // Greek yot
    assert!(is_pcen_char('\u{1FFF}'));
    assert!(is_pcen_char('\u{200C}')); // Zero-width non-joiner
    assert!(is_pcen_char('\u{200D}')); // Zero-width joiner
    assert!(is_pcen_char('\u{203F}')); // Undertie
    assert!(is_pcen_char('\u{2040}')); // Character tie
    assert!(is_pcen_char('\u{2070}')); // Superscript zero
    assert!(is_pcen_char('\u{218F}'));
    assert!(is_pcen_char('\u{2C00}')); // Glagolitic
    assert!(is_pcen_char('\u{2FEF}'));
    assert!(is_pcen_char('\u{3001}')); // Ideographic comma
    assert!(is_pcen_char('\u{65E5}')); // Kanji 日
    assert!(is_pcen_char('\u{D7FF}'));
    assert!(is_pcen_char('\u{F900}'));
    assert!(is_pcen_char('\u{FDCF}'));
    assert!(is_pcen_char('\u{FDF0}'));
    assert!(is_pcen_char('\u{FFFD}')); // Replacement char
    assert!(is_pcen_char('\u{10000}')); // Linear B
    assert!(is_pcen_char('\u{EFFFF}'));

    // Caracteres proibidos
    assert!(!is_pcen_char('A'));
    assert!(!is_pcen_char('Z'));
    assert!(!is_pcen_char(' '));
    assert!(!is_pcen_char('#'));
    assert!(!is_pcen_char('$'));
    assert!(!is_pcen_char('%'));
    assert!(!is_pcen_char('&'));
    assert!(!is_pcen_char('@'));
    assert!(!is_pcen_char('!'));
    assert!(!is_pcen_char('/'));
    assert!(!is_pcen_char(':'));
    assert!(!is_pcen_char(';'));
    assert!(!is_pcen_char('\u{00D7}')); // Multiplication sign (buraco entre 00D6 e 00D8)
    assert!(!is_pcen_char('\u{00F7}')); // Division sign (buraco entre 00F6 e 00F8)
    assert!(!is_pcen_char('\u{037E}')); // Greek question mark (buraco entre 037D e 037F)
}

#[test]
fn test_is_valid_custom_element_name_valid_cases() {
    // Padrões normais
    assert!(is_valid_custom_element_name("a-b"));
    assert!(is_valid_custom_element_name("x-tag"));
    assert!(is_valid_custom_element_name("user-profile"));
    assert!(is_valid_custom_element_name("super-awesome-button-2000"));
    assert!(is_valid_custom_element_name("my.custom-element_name"));
    assert!(is_valid_custom_element_name("app-main-drawer-v2"));

    // Nomes com caracteres Unicode válidos
    assert!(is_valid_custom_element_name("my-élément"));
    assert!(is_valid_custom_element_name("x-日本語-tag"));
    assert!(is_valid_custom_element_name("math-·-dot"));
}

#[test]
fn test_is_valid_custom_element_name_invalid_cases() {
    // 1. Sem hífen
    assert!(!is_valid_custom_element_name(""));
    assert!(!is_valid_custom_element_name("div"));
    assert!(!is_valid_custom_element_name("user"));
    assert!(!is_valid_custom_element_name("custom"));
    assert!(!is_valid_custom_element_name("element"));

    // 2. Não começa com [a-z]
    assert!(!is_valid_custom_element_name("-tag"));
    assert!(!is_valid_custom_element_name(".my-tag"));
    assert!(!is_valid_custom_element_name("_my-tag"));
    assert!(!is_valid_custom_element_name("1-element"));
    assert!(!is_valid_custom_element_name("9-custom"));

    // 3. Contém maiúsculas ASCII
    assert!(!is_valid_custom_element_name("My-element"));
    assert!(!is_valid_custom_element_name("my-Element"));
    assert!(!is_valid_custom_element_name("MY-ELEMENT"));
    assert!(!is_valid_custom_element_name("user-Profile-card"));

    // 4. Contém caracteres especiais ou espaços inválidos
    assert!(!is_valid_custom_element_name("user-card#1"));
    assert!(!is_valid_custom_element_name("user-card@home"));
    assert!(!is_valid_custom_element_name("user-card$special"));
    assert!(!is_valid_custom_element_name("user-card!alert"));
    assert!(!is_valid_custom_element_name("bad-tag name"));
    assert!(!is_valid_custom_element_name(" bad-tag"));
    assert!(!is_valid_custom_element_name("bad-tag "));

    // 5. Todos os 8 nomes reservados proibidos (WHATWG §4.13.1.2)
    assert!(!is_valid_custom_element_name("annotation-xml"));
    assert!(!is_valid_custom_element_name("color-profile"));
    assert!(!is_valid_custom_element_name("font-face"));
    assert!(!is_valid_custom_element_name("font-face-src"));
    assert!(!is_valid_custom_element_name("font-face-uri"));
    assert!(!is_valid_custom_element_name("font-face-format"));
    assert!(!is_valid_custom_element_name("font-face-name"));
    assert!(!is_valid_custom_element_name("missing-glyph"));

    // Variações que NÃO são os nomes reservados exatos devem ser aceitas se válidas
    assert!(is_valid_custom_element_name("annotation-xml-extended"));
    assert!(is_valid_custom_element_name("my-font-face"));
    assert!(is_valid_custom_element_name("missing-glyph-custom"));
}

#[test]
fn test_custom_element_registry_define_and_lookup() {
    let mut registry = CustomElementRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);

    // Registro bem-sucedido
    let res = registry.define("my-card", &["theme", "elevation", "data-user"]);
    assert!(res.is_ok());
    assert!(!registry.is_empty());
    assert_eq!(registry.len(), 1);
    assert!(registry.is_defined("my-card"));
    assert!(!registry.is_defined("other-card"));

    let def = registry.get("my-card").expect("definição encontrada");
    assert_eq!(def.name.as_str(), "my-card");
    assert_eq!(def.observed_attributes.len(), 3);
    assert!(def.observes_attribute("theme"));
    assert!(def.observes_attribute("THEME")); // case-insensitive
    assert!(def.observes_attribute("elevation"));
    assert!(def.observes_attribute("data-user"));
    assert!(!def.observes_attribute("unobserved"));

    // Erro em nome inválido
    let err_invalid = registry.define("invalidName", &[]);
    assert!(err_invalid.is_err());

    // Erro em nome reservado
    let err_reserved = registry.define("font-face", &[]);
    assert!(err_reserved.is_err());

    // Erro em duplicata
    let err_dup = registry.define("my-card", &[]);
    assert!(err_dup.is_err());
}

#[test]
fn test_custom_element_registry_when_defined() {
    let mut registry = CustomElementRegistry::new();

    let called_before_1 = Arc::new(AtomicBool::new(false));
    let called_before_2 = Arc::new(AtomicBool::new(false));
    let called_after = Arc::new(AtomicBool::new(false));

    let cb1 = {
        let flag = called_before_1.clone();
        move || flag.store(true, Ordering::SeqCst)
    };
    let cb2 = {
        let flag = called_before_2.clone();
        move || flag.store(true, Ordering::SeqCst)
    };

    // Registra listeners antes de definir "x-widget"
    registry.when_defined("x-widget", cb1);
    registry.when_defined("x-widget", cb2);

    assert!(!called_before_1.load(Ordering::SeqCst));
    assert!(!called_before_2.load(Ordering::SeqCst));

    // Define "x-widget" -> deve resolver os callbacks pendentes
    registry
        .define("x-widget", &["active"])
        .expect("definição com sucesso");

    assert!(called_before_1.load(Ordering::SeqCst));
    assert!(called_before_2.load(Ordering::SeqCst));

    // when_defined chamado DEPOIS da definição deve invocar imediatamente
    let cb3 = {
        let flag = called_after.clone();
        move || flag.store(true, Ordering::SeqCst)
    };
    registry.when_defined("x-widget", cb3);
    assert!(called_after.load(Ordering::SeqCst));
}

#[test]
fn test_custom_element_state_initialization() {
    let mut doc = Document::new(None);

    // 1. Elemento padrão (HTML <div>) -> Uncustomized
    let div_id = doc.create_element("div", Namespace::Html);
    let div_node = doc.get_node(div_id).unwrap();
    let div_el = div_node.as_element().unwrap();
    assert_eq!(div_el.custom_element_state(), CustomElementState::Uncustomized);

    // 2. Elemento com nome customizado válido antes de registro -> Undefined
    let custom_id = doc.create_element("user-badge", Namespace::Html);
    let custom_node = doc.get_node(custom_id).unwrap();
    let custom_el = custom_node.as_element().unwrap();
    assert_eq!(custom_el.custom_element_state(), CustomElementState::Undefined);

    // 3. Mutação manual de estado
    if let Some(el_mut) = doc.get_node_mut(custom_id).and_then(|n| n.as_element_mut()) {
        el_mut.set_custom_element_state(CustomElementState::Custom);
        el_mut.set_custom_element_definition(Some(Atom::new("user-badge")));
        el_mut.set_is_value(Some(Atom::new("custom-button")));
    }

    let updated_el = doc.get_node(custom_id).unwrap().as_element().unwrap();
    assert_eq!(updated_el.custom_element_state(), CustomElementState::Custom);
    assert_eq!(
        updated_el.custom_element_definition(),
        Some(Atom::new("user-badge"))
    );
    assert_eq!(
        updated_el.is_value(),
        Some(Atom::new("custom-button"))
    );
}

#[test]
fn test_custom_element_upgrade_algorithm() {
    let mut doc = Document::new(None);
    let mut registry = CustomElementRegistry::new();
    let mut reactions = CustomElementReactionsStack::new();

    // Monta a estrutura da árvore com nós ainda no estado `Undefined`:
    // <div id="container">
    //   <user-item theme="dark" unobserved="foo"></user-item>
    //   <span><user-item theme="light"></user-item></span>
    //   <p>Texto</p>
    // </div>
    let container_id = doc.create_element("div", Namespace::Html);
    let user_item_1 = doc.create_element("user-item", Namespace::Html);
    let span_id = doc.create_element("span", Namespace::Html);
    let user_item_2 = doc.create_element("user-item", Namespace::Html);
    let p_id = doc.create_element("p", Namespace::Html);

    if let Some(el) = doc.get_node_mut(user_item_1).and_then(|n| n.as_element_mut()) {
        el.set_attribute("theme", "dark");
        el.set_attribute("unobserved", "foo");
    }
    if let Some(el) = doc.get_node_mut(user_item_2).and_then(|n| n.as_element_mut()) {
        el.set_attribute("theme", "light");
    }

    doc.append_child(container_id, user_item_1).unwrap();
    doc.append_child(container_id, span_id).unwrap();
    doc.append_child(span_id, user_item_2).unwrap();
    doc.append_child(container_id, p_id).unwrap();
    doc.append_child(doc.root(), container_id).unwrap();

    // Confirma que os nós ainda estão Undefined
    assert_eq!(
        doc.get_node(user_item_1).unwrap().as_element().unwrap().custom_element_state(),
        CustomElementState::Undefined
    );
    assert_eq!(
        doc.get_node(user_item_2).unwrap().as_element().unwrap().custom_element_state(),
        CustomElementState::Undefined
    );

    // Registra "user-item" com atributo observado "theme"
    registry
        .define("user-item", &["theme"])
        .expect("registro ok");

    // Executa upgrade na subárvore sob `container_id`
    registry.upgrade(doc.arena_mut(), container_id, Some(&mut reactions));

    // Verifica se os nós transicionaram para Custom
    assert_eq!(
        doc.get_node(user_item_1).unwrap().as_element().unwrap().custom_element_state(),
        CustomElementState::Custom
    );
    assert_eq!(
        doc.get_node(user_item_2).unwrap().as_element().unwrap().custom_element_state(),
        CustomElementState::Custom
    );

    // Verifica as reações enfileiradas pelo upgrade:
    // Deve haver Connected e AttributeChanged para "theme"
    let queued = reactions.drain_all();
    assert!(
        queued.contains(&CustomElementReaction::Connected(user_item_1)),
        "user_item_1 deve receber ConnectedReaction"
    );
    assert!(
        queued.contains(&CustomElementReaction::Connected(user_item_2)),
        "user_item_2 deve receber ConnectedReaction"
    );
    assert!(
        queued.contains(&CustomElementReaction::AttributeChanged {
            node: user_item_1,
            name: SmolStr::new("theme"),
            old_value: None,
            new_value: Some(SmolStr::new("dark")),
        }),
        "user_item_1 deve receber AttributeChangedReaction para 'theme'"
    );
    assert!(
        queued.contains(&CustomElementReaction::AttributeChanged {
            node: user_item_2,
            name: SmolStr::new("theme"),
            old_value: None,
            new_value: Some(SmolStr::new("light")),
        }),
        "user_item_2 deve receber AttributeChangedReaction para 'theme'"
    );
    // Atributo "unobserved" NÃO deve ter reação enfileirada
    assert!(
        !queued.iter().any(|r| matches!(r, CustomElementReaction::AttributeChanged { name, .. } if name == "unobserved")),
        "Atributo unobserved não deve gerar reação"
    );
}

#[test]
fn test_custom_element_callbacks_definition_builder() {
    let connected_count = Arc::new(AtomicUsize::new(0));
    let disconnected_count = Arc::new(AtomicUsize::new(0));
    let attr_changed_count = Arc::new(AtomicUsize::new(0));

    let c_count = connected_count.clone();
    let d_count = disconnected_count.clone();
    let a_count = attr_changed_count.clone();

    let callbacks = CustomElementCallbacks::new()
        .with_connected(move |_id| {
            c_count.fetch_add(1, Ordering::SeqCst);
        })
        .with_disconnected(move |_id| {
            d_count.fetch_add(1, Ordering::SeqCst);
        })
        .with_attribute_changed(move |_id, name, old, new| {
            assert_eq!(name, "status");
            assert_eq!(old, None);
            assert_eq!(new, Some("ready"));
            a_count.fetch_add(1, Ordering::SeqCst);
        });

    let mut registry = CustomElementRegistry::new();
    let res = registry.define_with_callbacks("lifecycle-box", &["status"], callbacks);
    assert!(res.is_ok());

    let def = registry.get("lifecycle-box").unwrap();
    assert!(def.callbacks.connected_callback.is_some());
    assert!(def.callbacks.disconnected_callback.is_some());
    assert!(def.callbacks.attribute_changed_callback.is_some());
    assert!(def.callbacks.adopted_callback.is_none());
}
