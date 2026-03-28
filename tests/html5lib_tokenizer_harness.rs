use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use albedo::ace::html::{HtmlToken, HtmlTokenizer};
use serde_json::{Map, Value};

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
struct FixtureCase {
    id: String,
    category: String,
    input: String,
    expected: Vec<ExpectedToken>,
}

#[derive(Default)]
struct ConformanceStats {
    total: usize,
    passed: usize,
    by_category: HashMap<String, (usize, usize)>,
    failures: Vec<String>,
}

impl ConformanceStats {
    fn record(&mut self, category: &str, id: &str, ok: bool, detail: Option<String>) {
        self.total += 1;
        if ok {
            self.passed += 1;
        }

        let entry = self
            .by_category
            .entry(category.to_string())
            .or_insert((0usize, 0usize));
        if ok {
            entry.0 += 1;
        }
        entry.1 += 1;

        if !ok {
            if let Some(detail) = detail {
                self.failures.push(format!("{id}: {detail}"));
            } else {
                self.failures.push(id.to_string());
            }
        }
    }

    fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            return 1.0;
        }
        self.passed as f64 / self.total as f64
    }

    fn category_report(&self) -> String {
        let mut entries: Vec<_> = self.by_category.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));

        let mut report = String::new();
        for (idx, (name, (passed, total))) in entries.iter().enumerate() {
            if idx > 0 {
                report.push_str(", ");
            }
            report.push_str(&format!("{name}:{passed}/{total}"));
        }
        report
    }
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

#[test]
fn html5lib_tokenizer_subset_conformance() {
    let subset_path = Path::new("tests/html5lib/tokenizer_subset.json");
    let cases = load_custom_subset_cases(subset_path)
        .unwrap_or_else(|err| panic!("failed to load subset fixtures: {err}"));

    let stats = run_cases(&cases);
    let pass_rate = stats.pass_rate();

    assert!(
        pass_rate >= 0.95,
        "subset conformance below target: {:.2}% ({}/{}). categories: {}. first failures: {:?}",
        pass_rate * 100.0,
        stats.passed,
        stats.total,
        stats.category_report(),
        stats.failures.into_iter().take(8).collect::<Vec<_>>()
    );
}

#[test]
fn html5lib_tokenizer_external_relevant_fixtures() {
    let Ok(raw_dir) = std::env::var("HTML5LIB_TOKENIZER_DIR") else {
        return;
    };

    let fixture_dir = PathBuf::from(raw_dir);
    let cases = load_external_html5lib_cases(&fixture_dir)
        .unwrap_or_else(|err| panic!("failed to load external html5lib fixtures: {err}"));

    if cases.is_empty() {
        return;
    }

    let stats = run_cases(&cases);
    let pass_rate = stats.pass_rate();

    assert!(
        pass_rate >= 0.95,
        "external html5lib conformance below target: {:.2}% ({}/{}). categories: {}. first failures: {:?}",
        pass_rate * 100.0,
        stats.passed,
        stats.total,
        stats.category_report(),
        stats.failures.into_iter().take(12).collect::<Vec<_>>()
    );
}

fn run_cases(cases: &[FixtureCase]) -> ConformanceStats {
    let mut stats = ConformanceStats::default();

    for case in cases {
        let (ok, detail) = match tokenize(&case.input) {
            Ok(actual) => {
                let ok = actual == case.expected;
                let detail = if ok {
                    None
                } else {
                    Some(format!("expected {:?}, got {:?}", case.expected, actual))
                };
                (ok, detail)
            }
            Err(err) => (false, Some(err)),
        };

        stats.record(&case.category, &case.id, ok, detail);
    }

    stats
}

fn tokenize(input: &str) -> Result<Vec<ExpectedToken>, String> {
    let mut tokenizer = HtmlTokenizer::new(input);
    let mut out = Vec::new();
    let mut guard = 0usize;

    loop {
        guard += 1;
        if guard > 4096 {
            return Err(format!(
                "tokenizer did not terminate within 4096 steps for input {:?}",
                input
            ));
        }

        let token = tokenizer.next_token();
        if let Some(mapped) = ExpectedToken::from_runtime(token) {
            if let Some(ExpectedToken::Character { data: previous }) = out.last_mut() {
                if let ExpectedToken::Character { data: next } = mapped {
                    previous.push_str(&next);
                    continue;
                }
            }
            out.push(mapped);
        } else {
            break;
        }
    }

    Ok(out)
}

fn load_custom_subset_cases(path: &Path) -> Result<Vec<FixtureCase>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    let json: Value = serde_json::from_str(&raw)
        .map_err(|err| format!("invalid JSON in {}: {err}", path.display()))?;

    let arr = json
        .as_array()
        .ok_or_else(|| format!("{} must be a JSON array", path.display()))?;

    let mut out = Vec::with_capacity(arr.len());
    for (index, item) in arr.iter().enumerate() {
        let obj = item
            .as_object()
            .ok_or_else(|| format!("case #{index} must be an object"))?;

        let id = get_required_str(obj, "id")?.to_string();
        let category = get_required_str(obj, "category")?.to_string();
        let input = get_required_str(obj, "input")?.to_string();
        let expected_value = obj
            .get("expected")
            .ok_or_else(|| format!("{id}: missing field 'expected'"))?;

        let expected = parse_custom_expected_tokens(expected_value)
            .map_err(|err| format!("{id}: {err}"))?;

        out.push(FixtureCase {
            id,
            category,
            input,
            expected,
        });
    }

    Ok(out)
}

fn parse_custom_expected_tokens(value: &Value) -> Result<Vec<ExpectedToken>, String> {
    let arr = value
        .as_array()
        .ok_or_else(|| "'expected' must be an array".to_string())?;

    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let obj = item
            .as_object()
            .ok_or_else(|| "token must be an object".to_string())?;
        out.push(parse_custom_expected_token(obj)?);
    }

    Ok(out)
}

fn parse_custom_expected_token(obj: &Map<String, Value>) -> Result<ExpectedToken, String> {
    let token_type = get_required_str(obj, "type")?;
    match token_type {
        "StartTag" => {
            let name = get_required_str(obj, "name")?.to_ascii_lowercase();
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
            name: get_required_str(obj, "name")?.to_ascii_lowercase(),
        }),
        "Character" => Ok(ExpectedToken::Character {
            data: get_required_str(obj, "data")?.to_string(),
        }),
        "Comment" => Ok(ExpectedToken::Comment {
            data: get_required_str(obj, "data")?.to_string(),
        }),
        "Doctype" => Ok(ExpectedToken::Doctype {
            name: get_optional_str(obj.get("name")).map(|s| s.to_ascii_lowercase()),
            public_id: get_optional_str(obj.get("public_id")),
            system_id: get_optional_str(obj.get("system_id")),
            force_quirks: obj
                .get("force_quirks")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        _ => Err(format!("unsupported token type '{token_type}'")),
    }
}

fn load_external_html5lib_cases(dir: &Path) -> Result<Vec<FixtureCase>, String> {
    let mut files = Vec::new();
    collect_json_files(dir, &mut files)?;

    let mut cases = Vec::new();
    for path in files {
        let content = fs::read_to_string(&path)
            .map_err(|err| format!("cannot read {}: {err}", path.display()))?;
        let json: Value = match serde_json::from_str(&content) {
            Ok(value) => value,
            Err(_) => continue,
        };

        let file_cases = parse_html5lib_file(&path, &json);
        cases.extend(file_cases);
    }

    Ok(cases)
}

fn collect_json_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut stack = vec![root.to_path_buf()];

    while let Some(path) = stack.pop() {
        let entries = fs::read_dir(&path)
            .map_err(|err| format!("cannot read dir {}: {err}", path.display()))?;

        for entry in entries {
            let entry = entry.map_err(|err| format!("read_dir error in {}: {err}", path.display()))?;
            let entry_path = entry.path();
            if entry_path.is_dir() {
                stack.push(entry_path);
                continue;
            }

            let is_json = entry_path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json") || ext.eq_ignore_ascii_case("test"));
            if is_json {
                out.push(entry_path);
            }
        }
    }

    out.sort();
    Ok(())
}

fn parse_html5lib_file(path: &Path, json: &Value) -> Vec<FixtureCase> {
    let mut out = Vec::new();

    let tests = if let Some(root_obj) = json.as_object() {
        root_obj
            .get("tests")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    } else if let Some(arr) = json.as_array() {
        arr.clone()
    } else {
        Vec::new()
    };

    let file_label = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    for (index, test_case) in tests.iter().enumerate() {
        let Some(obj) = test_case.as_object() else {
            continue;
        };

        if should_skip_external_case(obj) {
            continue;
        }

        let Some(input) = obj.get("input").and_then(Value::as_str) else {
            continue;
        };

        let Some(output) = obj.get("output").and_then(Value::as_array) else {
            continue;
        };

        let mut expected = Vec::new();
        let mut unsupported = false;
        for token in output {
            let Some(mapped) = parse_html5lib_output_token(token) else {
                unsupported = true;
                break;
            };
            expected.push(mapped);
        }

        if unsupported {
            continue;
        }

        let id = obj
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("{file_label}#{index}"));

        out.push(FixtureCase {
            id,
            category: file_label.to_string(),
            input: input.to_string(),
            expected,
        });
    }

    out
}

fn should_skip_external_case(obj: &Map<String, Value>) -> bool {
    if obj.get("lastStartTag").is_some() {
        return true;
    }

    if obj
        .get("doubleEscaped")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return true;
    }

    if let Some(states) = obj.get("initialStates").and_then(Value::as_array) {
        let has_data_state = states
            .iter()
            .any(|state| state.as_str() == Some("Data state"));
        if !has_data_state {
            return true;
        }
    }

    false
}

fn parse_html5lib_output_token(value: &Value) -> Option<ExpectedToken> {
    let arr = value.as_array()?;
    let kind = arr.first()?.as_str()?;

    match kind {
        "StartTag" => {
            let name = arr.get(1)?.as_str()?.to_ascii_lowercase();
            let attrs_obj = arr.get(2)?.as_object()?;
            let mut attrs = BTreeMap::new();
            for (key, val) in attrs_obj {
                attrs.insert(key.to_string(), val.as_str()?.to_string());
            }
            let self_closing = arr.get(3).and_then(Value::as_bool).unwrap_or(false);

            Some(ExpectedToken::StartTag {
                name,
                attrs,
                self_closing,
            })
        }
        "EndTag" => Some(ExpectedToken::EndTag {
            name: arr.get(1)?.as_str()?.to_ascii_lowercase(),
        }),
        "Character" => Some(ExpectedToken::Character {
            data: arr.get(1)?.as_str()?.to_string(),
        }),
        "Comment" => Some(ExpectedToken::Comment {
            data: arr.get(1)?.as_str()?.to_string(),
        }),
        "DOCTYPE" => {
            let name = value_opt_string(arr.get(1));
            let public_id = value_opt_string(arr.get(2));
            let system_id = value_opt_string(arr.get(3));
            let force_quirks = arr.get(4).and_then(Value::as_bool).unwrap_or(false);
            Some(ExpectedToken::Doctype {
                name: name.map(|s| s.to_ascii_lowercase()),
                public_id,
                system_id,
                force_quirks,
            })
        }
        _ => None,
    }
}

fn parse_attrs(value: Option<&Value>) -> Result<BTreeMap<String, String>, String> {
    let mut attrs = BTreeMap::new();

    let Some(value) = value else {
        return Ok(attrs);
    };

    let Some(obj) = value.as_object() else {
        return Err("attrs must be an object".to_string());
    };

    for (key, val) in obj {
        let Some(string_value) = val.as_str() else {
            return Err(format!("attribute '{key}' must be a string"));
        };
        attrs.insert(key.to_string(), string_value.to_string());
    }

    Ok(attrs)
}

fn get_required_str<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    obj.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing or invalid string field '{key}'"))
}

fn get_optional_str(value: Option<&Value>) -> Option<String> {
    value_opt_string(value)
}

fn value_opt_string(value: Option<&Value>) -> Option<String> {
    let value = value?;
    if value.is_null() {
        None
    } else {
        value.as_str().map(str::to_string)
    }
}
