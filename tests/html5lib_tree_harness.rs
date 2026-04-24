/// html5lib tree-construction conformance harness (Gate 2).
///
/// Reads all *.dat files from tests/html5lib/tree-construction/, parses each
/// test case, runs it through the ACE-html parser, and computes a pass rate.
///
/// Threshold: the `ace_html_tree_construction_smoke` test enforces ≥ 75% passing;
/// the secondary `ace_html_tree_construction_full_report` test prints all
/// failures without enforcing a threshold, so you can see exactly what's failing.
use std::fmt::Write as _;
use std::path::Path;

use albedo::ace::html::{
    parse_document_with_options, parse_fragment_with_context, FragmentContext, HtmlNode, Namespace,
    ParserOptions,
};

// ─── .dat parser ──────────────────────────────────────────────────────────────

struct TestCase {
    file: String,
    index: usize,
    data: String,
    fragment_ctx: Option<String>,
    scripting_enabled: bool,
    expected_tree: String,
}

fn parse_dat(path: &Path) -> Vec<TestCase> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mut cases = Vec::new();
    let mut data = String::new();
    let mut fragment_ctx: Option<String> = None;
    let mut scripting_enabled = true;
    let mut expected_tree = String::new();
    let mut section = "";
    let mut index = 0usize;

    for line in content.lines() {
        match line {
            "#data" => {
                if !data.is_empty() || fragment_ctx.is_some() || !expected_tree.is_empty() {
                    cases.push(TestCase {
                        file: file_name.clone(),
                        index,
                        data: data.trim_end_matches('\n').to_string(),
                        fragment_ctx: fragment_ctx.take(),
                        scripting_enabled,
                        expected_tree: expected_tree.trim_end_matches('\n').to_string(),
                    });
                    data.clear();
                    expected_tree.clear();
                    scripting_enabled = true;
                    index += 1;
                }
                section = "data";
            }
            "#errors" => section = "errors",
            "#new-errors" => section = "errors",
            "#document" => section = "document",
            "#document-fragment" => section = "fragment",
            "#script-off" => {
                scripting_enabled = false;
                section = "skip";
            }
            "#script-on" => {
                scripting_enabled = true;
                section = "skip";
            }
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
    // Push the last test case.
    if !data.is_empty() || fragment_ctx.is_some() {
        cases.push(TestCase {
            file: file_name,
            index,
            data: data.trim_end_matches('\n').to_string(),
            fragment_ctx,
            scripting_enabled,
            expected_tree: expected_tree.trim_end_matches('\n').to_string(),
        });
    }
    cases
}

fn fragment_context_from_tag(tag_name: &str, scripting_enabled: bool) -> FragmentContext {
    let mut context = FragmentContext::new(tag_name).with_scripting(scripting_enabled);
    context.namespace = match tag_name {
        "svg" => Namespace::Svg,
        "math" | "mi" | "mo" | "mn" | "ms" | "mtext" | "annotation-xml" => Namespace::MathMl,
        _ => Namespace::Html,
    };
    context
}

// ─── Serializer ───────────────────────────────────────────────────────────────

fn serialize_tree(nodes: &[HtmlNode], is_fragment: bool) -> String {
    let mut out = String::new();
    // For fragments, start from depth=0 directly.
    // For documents, we also start from depth=0 but don't add a wrapper.
    for node in nodes {
        serialize_node(node, 0, &mut out);
    }
    // Trim trailing blank line if present.
    let _ = is_fragment;
    out
}

fn serialize_node(node: &HtmlNode, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    match node {
        HtmlNode::Element(el) => {
            if el.tag == "template-content" {
                writeln!(out, "| {}content", indent).unwrap();
                for child in &el.children {
                    serialize_node(child, depth + 1, out);
                }
                return;
            }

            let ns = match el.namespace {
                Namespace::Html => String::new(),
                Namespace::Svg => "svg ".to_string(),
                Namespace::MathMl => "math ".to_string(),
            };
            writeln!(out, "| {}<{}{}>", indent, ns, el.tag).unwrap();
            let mut attrs: Vec<_> = el.attributes.iter().collect();
            attrs.sort_by(|(left, _), (right, _)| left.cmp(right));
            for (name, value) in attrs {
                writeln!(out, "| {}  {}=\"{}\"", indent, name, value).unwrap();
            }
            for child in &el.children {
                serialize_node(child, depth + 1, out);
            }
        }
        HtmlNode::Text(text) => writeln!(out, "| {}\"{}\"", indent, text).unwrap(),
        HtmlNode::Comment(text) => writeln!(out, "| {}<!-- {} -->", indent, text).unwrap(),
    }
}

// ─── Runner ───────────────────────────────────────────────────────────────────

struct RunResult {
    total: usize,
    passed: usize,
    failures: Vec<String>,
}

fn file_filter() -> Option<String> {
    std::env::var("ACE_HTML_TREE_FILE_FILTER")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn case_limit() -> Option<usize> {
    std::env::var("ACE_HTML_TREE_CASE_LIMIT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
}

fn run_all_cases() -> RunResult {
    let dat_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/html5lib/tree-construction");

    let filter = file_filter();
    let limit = case_limit();
    let mut entries: Vec<_> = std::fs::read_dir(&dat_dir)
        .expect("tree-construction dir missing; run tests with corpus vendorized")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "dat"))
        .filter(|entry| {
            filter
                .as_ref()
                .is_none_or(|needle| entry.file_name().to_string_lossy().contains(needle))
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut total = 0usize;
    let mut passed = 0usize;
    let mut failures = Vec::new();

    for entry in entries {
        let path = entry.path();
        let cases = parse_dat(&path);
        for case in cases {
            if limit.is_some_and(|max_cases| total >= max_cases) {
                return RunResult {
                    total,
                    passed,
                    failures,
                };
            }
            total += 1;
            let options = ParserOptions {
                scripting_enabled: case.scripting_enabled,
                ..ParserOptions::default()
            };

            let actual = if let Some(ref ctx) = case.fragment_ctx {
                let context = fragment_context_from_tag(ctx, case.scripting_enabled);
                let nodes = parse_fragment_with_context(&case.data, Some(&context), &options);
                serialize_tree(&nodes, true)
            } else {
                let doc = parse_document_with_options(&case.data, &options);
                let mut out = String::new();
                if let Some(ref dt) = doc.doctype {
                    let name = dt.name.as_deref().unwrap_or("");
                    let public = dt.public_id.as_deref().unwrap_or("");
                    let system = dt.system_id.as_deref().unwrap_or("");
                    write!(out, "| <!DOCTYPE {}", name).unwrap();
                    if !public.is_empty() || !system.is_empty() {
                        write!(out, " \"{}\" \"{}\"", public, system).unwrap();
                    }
                    out.push_str(">\n");
                }
                out.push_str(&serialize_tree(&doc.children, false));
                out
            };

            let actual_trimmed = actual.trim_end();
            let expected_trimmed = case.expected_tree.trim_end();

            if actual_trimmed == expected_trimmed {
                passed += 1;
            } else {
                failures.push(format!(
                    "[{}#{}] input={:?} ctx={:?}\nEXPECTED:\n{}\nACTUAL:\n{}\n",
                    case.file,
                    case.index,
                    case.data,
                    case.fragment_ctx,
                    expected_trimmed,
                    actual_trimmed,
                ));
            }
        }
    }

    RunResult {
        total,
        passed,
        failures,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn tree_construction_manifest_exists() {
    let dat_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/html5lib/tree-construction");
    assert!(
        dat_dir.exists(),
        "html5lib/tree-construction directory not found"
    );
    let count = std::fs::read_dir(&dat_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "dat"))
        .count();
    assert!(count >= 10, "expected ≥ 10 .dat files, got {}", count);
}

/// Smoke test: enforces a minimum pass rate so obvious regressions are caught immediately.
/// The threshold starts at 75% and will be raised as compliance improves.
#[test]
fn ace_html_tree_construction_smoke() {
    let result = run_all_cases();
    let pct = if result.total == 0 {
        100.0f64
    } else {
        result.passed as f64 / result.total as f64 * 100.0
    };

    // Print up to 10 failures for quick diagnosis.
    for failure in result.failures.iter().take(10) {
        eprintln!("{}", failure);
    }
    if result.failures.len() > 10 {
        eprintln!("... and {} more failures", result.failures.len() - 10);
    }
    eprintln!(
        "\ntree-construction: {}/{} passed ({:.1}%)",
        result.passed, result.total, pct
    );

    // Minimum threshold — raised incrementally as bugs are fixed.
    const MIN_PCT: f64 = 75.0;
    assert!(
        pct >= MIN_PCT,
        "tree-construction pass rate {:.1}% is below minimum {:.1}% ({}/{} passed). first failure:\n{}",
        pct,
        MIN_PCT,
        result.passed,
        result.total,
        result.failures.first().map_or("(none)", String::as_str),
    );
}

/// Diagnostic test: always passes but prints the full failure list.
/// Run with: cargo test --test html5lib_tree_harness ace_html_tree_construction_full_report -- --nocapture
#[test]
#[ignore = "diagnostic-only; runs the full corpus and prints every failure"]
fn ace_html_tree_construction_full_report() {
    let result = run_all_cases();
    let pct = if result.total == 0 {
        100.0f64
    } else {
        result.passed as f64 / result.total as f64 * 100.0
    };

    eprintln!(
        "\n=== tree-construction: {}/{} passed ({:.1}%) ===",
        result.passed, result.total, pct
    );
    for (i, failure) in result.failures.iter().enumerate() {
        eprintln!("── Failure {} ──\n{}", i + 1, failure);
    }
}
