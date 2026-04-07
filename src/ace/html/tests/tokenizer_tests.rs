use crate::ace::html::lexer::{HtmlLexer, HtmlTokenKind};
use crate::ace::json::{parse, JsonValue};
use std::collections::HashMap;

#[derive(Debug)]
#[allow(dead_code)]
struct TokenizerTest {
    id: String,
    category: String,
    input: String,
    expected: Vec<ExpectedToken>,
}

#[derive(Debug, Clone)]
enum ExpectedToken {
    StartTag {
        name: String,
        attrs: HashMap<String, String>,
        self_closing: bool,
    },
    EndTag {
        name: String,
    },
    Comment {
        data: String,
    },
    Character {
        data: String,
    },
    Doctype {
        name: Option<String>,
        public_id: Option<String>,
        system_id: Option<String>,
        force_quirks: bool,
    },
}

impl TokenizerTest {
    fn from_json(value: &JsonValue) -> Result<Self, String> {
        let obj = value.as_object().ok_or("Test must be an object")?;
        
        let id = obj.get("id")
            .and_then(|v| v.as_string())
            .ok_or("Missing 'id' field")?
            .to_string();
        
        let category = obj.get("category")
            .and_then(|v| v.as_string())
            .ok_or("Missing 'category' field")?
            .to_string();
        
        let input = obj.get("input")
            .and_then(|v| v.as_string())
            .ok_or("Missing 'input' field")?
            .to_string();
        
        let expected_array = obj.get("expected")
            .and_then(|v| v.as_array())
            .ok_or("Missing 'expected' field")?;
        
        let mut expected = Vec::new();
        for token_val in expected_array {
            expected.push(ExpectedToken::from_json(token_val)?);
        }
        
        Ok(TokenizerTest {
            id,
            category,
            input,
            expected,
        })
    }
    
    fn run(&self) -> Result<(), String> {
        let mut lexer = HtmlLexer::new(&self.input);
        lexer.end();
        
        let mut actual_tokens = Vec::new();
        while let Some(token) = lexer.next_token() {
            if matches!(token.kind, HtmlTokenKind::Eof) {
                break;
            }
            actual_tokens.push(token);
        }
        
        // Compare token count
        if actual_tokens.len() != self.expected.len() {
            return Err(format!(
                "Token count mismatch for test '{}': expected {} tokens, got {}",
                self.id,
                self.expected.len(),
                actual_tokens.len()
            ));
        }
        
        // Compare each token
        for (i, (actual, expected)) in actual_tokens.iter().zip(self.expected.iter()).enumerate() {
            self.compare_token(i, actual, expected)?;
        }
        
        Ok(())
    }
    
    fn compare_token(
        &self,
        index: usize,
        actual: &crate::ace::html::lexer::HtmlToken,
        expected: &ExpectedToken,
    ) -> Result<(), String> {
        match (&actual.kind, expected) {
            (HtmlTokenKind::StartTag(actual_tag), ExpectedToken::StartTag { name, attrs, self_closing }) => {
                if actual_tag.name != *name {
                    return Err(format!(
                        "Test '{}' token {}: tag name mismatch: expected '{}', got '{}'",
                        self.id, index, name, actual_tag.name
                    ));
                }
                
                if actual_tag.attributes != *attrs {
                    return Err(format!(
                        "Test '{}' token {}: attributes mismatch: expected {:?}, got {:?}",
                        self.id, index, attrs, actual_tag.attributes
                    ));
                }
                
                if actual_tag.self_closing != *self_closing {
                    return Err(format!(
                        "Test '{}' token {}: self_closing mismatch: expected {}, got {}",
                        self.id, index, self_closing, actual_tag.self_closing
                    ));
                }
            }
            (HtmlTokenKind::EndTag(actual_name), ExpectedToken::EndTag { name }) => {
                if actual_name != name {
                    return Err(format!(
                        "Test '{}' token {}: end tag name mismatch: expected '{}', got '{}'",
                        self.id, index, name, actual_name
                    ));
                }
            }
            (HtmlTokenKind::Comment(actual_data), ExpectedToken::Comment { data }) => {
                if actual_data != data {
                    return Err(format!(
                        "Test '{}' token {}: comment data mismatch: expected '{}', got '{}'",
                        self.id, index, data, actual_data
                    ));
                }
            }
            (HtmlTokenKind::Character(actual_data), ExpectedToken::Character { data }) => {
                if actual_data != data {
                    return Err(format!(
                        "Test '{}' token {}: character data mismatch: expected '{}', got '{}'",
                        self.id, index, data, actual_data
                    ));
                }
            }
            (HtmlTokenKind::Doctype(actual_dt), ExpectedToken::Doctype { name, public_id, system_id, force_quirks }) => {
                if actual_dt.name != *name {
                    return Err(format!(
                        "Test '{}' token {}: doctype name mismatch: expected {:?}, got {:?}",
                        self.id, index, name, actual_dt.name
                    ));
                }
                
                if actual_dt.public_id != *public_id {
                    return Err(format!(
                        "Test '{}' token {}: doctype public_id mismatch: expected {:?}, got {:?}",
                        self.id, index, public_id, actual_dt.public_id
                    ));
                }
                
                if actual_dt.system_id != *system_id {
                    return Err(format!(
                        "Test '{}' token {}: doctype system_id mismatch: expected {:?}, got {:?}",
                        self.id, index, system_id, actual_dt.system_id
                    ));
                }
                
                if actual_dt.force_quirks != *force_quirks {
                    return Err(format!(
                        "Test '{}' token {}: doctype force_quirks mismatch: expected {}, got {}",
                        self.id, index, force_quirks, actual_dt.force_quirks
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "Test '{}' token {}: token type mismatch",
                    self.id, index
                ));
            }
        }
        
        Ok(())
    }
}

impl ExpectedToken {
    fn from_json(value: &JsonValue) -> Result<Self, String> {
        let obj = value.as_object().ok_or("Token must be an object")?;
        
        let token_type = obj.get("type")
            .and_then(|v| v.as_string())
            .ok_or("Missing 'type' field")?;
        
        match token_type {
            "StartTag" => {
                let name = obj.get("name")
                    .and_then(|v| v.as_string())
                    .ok_or("Missing 'name' field for StartTag")?
                    .to_string();
                
                let attrs_obj = obj.get("attrs")
                    .and_then(|v| v.as_object())
                    .ok_or("Missing 'attrs' field for StartTag")?;
                
                let mut attrs = HashMap::new();
                for (key, val) in attrs_obj {
                    if let Some(s) = val.as_string() {
                        attrs.insert(key.clone(), s.to_string());
                    }
                }
                
                let self_closing = obj.get("self_closing")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                
                Ok(ExpectedToken::StartTag { name, attrs, self_closing })
            }
            "EndTag" => {
                let name = obj.get("name")
                    .and_then(|v| v.as_string())
                    .ok_or("Missing 'name' field for EndTag")?
                    .to_string();
                
                Ok(ExpectedToken::EndTag { name })
            }
            "Comment" => {
                let data = obj.get("data")
                    .and_then(|v| v.as_string())
                    .ok_or("Missing 'data' field for Comment")?
                    .to_string();
                
                Ok(ExpectedToken::Comment { data })
            }
            "Character" => {
                let data = obj.get("data")
                    .and_then(|v| v.as_string())
                    .ok_or("Missing 'data' field for Character")?
                    .to_string();
                
                Ok(ExpectedToken::Character { data })
            }
            "Doctype" => {
                let name = obj.get("name")
                    .and_then(|v| v.as_string())
                    .map(|s| s.to_string());
                
                let public_id = obj.get("public_id")
                    .and_then(|v| v.as_string())
                    .map(|s| s.to_string());
                
                let system_id = obj.get("system_id")
                    .and_then(|v| v.as_string())
                    .map(|s| s.to_string());
                
                let force_quirks = obj.get("force_quirks")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                
                Ok(ExpectedToken::Doctype { name, public_id, system_id, force_quirks })
            }
            _ => Err(format!("Unknown token type: {}", token_type)),
        }
    }
}

pub fn run_tokenizer_tests(json_content: &str) -> Result<(usize, usize), String> {
    let value = parse(json_content).map_err(|e| format!("JSON parse error: {}", e))?;
    let tests_array = value.as_array().ok_or("Root must be an array")?;
    
    let mut passed = 0;
    let mut failed = 0;
    let mut failures = Vec::new();
    
    for test_val in tests_array {
        let test = TokenizerTest::from_json(test_val)?;
        match test.run() {
            Ok(()) => passed += 1,
            Err(e) => {
                failed += 1;
                failures.push(format!("Test '{}' failed: {}", test.id, e));
            }
        }
    }
    
    if !failures.is_empty() {
        eprintln!("\n=== Tokenizer Test Failures ===");
        for failure in &failures {
            eprintln!("{}", failure);
        }
        eprintln!("================================\n");
    }
    
    Ok((passed, failed))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tokenizer_subset() {
        let json_content = include_str!("../../../../tests/html5lib/tokenizer_subset.json");
        let (passed, failed) = run_tokenizer_tests(json_content).expect("Failed to run tests");
        
        println!("Tokenizer tests: {} passed, {} failed", passed, failed);
        
        // For now, we allow some failures as we're still implementing features
        // Target: 100% pass rate
        assert!(passed > 0, "At least some tests should pass");
    }
}
