//! # Suíte de Testes de Reentrância e Streaming do Tokenizer HTML5 (WHATWG §12.2.3 / §12.2.5)
//!
//! Cobertura exaustiva de inserção dinâmica no cabeçalho do fluxo (`document.write`),
//! aninhamento de pontos de inserção (LIFO), preservação cronológica de escritas consecutivas (FIFO),
//! divisão de tokens em limites arbitrários de chunks (tags, atributos, comentários, doctypes, CDATA)
//! e aceleração SIMD sob mutação contínua de buffers.

use ace_core::intern::Atom;
use ace_core::text::SegmentedString;
use ace_dom::tokenizer::{
    HTMLTokenizer, Token, TokenSink, TokenizerAction, TokenizerState,
};
use smol_str::SmolStr;

/// Sink de teste que grava todos os tokens emitidos em ordem linear.
#[derive(Default, Debug)]
struct TestRecorderSink {
    tokens: Vec<Token>,
}

impl TokenSink for TestRecorderSink {
    fn process_token(&mut self, token: Token) -> TokenizerAction {
        self.tokens.push(token);
        TokenizerAction::Continue
    }
}

/// Sink de teste reentrante que simula a execução de scripts disparando `document.write` dinâmico.
struct ScriptReentrantSink {
    tokens: Vec<Token>,
    script_injections: Vec<(Atom, SmolStr)>,
}

impl ScriptReentrantSink {
    fn new(injections: Vec<(&str, &str)>) -> Self {
        Self {
            tokens: Vec::new(),
            script_injections: injections
                .into_iter()
                .map(|(tag, markup)| (Atom::new(tag), SmolStr::new(markup)))
                .collect(),
        }
    }
}

impl TokenSink for ScriptReentrantSink {
    fn process_token(&mut self, token: Token) -> TokenizerAction {
        let action = if let Token::StartTag(ref tag) = token {
            if let Some(pos) = self.script_injections.iter().position(|(t, _)| t == &tag.name) {
                let (_, markup) = self.script_injections.remove(pos);
                TokenizerAction::InsertMarkup(markup)
            } else {
                TokenizerAction::Continue
            }
        } else {
            TokenizerAction::Continue
        };

        self.tokens.push(token);
        action
    }
}

/// Helper para extrair texto contínuo a partir de tokens de caracteres.
fn collect_characters(tokens: &[Token]) -> String {
    let mut s = String::new();
    for tok in tokens {
        if let Token::Character(ref c) = tok {
            s.push_str(c.as_str());
        }
    }
    s
}

#[test]
fn test_dynamic_insertion_consecutive_writes() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::from("<p>tail</p>");

    // Simula script executando duas chamadas consecutivas a document.write
    input.push_insertion_point();
    input.insert_stream_content("<div>First</div>");
    input.insert_stream_content("<span>Second</span>");
    input.pop_insertion_point();

    tokenizer.tokenize(&mut input, &mut sink);

    let start_tags: Vec<String> = sink
        .tokens
        .iter()
        .filter_map(|t| match t {
            Token::StartTag(tag) => Some(tag.name.as_str().to_string()),
            _ => None,
        })
        .collect();

    // Ordem cronológica esperada: "div", "span", "p" (FIFO dentro do ponto de inserção)
    assert_eq!(start_tags, vec!["div", "span", "p"]);
}

#[test]
fn test_dynamic_insertion_nested_writes() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::from("<main>original</main>");

    // Script 1 (outer)
    input.push_insertion_point();
    input.insert_stream_content("<header>outer_start</header>");

    // Script 2 (nested) é executado durante o processamento do outer
    input.push_insertion_point();
    input.insert_stream_content("<nav>nested_1</nav>");
    input.insert_stream_content("<nav>nested_2</nav>");
    input.pop_insertion_point();

    // Script 1 continua
    input.insert_stream_content("<footer>outer_end</footer>");
    input.pop_insertion_point();

    tokenizer.tokenize(&mut input, &mut sink);

    let start_tags: Vec<String> = sink
        .tokens
        .iter()
        .filter_map(|t| match t {
            Token::StartTag(tag) => Some(tag.name.as_str().to_string()),
            _ => None,
        })
        .collect();

    // Ordem LIFO/FIFO aninhada WHATWG: "nav" (nested_1), "nav" (nested_2), "header" (outer_start), "footer" (outer_end), "main" (original)
    assert_eq!(
        start_tags,
        vec!["nav", "nav", "header", "footer", "main"]
    );
}

#[test]
fn test_mid_token_split_tag_name() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::new();

    // Chunk 1: Início da tag
    input.insert_stream_content("<sp");
    tokenizer.tokenize_chunk(&mut input, &mut sink);

    assert_eq!(tokenizer.state, TokenizerState::TagName);

    // Chunk 2: Restante da tag e conteúdo
    input.insert_stream_content("an class='highlight'>Hello</span>");
    tokenizer.tokenize(&mut input, &mut sink);

    assert_eq!(tokenizer.state, TokenizerState::Data);

    let has_span = sink.tokens.iter().any(|t| match t {
        Token::StartTag(tag) => {
            tag.name.as_str() == "span"
                && tag.attributes.iter().any(|a| a.name.as_str() == "class" && a.value.as_str() == "highlight")
        }
        _ => false,
    });
    assert!(has_span);

    let text = collect_characters(&sink.tokens);
    assert_eq!(text, "Hello");
}

#[test]
fn test_mid_token_split_attribute_name_and_value() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::new();

    // Chunk 1: tag com atributo partido no nome
    input.insert_stream_content("<div da");
    tokenizer.tokenize_chunk(&mut input, &mut sink);
    assert_eq!(tokenizer.state, TokenizerState::AttributeName);

    // Chunk 2: fecha nome e abre aspas duplas no valor
    input.insert_stream_content("ta-id=\"usr_");
    tokenizer.tokenize_chunk(&mut input, &mut sink);
    assert_eq!(tokenizer.state, TokenizerState::AttributeValueDoubleQuoted);

    // Chunk 3: complementa valor e abre atributo com aspas simples
    input.insert_stream_content("123\" title='Bo");
    tokenizer.tokenize_chunk(&mut input, &mut sink);
    assert_eq!(tokenizer.state, TokenizerState::AttributeValueSingleQuoted);

    // Chunk 4: fecha aspas simples e fecha tag
    input.insert_stream_content("x'>Conteúdo</div>");
    tokenizer.tokenize(&mut input, &mut sink);
    assert_eq!(tokenizer.state, TokenizerState::Data);

    let start_tag = sink
        .tokens
        .iter()
        .find_map(|t| match t {
            Token::StartTag(tag) if tag.name.as_str() == "div" => Some(tag),
            _ => None,
        })
        .expect("StartTag div deve ter sido emitida");

    assert_eq!(start_tag.attributes.len(), 2);
    assert_eq!(start_tag.attributes[0].name.as_str(), "data-id");
    assert_eq!(start_tag.attributes[0].value.as_str(), "usr_123");
    assert_eq!(start_tag.attributes[1].name.as_str(), "title");
    assert_eq!(start_tag.attributes[1].value.as_str(), "Box");

    let text = collect_characters(&sink.tokens);
    assert_eq!(text, "Conteúdo");
}

#[test]
fn test_mid_token_split_comment() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::new();

    input.insert_stream_content("<!-- parte 1 ");
    tokenizer.tokenize_chunk(&mut input, &mut sink);
    assert_eq!(tokenizer.state, TokenizerState::Comment);

    input.insert_stream_content("e parte 2 ");
    tokenizer.tokenize_chunk(&mut input, &mut sink);
    assert_eq!(tokenizer.state, TokenizerState::Comment);

    input.insert_stream_content("final -->");
    tokenizer.tokenize(&mut input, &mut sink);
    assert_eq!(tokenizer.state, TokenizerState::Data);

    let comment = sink
        .tokens
        .iter()
        .find_map(|t| match t {
            Token::Comment(c) => Some(c.as_str()),
            _ => None,
        })
        .expect("Comment deve ter sido emitido");

    assert_eq!(comment, " parte 1 e parte 2 final ");
}

#[test]
fn test_mid_token_split_doctype_dynamic_write() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::new();

    input.push_insertion_point();
    input.insert_stream_content("<!DOC");
    input.insert_stream_content("TYPE ht");
    input.insert_stream_content("ml>");
    input.pop_insertion_point();

    tokenizer.tokenize(&mut input, &mut sink);

    let doctype = sink
        .tokens
        .iter()
        .find_map(|t| match t {
            Token::Doctype(dt) => Some(dt),
            _ => None,
        })
        .expect("Doctype deve ter sido emitido");

    assert_eq!(doctype.name.as_deref(), Some("html"));
    assert!(!doctype.force_quirks);
}

#[test]
fn test_mid_token_split_doctype_chunked_feed() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::new();

    input.append_preprocessed_chunk("<!DOC");
    tokenizer.tokenize_chunk(&mut input, &mut sink);

    input.append_preprocessed_chunk("TYPE ht");
    tokenizer.tokenize_chunk(&mut input, &mut sink);

    input.append_preprocessed_chunk("ml>");
    tokenizer.tokenize(&mut input, &mut sink);

    let doctype = sink
        .tokens
        .iter()
        .find_map(|t| match t {
            Token::Doctype(dt) => Some(dt),
            _ => None,
        })
        .expect("Doctype deve ter sido emitido");

    assert_eq!(doctype.name.as_deref(), Some("html"));
    assert!(!doctype.force_quirks);
}

#[test]
fn test_mid_token_split_cdata_dynamic_write() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::new();

    input.push_insertion_point();
    input.insert_stream_content("<![CD");
    input.insert_stream_content("ATA[dado_");
    input.insert_stream_content("bruto]]>");
    input.pop_insertion_point();

    tokenizer.tokenize(&mut input, &mut sink);

    let text = collect_characters(&sink.tokens);
    assert_eq!(text, "dado_bruto");
}

#[test]
fn test_mid_token_split_cdata_chunked_feed() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::new();

    input.append_preprocessed_chunk("<![CD");
    tokenizer.tokenize_chunk(&mut input, &mut sink);

    input.append_preprocessed_chunk("ATA[dado_");
    tokenizer.tokenize_chunk(&mut input, &mut sink);

    input.append_preprocessed_chunk("bruto]]>");
    tokenizer.tokenize(&mut input, &mut sink);

    let text = collect_characters(&sink.tokens);
    assert_eq!(text, "dado_bruto");
}

#[test]
fn test_token_sink_reentrant_action_insertion() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = ScriptReentrantSink::new(vec![("script", "<div class=\"injected\">From Script</div>")]);
    let mut input = SegmentedString::from("<script></script><p>After</p>");

    tokenizer.tokenize(&mut input, &mut sink);

    let tags: Vec<String> = sink
        .tokens
        .iter()
        .filter_map(|t| match t {
            Token::StartTag(tag) => Some(tag.name.as_str().to_string()),
            _ => None,
        })
        .collect();

    // Ao processar o StartTag "script", o sink injeta `<div class="injected">...</div>`
    // que é consumido imediatamente antes de `<p>After</p>`.
    assert_eq!(tags, vec!["script", "div", "p"]);
}

#[test]
fn test_whatwg_character_preprocessing_on_dynamic_insertion() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::new();

    input.push_insertion_point();
    input.insert_stream_content("Linha 1\r\nLinha 2\rLinha 3\0Fim");
    input.pop_insertion_point();

    tokenizer.tokenize(&mut input, &mut sink);

    let text = collect_characters(&sink.tokens);
    assert_eq!(text, "Linha 1\nLinha 2\nLinha 3\u{FFFD}Fim");
}

#[test]
fn test_simd_hardware_fast_path_with_dynamic_segments() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::new();

    // Injeta blocos grandes de texto ASCII puro (que ativam memchr3)
    let chunk1 = "A".repeat(4096);
    let chunk2 = "B".repeat(4096);

    input.insert_stream_content(&chunk1);
    tokenizer.tokenize_chunk(&mut input, &mut sink);

    input.insert_stream_content("<div>Middle</div>");
    tokenizer.tokenize_chunk(&mut input, &mut sink);

    input.insert_stream_content(&chunk2);
    tokenizer.tokenize(&mut input, &mut sink);

    let text = collect_characters(&sink.tokens);
    assert!(text.starts_with(&chunk1));
    assert!(text.contains("Middle"));
    assert!(text.ends_with(&chunk2));
}

#[test]
fn test_tokenizer_self_contained_pump_and_feed() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();

    tokenizer.feed("<h1>Titulo</h1>");
    tokenizer.push_insertion_point();
    tokenizer.insert_stream_content("<p>Paragrafo Injetado</p>");
    tokenizer.pop_insertion_point();

    tokenizer.pump(&mut sink, true);

    let start_tags: Vec<String> = sink
        .tokens
        .iter()
        .filter_map(|t| match t {
            Token::StartTag(tag) => Some(tag.name.as_str().to_string()),
            _ => None,
        })
        .collect();

    assert_eq!(start_tags, vec!["p", "h1"]);
    assert_eq!(sink.tokens.last(), Some(&Token::Eof));
}

#[test]
fn test_nested_reentrancy_stress_depth_10() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::from("<base_tail />");

    // Empilha 10 níveis de pontos de inserção recursivos
    for i in 1..=10 {
        input.push_insertion_point();
        input.insert_stream_content(&format!("<tag_{} />", i));
    }

    // Desempilha todos
    for _ in 1..=10 {
        input.pop_insertion_point();
    }

    tokenizer.tokenize(&mut input, &mut sink);

    let start_tags: Vec<String> = sink
        .tokens
        .iter()
        .filter_map(|t| match t {
            Token::StartTag(tag) => Some(tag.name.as_str().to_string()),
            _ => None,
        })
        .collect();

    // Ordem LIFO: tag_10 até tag_1, seguida por base_tail
    let expected: Vec<String> = (1..=10)
        .rev()
        .map(|i| format!("tag_{}", i))
        .chain(std::iter::once("base_tail".to_string()))
        .collect();

    assert_eq!(start_tags, expected);
}

#[test]
fn test_attribute_value_unquoted_split_writes() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::new();

    input.insert_stream_content("<a href=https://albedo-browser.");
    tokenizer.tokenize_chunk(&mut input, &mut sink);

    input.insert_stream_content("org/docs/api>Link</a>");
    tokenizer.tokenize(&mut input, &mut sink);

    let link_tag = sink
        .tokens
        .iter()
        .find_map(|t| match t {
            Token::StartTag(tag) if tag.name.as_str() == "a" => Some(tag),
            _ => None,
        })
        .expect("StartTag a esperada");

    assert_eq!(link_tag.attributes.len(), 1);
    assert_eq!(link_tag.attributes[0].name.as_str(), "href");
    assert_eq!(
        link_tag.attributes[0].value.as_str(),
        "https://albedo-browser.org/docs/api"
    );
}

#[test]
fn test_rawtext_and_rcdata_streaming_split() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::new();

    // RAWTEXT: <style>
    input.insert_stream_content("<style>body { background: ");
    tokenizer.tokenize_chunk(&mut input, &mut sink);

    input.insert_stream_content("#fff; }</style><title>Meu ");
    tokenizer.tokenize_chunk(&mut input, &mut sink);

    // RCDATA: <title>
    input.insert_stream_content("Titulo &amp; Mais</title>");
    tokenizer.tokenize(&mut input, &mut sink);

    let text = collect_characters(&sink.tokens);
    assert!(text.contains("body { background: #fff; }"));
    assert!(text.contains("Meu Titulo & Mais"));
}

#[test]
fn test_multibyte_utf8_split_across_dynamic_writes() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::new();

    input.push_insertion_point();
    input.insert_stream_content("<p>Olá 🦀 Al");
    input.insert_stream_content("bedo! Coração &gt; ");
    input.insert_stream_content("Emoção 🔥</p>");
    input.pop_insertion_point();

    tokenizer.tokenize(&mut input, &mut sink);

    let text = collect_characters(&sink.tokens);
    assert_eq!(text, "Olá 🦀 Albedo! Coração > Emoção 🔥");
}

#[test]
fn test_entity_split_across_dynamic_writes() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::new();

    input.push_insertion_point();
    input.insert_stream_content("<span title='&a");
    input.insert_stream_content("mp;'>&c");
    input.insert_stream_content("opy; 2026</span>");
    input.pop_insertion_point();

    tokenizer.tokenize(&mut input, &mut sink);

    let span = sink
        .tokens
        .iter()
        .find_map(|t| match t {
            Token::StartTag(tag) if tag.name.as_str() == "span" => Some(tag),
            _ => None,
        })
        .expect("StartTag span esperada");

    assert_eq!(span.attributes[0].value.as_str(), "&");

    let text = collect_characters(&sink.tokens);
    assert_eq!(text, "© 2026");
}

#[test]
fn test_comment_with_multiple_dashes_split_across_writes() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::new();

    input.push_insertion_point();
    input.insert_stream_content("<!-- ---");
    input.insert_stream_content("- internal dash --");
    input.insert_stream_content("-->");
    input.pop_insertion_point();

    tokenizer.tokenize(&mut input, &mut sink);

    let comment = sink
        .tokens
        .iter()
        .find_map(|t| match t {
            Token::Comment(c) => Some(c.as_str()),
            _ => None,
        })
        .expect("Comment esperado");

    assert!(comment.contains("internal dash"));
}

#[test]
fn test_bogus_comment_split_across_writes() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::new();

    input.push_insertion_point();
    input.insert_stream_content("<?xml-stylesheet ");
    input.insert_stream_content("type=\"text/css\" ");
    input.insert_stream_content("href=\"style.css\"?>");
    input.pop_insertion_point();

    tokenizer.tokenize(&mut input, &mut sink);

    let comment = sink
        .tokens
        .iter()
        .find_map(|t| match t {
            Token::Comment(c) => Some(c.as_str()),
            _ => None,
        })
        .expect("Bogus comment esperado");

    assert!(comment.contains("xml-stylesheet"));
    assert!(comment.contains("href=\"style.css\""));
}

#[test]
fn test_reentrancy_depth_50_stress() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::from("<tail />");

    for i in 1..=50 {
        input.push_insertion_point();
        input.insert_stream_content(&format!("<level_{} />", i));
    }

    for _ in 1..=50 {
        input.pop_insertion_point();
    }

    tokenizer.tokenize(&mut input, &mut sink);

    let tags: Vec<String> = sink
        .tokens
        .iter()
        .filter_map(|t| match t {
            Token::StartTag(tag) => Some(tag.name.as_str().to_string()),
            _ => None,
        })
        .collect();

    assert_eq!(tags.len(), 51);
    assert_eq!(tags[0], "level_50");
    assert_eq!(tags[49], "level_1");
    assert_eq!(tags[50], "tail");
}

#[test]
fn test_empty_writes_and_whitespace_preservation() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestRecorderSink::default();
    let mut input = SegmentedString::new();

    input.push_insertion_point();
    input.insert_stream_content("");
    input.insert_stream_content("   <div>   \n\t  ");
    input.insert_stream_content("");
    input.insert_stream_content("Hello");
    input.insert_stream_content("   </div>   ");
    input.insert_stream_content("");
    input.pop_insertion_point();

    tokenizer.tokenize(&mut input, &mut sink);

    let text = collect_characters(&sink.tokens);
    assert!(text.contains("Hello"));
    assert!(text.starts_with("   "));
}

