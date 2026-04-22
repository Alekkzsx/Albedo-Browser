use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::time::Instant;

use albedo::ace::html::{
    parse_document, parse_document_with_options, parse_fragment_with_context, FragmentContext,
    HtmlNode, Namespace, ParserOptions,
};
use serde_json::Value;

#[derive(Clone, Debug)]
struct Case {
    id: String,
    mode: String,
    context: Option<String>,
    input: String,
    expected_tree: String,
}

fn main() {
    let required_cases = load_required_tree_cases();
    let sample_cases = load_html5lib_sample(100);
    let perf_html = build_large_html(1_200_000);

    let required = run_cases(&required_cases);
    let sample = run_cases(&sample_cases);
    let perf = benchmark_perf(&perf_html, 20);

    let out = serde_json::json!({
        "datasets": {
            "requiredTreeCases": required_cases.len(),
            "html5libTreeSampleCases": sample_cases.len(),
            "perfHtmlBytes": perf_html.len(),
        },
        "ace_html": {
            "requiredTree": required,
            "html5libTreeSample": sample,
            "performance": perf,
        }
    });

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}

fn build_large_html(target_bytes: usize) -> String {
    let mut html = String::from("<!DOCTYPE html><html><head><title>v</title></head><body>");
    let mut i = 0usize;
    while html.len() < target_bytes.saturating_sub(32) {
        html.push_str("<div class=\"item\"><span>content ");
        html.push_str(&i.to_string());
        html.push_str("</span></div>");
        i += 1;
    }
    html.push_str("</body></html>");
    html
}

fn benchmark_perf(html: &str, iterations: usize) -> serde_json::Value {
    for _ in 0..3 {
        let _ = parse_document(html);
    }
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = parse_document(html);
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_secs_f64() * 1000.0 / iterations as f64;
    let throughput_mbps = (html.len() as f64 / 1_000_000.0) / (avg_ms / 1000.0);
    serde_json::json!({
        "avgMs": avg_ms,
        "throughputMbps": throughput_mbps
    })
}

fn run_cases(cases: &[Case]) -> serde_json::Value {
    let mut passed = 0usize;
    let mut failures = Vec::new();
    let mut total_case_ms = 0.0f64;

    for case in cases {
        let start = Instant::now();
        let actual = if case.mode == "document" {
            let doc = parse_document_with_options(&case.input, &ParserOptions::default());
            serialize_document(&doc)
        } else {
            let context = case.context.as_ref().map(|ctx| FragmentContext::new(ctx));
            let nodes = parse_fragment_with_context(&case.input, context.as_ref(), &ParserOptions::default());
            serialize_nodes(&nodes)
        };
        total_case_ms += start.elapsed().as_secs_f64() * 1000.0;

        if actual.trim_end() == case.expected_tree.trim_end() {
            passed += 1;
        } else if failures.len() < 5 {
            failures.push(serde_json::json!({
                "id": case.id,
                "expected": case.expected_tree,
                "actual": actual.trim_end(),
            }));
        }
    }

    serde_json::json!({
        "total": cases.len(),
        "passed": passed,
        "passRate": if cases.is_empty() { 0.0 } else { passed as f64 * 100.0 / cases.len() as f64 },
        "avgCaseMs": if cases.is_empty() { 0.0 } else { total_case_ms / cases.len() as f64 },
        "failures": failures,
    })
}

fn load_required_tree_cases() -> Vec<Case> {
    let raw = fs::read_to_string("tests/ace_html_conformance/required.json").unwrap();
    let json: Value = serde_json::from_str(&raw).unwrap();
    let mut out = Vec::new();
    for case in json["cases"].as_array().unwrap() {
        let mode = case["mode"].as_str().unwrap();
        if mode == "document" || mode == "fragment" {
            out.push(Case {
                id: case["case_id"].as_str().unwrap().to_string(),
                mode: mode.to_string(),
                context: case.get("context").and_then(|v| v.as_str()).map(str::to_string),
                input: case["input"].as_str().unwrap().to_string(),
                expected_tree: case["expected_tree"].as_str().unwrap().trim_end().to_string(),
            });
        }
    }
    out
}

fn load_html5lib_sample(limit: usize) -> Vec<Case> {
    let dat_dir = Path::new("tests/html5lib/tree-construction");
    let mut entries: Vec<_> = fs::read_dir(dat_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "dat"))
        .collect();
    entries.sort_by_key(|entry| entry.file_name());

    let mut out = Vec::new();
    for entry in entries {
        let file_name = entry.file_name().to_string_lossy().to_string();
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        let mut data = String::new();
        let mut fragment_ctx: Option<String> = None;
        let mut expected_tree = String::new();
        let mut section = "";
        let mut index = 0usize;

        for line in content.lines() {
            match line {
                "#data" => {
                    if !data.is_empty() || fragment_ctx.is_some() || !expected_tree.is_empty() {
                        out.push(Case {
                            id: format!("{file_name}#{index}"),
                            mode: if fragment_ctx.is_some() { "fragment".into() } else { "document".into() },
                            context: fragment_ctx.take(),
                            input: data.trim_end_matches('\n').to_string(),
                            expected_tree: expected_tree.trim_end().to_string(),
                        });
                        if out.len() >= limit {
                            return out;
                        }
                        data.clear();
                        expected_tree.clear();
                        index += 1;
                    }
                    section = "data";
                }
                "#document" => section = "document",
                "#document-fragment" => section = "fragment",
                "#errors" | "#new-errors" | "#script-on" | "#script-off" => section = "skip",
                _ if line.starts_with('#') => section = "skip",
                _ => match section {
                    "data" => {
                        data.push_str(line);
                        data.push('\n');
                    }
                    "fragment" => fragment_ctx = Some(line.trim().to_string()),
                    "document" => {
                        expected_tree.push_str(line);
                        expected_tree.push('\n');
                    }
                    _ => {}
                },
            }
        }

        if !data.is_empty() || fragment_ctx.is_some() || !expected_tree.is_empty() {
            out.push(Case {
                id: format!("{file_name}#{index}"),
                mode: if fragment_ctx.is_some() { "fragment".into() } else { "document".into() },
                context: fragment_ctx,
                input: data.trim_end_matches('\n').to_string(),
                expected_tree: expected_tree.trim_end().to_string(),
            });
            if out.len() >= limit {
                return out;
            }
        }
    }
    out
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
        out.push('\n');
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
