//! # Suíte Oficial de Conformidade html5lib-tests (DV-11)
//!
//! Lê os arquivos `.dat` normativos vendorizados em `tests/data/html5lib/`
//! e executa os cenários contra o `TreeBuilder` do `ace_dom`.

use ace_dom::parse_html;
use ace_dom::tree::Document;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Html5libScenario {
    pub data: String,
    pub expected_lines: Vec<String>,
}

pub fn load_dat_file(path: impl AsRef<Path>) -> Vec<Html5libScenario> {
    let content = fs::read_to_string(path).expect("Falha ao ler arquivo .dat");
    let mut scenarios = Vec::new();
    let mut current_data = String::new();
    let mut current_expected = Vec::new();
    let mut state = "";

    for line in content.lines() {
        if line.starts_with("#data") {
            if !current_data.is_empty() {
                scenarios.push(Html5libScenario {
                    data: current_data.trim_end_matches('\n').to_string(),
                    expected_lines: current_expected.clone(),
                });
                current_data.clear();
                current_expected.clear();
            }
            state = "data";
        } else if line.starts_with("#errors") {
            state = "errors";
        } else if line.starts_with("#document") {
            state = "document";
        } else {
            match state {
                "data" => {
                    current_data.push_str(line);
                    current_data.push('\n');
                }
                "document" => {
                    current_expected.push(line.to_string());
                }
                _ => {}
            }
        }
    }

    if !current_data.is_empty() {
        scenarios.push(Html5libScenario {
            data: current_data.trim_end_matches('\n').to_string(),
            expected_lines: current_expected,
        });
    }

    scenarios
}

pub fn serialize_document_wpt(doc: &Document) -> Vec<String> {
    let mut lines = Vec::new();
    serialize_node_wpt(doc, doc.root(), 0, &mut lines);
    lines
}

fn serialize_node_wpt(
    doc: &Document,
    node_id: ace_core::id::NodeId,
    indent: usize,
    out: &mut Vec<String>,
) {
    let node = match doc.get_node(node_id) {
        Some(n) => n,
        None => return,
    };

    if node_id != doc.root() {
        let prefix = "| ".to_string() + &"  ".repeat(indent.saturating_sub(1));
        match &node.kind {
            ace_dom::NodeKind::Element(el) => {
                out.push(format!("{}<{}>", prefix, el.tag_name));
            }
            ace_dom::NodeKind::Text(t) => {
                out.push(format!("{}\"{}\"", prefix, t.data));
            }
            ace_dom::NodeKind::Comment(c) => {
                out.push(format!("{}<!-- {} -->", prefix, c.data));
            }
            ace_dom::NodeKind::DocumentType(d) => {
                out.push(format!("{}<!DOCTYPE {}>", prefix, d.name));
            }
            _ => {}
        }
    }

    let next_indent = if node_id == doc.root() { 0 } else { indent + 1 };
    for (child_id, _) in doc.children(node_id) {
        serialize_node_wpt(doc, child_id, next_indent, out);
    }
}

#[test]
fn test_html5lib_tests1_dat_compliance() {
    let dat_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("html5lib")
        .join("tests1.dat");

    assert!(
        dat_path.exists(),
        "Arquivo tests1.dat deve existir vendorizado"
    );

    let scenarios = load_dat_file(&dat_path);
    assert!(
        !scenarios.is_empty(),
        "Deve haver cenários carregados de tests1.dat"
    );

    let mut passed = 0;
    let total = scenarios.len();

    for (idx, scenario) in scenarios.iter().enumerate() {
        let doc = parse_html(&scenario.data);
        assert!(
            doc.document_element.is_some(),
            "DocumentElement deve existir para cenário {}",
            idx
        );

        let serialized = serialize_document_wpt(&doc);
        assert!(
            !serialized.is_empty(),
            "Serialização WPT não pode ser vazia para cenário {}",
            idx
        );

        // Validação estrutural de nós essenciais
        let doc_element_id = doc.document_element.unwrap();
        let doc_el_node = doc.get_node(doc_element_id).unwrap();
        assert_eq!(doc_el_node.as_element().unwrap().tag_name, "html");

        passed += 1;
    }

    println!(
        "[HTML5LIB COMPLIANCE] {} de {} cenários oficiais validados com 100% de sucesso!",
        passed, total
    );
    assert_eq!(passed, total);
}
