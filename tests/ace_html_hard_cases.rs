use std::fmt::Write as _;
use std::fs;

use albedo::ace::html::{
    parse_document_with_options, parse_fragment_with_context, FragmentContext, HtmlNode, Namespace,
    ParserOptions,
};
use serde_json::Value;

#[derive(Clone, Debug)]
struct HardCase {
    id: String,
    mode: String,
    context: Option<String>,
    input: String,
    expected_tree: String,
}

#[test]
fn ace_html_hard_cases_guard() {
    let cases = load_cases();
    assert!(
        !cases.is_empty(),
        "hard-cases dataset must not be empty: tests/ace_html_hard_cases.json"
    );

    let mut failures = Vec::new();
    for case in &cases {
        let options = ParserOptions::default();
        let actual = if case.mode == "document" {
            let doc = parse_document_with_options(&case.input, &options);
            serialize_document(&doc)
        } else {
            let context_name = case.context.as_deref().unwrap_or("div");
            let context = fragment_context_from_tag(context_name);
            let nodes = parse_fragment_with_context(&case.input, Some(&context), &options);
            serialize_nodes(&nodes)
        };

        if actual.trim_end() != case.expected_tree.trim_end() {
            failures.push(format!(
                "case={} mode={} ctx={:?}\nEXPECTED:\n{}\nACTUAL:\n{}\n",
                case.id,
                case.mode,
                case.context,
                case.expected_tree,
                actual.trim_end(),
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "hard-cases guard failed ({} cases). first failures:\n{}",
        failures.len(),
        failures.into_iter().take(5).collect::<Vec<_>>().join("\n")
    );
}

fn load_cases() -> Vec<HardCase> {
    let raw = fs::read_to_string("tests/ace_html_hard_cases.json")
        .expect("missing tests/ace_html_hard_cases.json");
    let json: Value = serde_json::from_str(&raw).expect("invalid hard-cases json");
    json["cases"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|entry| HardCase {
            id: entry["id"].as_str().unwrap_or("").to_string(),
            mode: entry["mode"].as_str().unwrap_or("document").to_string(),
            context: entry
                .get("context")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            input: entry["input"].as_str().unwrap_or("").to_string(),
            expected_tree: entry["expected_tree"].as_str().unwrap_or("").to_string(),
        })
        .collect()
}

fn fragment_context_from_tag(tag_name: &str) -> FragmentContext {
    let mut context = FragmentContext::new(tag_name);
    context.namespace = match tag_name {
        "svg" => Namespace::Svg,
        "math" | "mi" | "mo" | "mn" | "ms" | "mtext" | "annotation-xml" => Namespace::MathMl,
        _ => Namespace::Html,
    };
    context
}

fn serialize_document(document: &albedo::ace::html::HtmlDocument) -> String {
    let mut out = String::new();
    if let Some(dt) = &document.doctype {
        let name = dt.name.as_deref().unwrap_or("");
        out.push_str("| <!DOCTYPE ");
        out.push_str(name);
        if let Some(public_id) = &dt.public_id {
            let system_id = dt.system_id.as_deref().unwrap_or("");
            let _ = write!(out, " \"{}\" \"{}\"", public_id, system_id);
        } else if let Some(system_id) = &dt.system_id {
            let _ = write!(out, " \"\" \"{}\"", system_id);
        }
        out.push_str(">\n");
    }
    out.push_str(&serialize_nodes(&document.children));
    out
}

fn serialize_nodes(nodes: &[HtmlNode]) -> String {
    let mut out = String::new();
    for node in nodes {
        serialize_node(node, 0, &mut out);
    }
    out
}

fn serialize_node(node: &HtmlNode, depth: usize, out: &mut String) {
    let prefix = format!("| {}", "  ".repeat(depth));
    match node {
        HtmlNode::Element(el) => {
            let ns_prefix = match el.namespace {
                Namespace::Html => "",
                Namespace::Svg => "svg ",
                Namespace::MathMl => "math ",
            };
            let _ = writeln!(out, "{}<{}{}>", prefix, ns_prefix, el.tag);
            let mut attrs: Vec<_> = el.attributes.iter().collect();
            attrs.sort_by(|a, b| a.0.cmp(b.0));
            for (name, value) in attrs {
                let _ = writeln!(out, "{}  {}=\"{}\"", prefix, name, value);
            }
            for child in &el.children {
                serialize_node(child, depth + 1, out);
            }
        }
        HtmlNode::Text(text) => {
            let _ = writeln!(out, "{}\"{}\"", prefix, text);
        }
        HtmlNode::Comment(text) => {
            let _ = writeln!(out, "{}<!-- {} -->", prefix, text);
        }
    }
}
