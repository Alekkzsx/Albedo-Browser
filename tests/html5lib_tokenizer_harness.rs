use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use albedo::ace::html::{
    build_document_with_errors, build_fragment_with_errors, HtmlNode, HtmlToken, HtmlTokenizer,
    Namespace,
};
use serde_json::{Map, Value};

const REQUIRED_MANIFEST: &str = "tests/ace_html_conformance/required.json";
const NON_REQUIRED_MANIFEST: &str = "tests/ace_html_conformance/non_required.json";

#[derive(Clone, Debug, PartialEq, Eq)]
enum CaseMode {
    Tokenizer,
    Document,
    Fragment,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ExpectedToken {
    StartTag {
        name: String,
        attrs: BTreeMap<String, String>,
        self_closing: bool,
    },
    EndTag {
        name: String,
    },
    Character {
        data: String,
    },
    Comment {
        data: String,
    },
    Doctype {
        name: Option<String>,
        public_id: Option<String>,
        system_id: Option<String>,
        force_quirks: bool,
    },
}

#[derive(Clone, Debug)]
struct ConformanceCase {
    case_id: String,
    spec_ref: String,
    mode: CaseMode,
    input: String,
    context: Option<String>,
    expected_tokens: Option<Vec<ExpectedToken>>,
    expected_tree: Option<String>,
    expected_errors: Vec<String>,
}

#[derive(Default)]
struct RunStats {
    total: usize,
    passed: usize,
    failures: Vec<String>,
}

#[derive(Clone, Debug)]
struct CaseResult {
    tokens: Option<Vec<ExpectedToken>>,
    tree: Option<String>,
    errors: Vec<String>,
}

#[test]
fn ace_html_required_conformance_passes_100() {
    let cases = load_manifest(REQUIRED_MANIFEST)
        .unwrap_or_else(|err| panic!("failed to load required conformance manifest: {err}"));

    let mut stats = RunStats::default();

    for case in &cases {
        stats.total += 1;
        match run_case(case) {
            Ok(result) => {
                if let Err(detail) = assert_case(case, &result) {
                    stats
                        .failures
                        .push(format!("{} [{}] -> {}", case.case_id, case.spec_ref, detail));
                } else {
                    stats.passed += 1;
                }
            }
            Err(err) => stats
                .failures
                .push(format!("{} [{}] -> {}", case.case_id, case.spec_ref, err)),
        }
    }

    let pass_rate = if stats.total == 0 {
        1.0
    } else {
        stats.passed as f64 / stats.total as f64
    };

    assert!(
        pass_rate == 1.0 && stats.total >= 19,
        "required conformance must be 100% and total tests must be >= 19 (got {:.2}%, {}/{}). first failures: {:?}",
        pass_rate * 100.0,
        stats.passed,
        stats.total,
        stats.failures.into_iter().take(10).collect::<Vec<_>>()
    );
}

#[test]
fn ace_html_non_required_smoke_report() {
    let cases = load_manifest(NON_REQUIRED_MANIFEST)
        .unwrap_or_else(|err| panic!("failed to load non-required conformance manifest: {err}"));

    let mut failures = Vec::new();
    for case in &cases {
        let result = std::panic::catch_unwind(|| run_case(case));
        match result {
            Ok(Ok(out)) => {
                if let Err(detail) = assert_case(case, &out) {
                    failures.push(format!(
                        "{} [{}] -> {}",
                        case.case_id, case.spec_ref, detail
                    ));
                }
            }
            Ok(Err(err)) => failures.push(format!(
                "{} [{}] -> execution error: {}",
                case.case_id, case.spec_ref, err
            )),
            Err(_) => failures.push(format!(
                "{} [{}] -> panic during execution",
                case.case_id, case.spec_ref
            )),
        }
    }

    if !failures.is_empty() {
        eprintln!(
            "non-required conformance failures (does not block): {:?}",
            failures
        );
    }
}

fn run_case(case: &ConformanceCase) -> Result<CaseResult, String> {
    match case.mode {
        CaseMode::Tokenizer => {
            let mut tokenizer = HtmlTokenizer::new(&case.input);
            let mut tokens = Vec::new();
            let mut guard = 0usize;

            loop {
                guard += 1;
                if guard > 8192 {
                    return Err(format!(
                        "tokenizer did not terminate within 8192 iterations for {:?}",
                        case.input
                    ));
                }

                let token = tokenizer.next_token();
                if let Some(mapped) = ExpectedToken::from_runtime(token) {
                    if let Some(ExpectedToken::Character { data: previous }) = tokens.last_mut() {
                        if let ExpectedToken::Character { data: next } = mapped {
                            previous.push_str(&next);
                            continue;
                        }
                    }
                    tokens.push(mapped);
                } else {
                    break;
                }
            }

            let errors = tokenizer
                .errors()
                .iter()
                .map(|e| format!("{}:{:?}", e.code, e.kind))
                .collect::<Vec<_>>();

            Ok(CaseResult {
                tokens: Some(tokens),
                tree: None,
                errors,
            })
        }
        CaseMode::Document => {
            let output = build_document_with_errors(&case.input);
            let tree = serialize_document(
                output.document.doctype.as_ref(),
                &output.document.children,
            );
            let errors = output
                .errors
                .iter()
                .map(|e| format!("{:?}:{:?}", e.source, e.kind))
                .collect::<Vec<_>>();

            Ok(CaseResult {
                tokens: None,
                tree: Some(tree),
                errors,
            })
        }
        CaseMode::Fragment => {
            let output = build_fragment_with_errors(&case.input, case.context.as_deref());
            let tree = serialize_nodes(&output.document.children);
            let errors = output
                .errors
                .iter()
                .map(|e| format!("{:?}:{:?}", e.source, e.kind))
                .collect::<Vec<_>>();

            Ok(CaseResult {
                tokens: None,
                tree: Some(tree),
                errors,
            })
        }
    }
}

fn assert_case(case: &ConformanceCase, result: &CaseResult) -> Result<(), String> {
    match case.mode {
        CaseMode::Tokenizer => {
            let expected = case
                .expected_tokens
                .as_ref()
                .ok_or_else(|| format!("{}: missing expected_tokens", case.case_id))?;
            let actual = result
                .tokens
                .as_ref()
                .ok_or_else(|| format!("{}: tokenizer run produced no tokens", case.case_id))?;

            if actual != expected {
                return Err(format!("tokens mismatch. expected {:?}, got {:?}", expected, actual));
            }
        }
        CaseMode::Document | CaseMode::Fragment => {
            let expected = case
                .expected_tree
                .as_ref()
                .ok_or_else(|| format!("{}: missing expected_tree", case.case_id))?;
            let actual = result
                .tree
                .as_ref()
                .ok_or_else(|| format!("{}: parse run produced no tree", case.case_id))?;

            if actual.trim() != expected.trim() {
                return Err(format!(
                    "tree mismatch. expected:\n{}\nactual:\n{}",
                    expected, actual
                ));
            }
        }
    }

    let mut expected_errors = case.expected_errors.clone();
    let mut actual_errors = result.errors.clone();
    expected_errors.sort();
    actual_errors.sort();

    if expected_errors != actual_errors {
        return Err(format!(
            "errors mismatch. expected {:?}, got {:?}",
            expected_errors, actual_errors
        ));
    }

    Ok(())
}

impl ExpectedToken {
    fn from_runtime(token: HtmlToken) -> Option<Self> {
        match token {
            HtmlToken::StartTag(tag) => Some(Self::StartTag {
                name: tag.name,
                attrs: tag.attributes.into_iter().collect(),
                self_closing: tag.self_closing,
            }),
            HtmlToken::EndTag(tag) => Some(Self::EndTag { name: tag.name }),
            HtmlToken::Character(text) => Some(Self::Character { data: text.data }),
            HtmlToken::Comment(comment) => Some(Self::Comment { data: comment.data }),
            HtmlToken::Doctype(dt) => Some(Self::Doctype {
                name: dt.name,
                public_id: dt.public_id,
                system_id: dt.system_id,
                force_quirks: dt.force_quirks,
            }),
            HtmlToken::Eof => None,
        }
    }
}

fn load_manifest(path: &str) -> Result<Vec<ConformanceCase>, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("cannot read {path}: {err}"))?;
    let json: Value = serde_json::from_str(&raw)
        .map_err(|err| format!("invalid JSON in {path}: {err}"))?;

    let root = json
        .as_object()
        .ok_or_else(|| format!("{path} must be a JSON object"))?;

    let _suite_name = required_str(root, "suite")?;
    let _version = root
        .get("version")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{path}: missing or invalid numeric field 'version'"))?;

    let cases_val = root
        .get("cases")
        .ok_or_else(|| format!("{path}: missing field 'cases'"))?;
    let cases_arr = cases_val
        .as_array()
        .ok_or_else(|| format!("{path}: 'cases' must be an array"))?;

    let mut out = Vec::with_capacity(cases_arr.len());
    for (idx, item) in cases_arr.iter().enumerate() {
        let obj = item
            .as_object()
            .ok_or_else(|| format!("{path}: case #{idx} must be an object"))?;

        let case_id = required_str(obj, "case_id")?.to_string();
        let spec_ref = required_str(obj, "spec_ref")?.to_string();
        let mode = parse_mode(required_str(obj, "mode")?)
            .map_err(|err| format!("{path}: {case_id}: {err}"))?;
        let input = required_str(obj, "input")?.to_string();

        let context = obj
            .get("context")
            .and_then(Value::as_str)
            .map(str::to_string);

        let expected_errors = parse_expected_errors(obj.get("expected_errors"))
            .map_err(|err| format!("{path}: {case_id}: {err}"))?;

        let expected_tokens = match mode {
            CaseMode::Tokenizer => {
                let val = obj.get("expected_tokens").ok_or_else(|| {
                    format!("{path}: {case_id}: missing 'expected_tokens' for tokenizer mode")
                })?;
                Some(
                    parse_expected_tokens(val)
                        .map_err(|err| format!("{path}: {case_id}: {err}"))?,
                )
            }
            _ => None,
        };

        let expected_tree = match mode {
            CaseMode::Document | CaseMode::Fragment => Some(
                required_str(obj, "expected_tree")
                    .map_err(|err| format!("{path}: {case_id}: {err}"))?
                    .to_string(),
            ),
            _ => None,
        };

        out.push(ConformanceCase {
            case_id,
            spec_ref,
            mode,
            input,
            context,
            expected_tokens,
            expected_tree,
            expected_errors,
        });
    }

    Ok(out)
}

fn parse_mode(mode: &str) -> Result<CaseMode, String> {
    match mode {
        "tokenizer" => Ok(CaseMode::Tokenizer),
        "document" => Ok(CaseMode::Document),
        "fragment" => Ok(CaseMode::Fragment),
        _ => Err(format!("unsupported mode '{mode}'")),
    }
}

fn parse_expected_tokens(value: &Value) -> Result<Vec<ExpectedToken>, String> {
    let arr = value
        .as_array()
        .ok_or_else(|| "expected_tokens must be an array".to_string())?;

    let mut out = Vec::with_capacity(arr.len());
    for token in arr {
        let obj = token
            .as_object()
            .ok_or_else(|| "token entry must be an object".to_string())?;
        out.push(parse_expected_token(obj)?);
    }

    Ok(out)
}

fn parse_expected_token(obj: &Map<String, Value>) -> Result<ExpectedToken, String> {
    let token_type = required_str(obj, "type")?;
    match token_type {
        "StartTag" => {
            let name = required_str(obj, "name")?.to_ascii_lowercase();
            let attrs = parse_attrs(obj.get("attrs"))?;
            let self_closing = obj
                .get("self_closing")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            Ok(ExpectedToken::StartTag {
                name,
                attrs,
                self_closing,
            })
        }
        "EndTag" => Ok(ExpectedToken::EndTag {
            name: required_str(obj, "name")?.to_ascii_lowercase(),
        }),
        "Character" => Ok(ExpectedToken::Character {
            data: required_str(obj, "data")?.to_string(),
        }),
        "Comment" => Ok(ExpectedToken::Comment {
            data: required_str(obj, "data")?.to_string(),
        }),
        "Doctype" => Ok(ExpectedToken::Doctype {
            name: optional_str(obj.get("name")).map(|s| s.to_ascii_lowercase()),
            public_id: optional_str(obj.get("public_id")),
            system_id: optional_str(obj.get("system_id")),
            force_quirks: obj
                .get("force_quirks")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        _ => Err(format!("unsupported token type '{token_type}'")),
    }
}

fn parse_expected_errors(value: Option<&Value>) -> Result<Vec<String>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };

    let arr = value
        .as_array()
        .ok_or_else(|| "expected_errors must be an array".to_string())?;

    let mut out = Vec::with_capacity(arr.len());
    for val in arr {
        let s = val
            .as_str()
            .ok_or_else(|| "expected_errors must contain only strings".to_string())?;
        out.push(s.to_string());
    }
    Ok(out)
}

fn parse_attrs(value: Option<&Value>) -> Result<BTreeMap<String, String>, String> {
    let mut attrs = BTreeMap::new();

    let Some(value) = value else {
        return Ok(attrs);
    };

    let obj = value
        .as_object()
        .ok_or_else(|| "attrs must be an object".to_string())?;

    for (k, v) in obj {
        let value = v
            .as_str()
            .ok_or_else(|| format!("attribute '{k}' must be string"))?;
        attrs.insert(k.to_string(), value.to_string());
    }

    Ok(attrs)
}

fn required_str<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    obj.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing or invalid string field '{key}'"))
}

fn optional_str(value: Option<&Value>) -> Option<String> {
    let value = value?;
    if value.is_null() {
        None
    } else {
        value.as_str().map(str::to_string)
    }
}

fn serialize_document(doctype: Option<&albedo::ace::html::DoctypeToken>, nodes: &[HtmlNode]) -> String {
    let mut out = String::new();

    if let Some(dt) = doctype {
        let name = dt.name.as_deref().unwrap_or("");
        out.push_str("| <!DOCTYPE ");
        out.push_str(name);
        out.push_str(">\n");
    }

    out.push_str(&serialize_nodes(nodes));
    out
}

fn serialize_nodes(nodes: &[HtmlNode]) -> String {
    let mut out = String::new();
    for node in nodes {
        serialize_node(node, 0, &mut out);
    }
    out
}

fn serialize_node(node: &HtmlNode, indent: usize, out: &mut String) {
    let prefix = format!("| {}", "  ".repeat(indent));

    match node {
        HtmlNode::Element(el) => {
            let ns_prefix = match el.namespace {
                Namespace::Html => "",
                Namespace::Svg => "svg ",
                Namespace::MathMl => "math ",
            };

            out.push_str(&format!("{}<{}{}>\n", prefix, ns_prefix, el.tag));

            let mut attrs: Vec<_> = el.attributes.iter().collect();
            attrs.sort_by(|a, b| a.0.cmp(b.0));
            for (name, value) in attrs {
                out.push_str(&format!("{}  {}=\"{}\"\n", prefix, name, value));
            }

            for child in &el.children {
                serialize_node(child, indent + 1, out);
            }
        }
        HtmlNode::Text(text) => {
            out.push_str(&format!("{}\"{}\"\n", prefix, text));
        }
        HtmlNode::Comment(text) => {
            out.push_str(&format!("{}<!-- {} -->\n", prefix, text));
        }
    }
}

#[test]
fn required_manifest_exists() {
    assert!(Path::new(REQUIRED_MANIFEST).exists());
}

#[test]
fn non_required_manifest_exists() {
    assert!(Path::new(NON_REQUIRED_MANIFEST).exists());
}
