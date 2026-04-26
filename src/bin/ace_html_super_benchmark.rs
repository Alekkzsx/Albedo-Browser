use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::time::Instant;

use albedo::ace::html::{
    parse_document, parse_document_with_options, parse_fragment_with_context, FragmentContext,
    HtmlNode, HtmlToken, HtmlTokenKind, HtmlTokenizer, Namespace, ParserOptions,
};
use serde_json::{json, Value};

#[derive(Clone, Debug)]
struct TreeCase {
    id: String,
    mode: String,
    context: Option<String>,
    input: String,
    expected_tree: String,
    scripting_enabled: bool,
}

#[derive(Clone, Debug)]
struct TokenizerCase {
    id: String,
    category: String,
    input: String,
    expected: Vec<Value>,
}

fn main() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .try_init();

    let tree_limit = std::env::var("ACE_HTML_TREE_LIMIT")
        .ok()
        .and_then(|v| v.parse::<usize>().ok());
    let perf_bytes = std::env::var("ACE_HTML_PERF_HTML_BYTES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1_200_000);
    let perf_warmup = std::env::var("ACE_HTML_PERF_WARMUP")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(5);
    let perf_iters = std::env::var("ACE_HTML_PERF_ITER")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(20);
    let perf_samples = std::env::var("ACE_HTML_PERF_SAMPLES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(15);

    let required_cases = load_required_tree_cases();
    let html5lib_cases = load_html5lib_cases(tree_limit);
    let tokenizer_cases = load_tokenizer_subset();
    let perf_html = build_large_html(perf_bytes);

    let required_tree = run_tree_cases(&required_cases);
    let html5lib_tree = run_tree_cases(&html5lib_cases);
    let tokenizer = run_tokenizer_cases(&tokenizer_cases);
    let performance = benchmark_perf_distribution(&perf_html, perf_warmup, perf_iters, perf_samples);

    let out = json!({
        "generatedAt": format!("{:?}", std::time::SystemTime::now()),
        "datasets": {
            "requiredTreeCases": required_cases.len(),
            "html5libTreeCases": html5lib_cases.len(),
            "tokenizerSubsetCases": tokenizer_cases.len(),
            "perfHtmlBytes": perf_html.len(),
        },
        "ace_html": {
            "requiredTree": required_tree,
            "html5libTreeFull": html5lib_tree,
            "tokenizerSubset": tokenizer,
            "performance": performance,
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

fn benchmark_perf_distribution(
    html: &str,
    warmup_iterations: usize,
    iterations_per_sample: usize,
    samples: usize,
) -> Value {
    for _ in 0..warmup_iterations {
        let _ = parse_document(html);
    }

    let mut sample_avg_ms = Vec::with_capacity(samples);
    let mut sample_throughput = Vec::with_capacity(samples);

    for _ in 0..samples {
        let started = Instant::now();
        for _ in 0..iterations_per_sample {
            let _ = parse_document(html);
        }
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        let avg_ms = elapsed_ms / iterations_per_sample as f64;
        let throughput_mbps = (html.len() as f64 / 1_000_000.0) / (avg_ms / 1000.0);

        sample_avg_ms.push(avg_ms);
        sample_throughput.push(throughput_mbps);
    }

    let avg_stats = summarize(&sample_avg_ms);
    let throughput_stats = summarize(&sample_throughput);

    json!({
        "warmupIterations": warmup_iterations,
        "iterationsPerSample": iterations_per_sample,
        "samples": samples,
        "avgMs": avg_stats.mean,
        "throughputMbps": throughput_stats.mean,
        "stddevMs": avg_stats.stddev,
        "p50Ms": avg_stats.p50,
        "p95Ms": avg_stats.p95,
        "p99Ms": avg_stats.p99,
        "minMs": avg_stats.min,
        "maxMs": avg_stats.max,
        "throughputP50Mbps": throughput_stats.p50,
        "throughputP95Mbps": throughput_stats.p95,
        "throughputP99Mbps": throughput_stats.p99,
        "throughputMinMbps": throughput_stats.min,
        "throughputMaxMbps": throughput_stats.max,
        "sampleAvgMs": sample_avg_ms,
        "sampleThroughputMbps": sample_throughput,
    })
}

#[derive(Clone, Debug)]
struct Summary {
    mean: f64,
    stddev: f64,
    p50: f64,
    p95: f64,
    p99: f64,
    min: f64,
    max: f64,
}

fn summarize(values: &[f64]) -> Summary {
    if values.is_empty() {
        return Summary {
            mean: 0.0,
            stddev: 0.0,
            p50: 0.0,
            p95: 0.0,
            p99: 0.0,
            min: 0.0,
            max: 0.0,
        };
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| {
            let d = value - mean;
            d * d
        })
        .sum::<f64>()
        / values.len() as f64;
    let stddev = variance.sqrt();
    let min = values
        .iter()
        .fold(f64::INFINITY, |acc, value| if *value < acc { *value } else { acc });
    let max = values.iter().fold(f64::NEG_INFINITY, |acc, value| {
        if *value > acc { *value } else { acc }
    });

    Summary {
        mean,
        stddev,
        p50: percentile(values, 0.50),
        p95: percentile(values, 0.95),
        p99: percentile(values, 0.99),
        min,
        max,
    }
}

fn percentile(values: &[f64], quantile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let q = quantile.clamp(0.0, 1.0);
    let idx = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[idx]
}

fn run_tree_cases(cases: &[TreeCase]) -> Value {
    let mut passed = 0usize;
    let mut failures = Vec::new();
    let mut elapsed_per_case_ms = Vec::with_capacity(cases.len());

    for case in cases {
        let started = Instant::now();
        let options = ParserOptions {
            scripting_enabled: case.scripting_enabled,
            ..ParserOptions::default()
        };
        let actual = if case.mode == "document" {
            let doc = parse_document_with_options(&case.input, &options);
            serialize_document(&doc)
        } else {
            let context = case.context.as_ref().map(|ctx| {
                let mut context = FragmentContext::new(ctx).with_scripting(case.scripting_enabled);
                context.namespace = match ctx.as_str() {
                    "svg" => Namespace::Svg,
                    "math" | "mi" | "mo" | "mn" | "ms" | "mtext" | "annotation-xml" => {
                        Namespace::MathMl
                    }
                    _ => Namespace::Html,
                };
                context
            });
            let nodes = parse_fragment_with_context(&case.input, context.as_ref(), &options);
            serialize_nodes(&nodes)
        };
        elapsed_per_case_ms.push(started.elapsed().as_secs_f64() * 1000.0);

        let actual_trimmed = actual.trim_end();
        let expected_trimmed = case.expected_tree.trim_end();
        let actual_normalized = normalize_tree_template_markers(actual_trimmed);
        let expected_normalized = normalize_tree_template_markers(expected_trimmed);

        if actual_normalized == expected_normalized {
            passed += 1;
        } else if failures.len() < 20 {
            failures.push(json!({
                "id": case.id,
                "mode": case.mode,
                "context": case.context,
                "expected": expected_trimmed,
                "actual": actual_trimmed,
                "expectedNormalized": expected_normalized,
                "actualNormalized": actual_normalized,
            }));
        }
    }

    let timings = summarize(&elapsed_per_case_ms);
    json!({
        "total": cases.len(),
        "passed": passed,
        "passRate": if cases.is_empty() { 0.0 } else { passed as f64 * 100.0 / cases.len() as f64 },
        "avgCaseMs": timings.mean,
        "p50CaseMs": timings.p50,
        "p95CaseMs": timings.p95,
        "p99CaseMs": timings.p99,
        "failures": failures,
    })
}

fn run_tokenizer_cases(cases: &[TokenizerCase]) -> Value {
    let mut passed = 0usize;
    let mut failures = Vec::new();
    let mut elapsed_per_case_ms = Vec::with_capacity(cases.len());

    for case in cases {
        let started = Instant::now();
        let actual = tokenize_as_json(&case.input);
        elapsed_per_case_ms.push(started.elapsed().as_secs_f64() * 1000.0);

        if actual == case.expected {
            passed += 1;
        } else if failures.len() < 20 {
            failures.push(json!({
                "id": case.id,
                "category": case.category,
                "expected": case.expected,
                "actual": actual,
            }));
        }
    }

    let timings = summarize(&elapsed_per_case_ms);
    json!({
        "total": cases.len(),
        "passed": passed,
        "passRate": if cases.is_empty() { 0.0 } else { passed as f64 * 100.0 / cases.len() as f64 },
        "avgCaseMs": timings.mean,
        "p50CaseMs": timings.p50,
        "p95CaseMs": timings.p95,
        "p99CaseMs": timings.p99,
        "failures": failures,
    })
}

fn tokenize_as_json(input: &str) -> Vec<Value> {
    let mut tokenizer = HtmlTokenizer::new(input);
    let mut out: Vec<Value> = Vec::new();

    while let Some(token) = tokenizer.next_token() {
        if let Some(mapped) = map_token_to_json(token) {
            if let Some(Value::Object(previous)) = out.last_mut() {
                if previous.get("type") == Some(&Value::String("Character".to_string()))
                    && mapped.get("type") == Some(&Value::String("Character".to_string()))
                {
                    let merged = format!(
                        "{}{}",
                        previous
                            .get("data")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                        mapped.get("data").and_then(Value::as_str).unwrap_or_default()
                    );
                    previous.insert("data".to_string(), Value::String(merged));
                    continue;
                }
            }
            out.push(mapped);
        }
    }

    out
}

fn map_token_to_json(token: HtmlToken) -> Option<Value> {
    match token.kind {
        HtmlTokenKind::StartTag(tag) => Some(json!({
            "type": "StartTag",
            "name": tag.name,
            "attrs": tag.attributes,
            "self_closing": tag.self_closing,
        })),
        HtmlTokenKind::EndTag(tag) => Some(json!({
            "type": "EndTag",
            "name": tag.name,
        })),
        HtmlTokenKind::Character(token) => Some(json!({
            "type": "Character",
            "data": token.data,
        })),
        HtmlTokenKind::Comment(token) => Some(json!({
            "type": "Comment",
            "data": token.data,
        })),
        HtmlTokenKind::Doctype(token) => Some(json!({
            "type": "Doctype",
            "name": token.name,
            "public_id": token.public_id,
            "system_id": token.system_id,
            "force_quirks": token.force_quirks,
        })),
        HtmlTokenKind::Eof => None,
    }
}

fn load_required_tree_cases() -> Vec<TreeCase> {
    let raw = fs::read_to_string("tests/ace_html_conformance/required.json").unwrap();
    let json: Value = serde_json::from_str(&raw).unwrap();
    let mut out = Vec::new();
    for case in json["cases"].as_array().unwrap() {
        let mode = case["mode"].as_str().unwrap();
        if mode == "document" || mode == "fragment" {
            out.push(TreeCase {
                id: case["case_id"].as_str().unwrap().to_string(),
                mode: mode.to_string(),
                context: case
                    .get("context")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                input: case["input"].as_str().unwrap().to_string(),
                expected_tree: case["expected_tree"]
                    .as_str()
                    .unwrap()
                    .trim_end()
                    .to_string(),
                scripting_enabled: true,
            });
        }
    }
    out
}

fn load_html5lib_cases(limit: Option<usize>) -> Vec<TreeCase> {
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
        let mut scripting_enabled = true;
        let mut section = "";
        let mut index = 0usize;

        for line in content.lines() {
            match line {
                "#data" => {
                    if !data.is_empty() || fragment_ctx.is_some() || !expected_tree.is_empty() {
                        out.push(TreeCase {
                            id: format!("{file_name}#{index}"),
                            mode: if fragment_ctx.is_some() {
                                "fragment".into()
                            } else {
                                "document".into()
                            },
                            context: fragment_ctx.take(),
                            input: data.trim_end_matches('\n').to_string(),
                            expected_tree: expected_tree.trim_end().to_string(),
                            scripting_enabled,
                        });
                        if limit.is_some_and(|max| out.len() >= max) {
                            return out;
                        }
                        data.clear();
                        expected_tree.clear();
                        scripting_enabled = true;
                        index += 1;
                    }
                    section = "data";
                }
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
                "#errors" | "#new-errors" => section = "skip",
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
            out.push(TreeCase {
                id: format!("{file_name}#{index}"),
                mode: if fragment_ctx.is_some() {
                    "fragment".into()
                } else {
                    "document".into()
                },
                context: fragment_ctx,
                input: data.trim_end_matches('\n').to_string(),
                expected_tree: expected_tree.trim_end().to_string(),
                scripting_enabled,
            });
            if limit.is_some_and(|max| out.len() >= max) {
                return out;
            }
        }
    }
    out
}

fn load_tokenizer_subset() -> Vec<TokenizerCase> {
    let raw = fs::read_to_string("tests/html5lib/tokenizer_subset.json").unwrap();
    let json: Value = serde_json::from_str(&raw).unwrap();
    let items = json.as_array().cloned().unwrap_or_default();
    items
        .iter()
        .map(|item| TokenizerCase {
            id: item["id"].as_str().unwrap_or("").to_string(),
            category: item["category"].as_str().unwrap_or("").to_string(),
            input: item["input"].as_str().unwrap_or("").to_string(),
            expected: item["expected"].as_array().cloned().unwrap_or_default(),
        })
        .collect()
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

fn normalize_tree_template_markers(tree: &str) -> String {
    let mut normalized = String::new();
    for (index, line) in tree.lines().enumerate() {
        if index > 0 {
            normalized.push('\n');
        }
        normalized.push_str(&normalize_tree_line(line));
    }
    normalized
}

fn normalize_tree_line(line: &str) -> String {
    if let Some(after_pipe) = line.strip_prefix("| ") {
        let indent_len = after_pipe.chars().take_while(|c| *c == ' ').count();
        let indent = &after_pipe[..indent_len];
        let content = &after_pipe[indent_len..];
        if content == "<template-content>" || content == "content" {
            return format!("| {}content", indent);
        }
    }
    line.to_string()
}
