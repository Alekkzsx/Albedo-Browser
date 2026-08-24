//! # WPT & html5lib-tests Integration Harness (ace_dom)
//!
//! Executor e validador nativo para arquivos de teste no formato canônico `.dat`
//! do consórcio W3C Web Platform Tests (WPT).

use ace_dom::parse_html;
use ace_dom::tree::Document;

/// Cenário de teste individual extraído de arquivo .dat do html5lib
#[derive(Debug, Clone)]
pub struct WptTestCase {
    pub data: String,
    pub expected_lines: Vec<String>,
}

/// Carregador e executor de arquivos .dat do html5lib
pub struct WptHarness;

impl WptHarness {
    /// Faz o parsing de um conteúdo .dat com múltiplas seções #data e #document
    pub fn parse_dat_scenarios(content: &str) -> Vec<WptTestCase> {
        let mut scenarios = Vec::new();
        let mut current_data = String::new();
        let mut current_expected = Vec::new();
        let mut state = "";

        for line in content.lines() {
            if line.starts_with("#data") {
                if !current_data.is_empty() {
                    scenarios.push(WptTestCase {
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
            scenarios.push(WptTestCase {
                data: current_data.trim_end_matches('\n').to_string(),
                expected_lines: current_expected,
            });
        }

        scenarios
    }

    /// Serializa uma árvore Document no formato de comparação canônica do html5lib (| <html> ...)
    pub fn serialize_to_wpt_format(doc: &Document) -> Vec<String> {
        let mut lines = Vec::new();
        Self::serialize_node_wpt(doc, doc.root(), 0, &mut lines);
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
            Self::serialize_node_wpt(doc, child_id, next_indent, out);
        }
    }
}

#[test]
fn test_wpt_dat_scenarios_execution() {
    let dat_sample = r#"
#data
<p>Hello <b>World</b>
#errors
#document
| <html>
|   <head>
|   <body>
|     <p>
|       "Hello "
|       <b>
|         "World"

#data
<div><span>Nested</span></div>
#errors
#document
| <html>
|   <head>
|   <body>
|     <div>
|       <span>
|         "Nested"
"#;

    let scenarios = WptHarness::parse_dat_scenarios(dat_sample);
    assert_eq!(scenarios.len(), 2);

    for scenario in scenarios {
        let doc = parse_html(&scenario.data);
        assert!(doc.document_element.is_some());
        assert!(doc.body.is_some());

        let serialized = WptHarness::serialize_to_wpt_format(&doc);
        assert!(!serialized.is_empty());
        // Verifica se a estrutura básica coincide
        assert_eq!(serialized[0], "| <html>");
    }
}
