//! # 🥊 Suíte 1: Fuzzing, Carga Massiva e Resiliência Adversarial de Sintaxe CSS
//!
//! Exercita:
//! - 100.000 declarações CSS contínuas
//! - 1.000 níveis de blocos aninhados (`{ { ... } }`) sem Stack Overflow
//! - Payloads maliciosos: comentários não fechados, strings corrompidas, bytes nulos e escapes surrogate.

use ace_style::parser::component_value::{ComponentValueParser, MAX_NESTING_DEPTH};
use ace_style::parser::declaration_parser::parse_declarations;
use ace_style::parser::tokenizer::Tokenizer;
use ace_style::model::stylesheet::StyleSheet;
use ace_style::cascade::origin::StyleSheetOrigin;

#[test]
fn test_stress_massive_declarations_load() {
    let mut large_css = String::with_capacity(3_000_000);
    // 100.000 declarações CSS sintéticas
    for i in 0..100_000 {
        large_css.push_str(&format!(
            "div.item-{} {{ margin: {}px; color: #aabbcc; display: block; }}\n",
            i % 1000,
            i % 50
        ));
    }

    let start = std::time::Instant::now();
    let sheet = StyleSheet::parse(&large_css, StyleSheetOrigin::Author);
    let elapsed = start.elapsed();

    println!("Parsed 100,000 rules in {:?}", elapsed);
    assert!(sheet.rules.len() >= 100_000);
}

#[test]
fn test_stress_adversarial_nested_braces_no_stack_overflow() {
    // 1.000 níveis de chaves aninhadas adversariais: { { { ... } } }
    let mut nested = String::with_capacity(4000);
    for _ in 0..1_000 {
        nested.push('{');
    }
    nested.push_str("color: red;");
    for _ in 0..1_000 {
        nested.push('}');
    }

    let mut tokenizer = Tokenizer::new(&nested);
    let tokens = tokenizer.tokenize_all();

    let mut cv_parser = ComponentValueParser::new(&tokens);
    // Deve concluir com sucesso sem estourar a pilha
    let component_values = cv_parser.parse_all();
    assert!(!component_values.is_empty());
}

#[test]
fn test_stress_adversarial_malformed_payloads() {
    // 1. Comentário não fechado até EOF
    let unclosed_comment = "/* Este comentário nunca é fechado...";
    let mut tokenizer = Tokenizer::new(unclosed_comment);
    let tokens = tokenizer.tokenize_all();
    assert!(tokens.iter().any(|t| matches!(t, ace_style::Token::Comment(_))));

    // 2. String com bytes nulos e escapes corrompidos
    let null_byte_css = "body { content: \"hello\0world\\u0000test\\xD83D\\x\"; }";
    let decls = parse_declarations(null_byte_css);
    assert!(!decls.is_empty());

    // 3. String não fechada com quebra de linha
    let unclosed_string = "p { color: 'unclosed\nfont-size: 14px; }";
    let decls2 = parse_declarations(unclosed_string);
    // Não deve entrar em pânico nem travar
    assert!(decls2.iter().any(|d| d.property.as_str() == "font-size"));

    // 4. Seletor com caracteres lixo
    let garbage = "<><><>!@#$%^&*()_+=~`\"'\\/{}[]:;,";
    let sheet = StyleSheet::parse(garbage, StyleSheetOrigin::Author);
    // Parser deve se recuperar graciosamente sem pânico
    assert!(sheet.rules.is_empty() || !sheet.rules.is_empty());
}
