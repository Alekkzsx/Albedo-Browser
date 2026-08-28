use ace_dom::{tokenize_to_compact_tokens, CompactHTMLToken};

#[test]
fn test_background_compact_token_streaming() {
    let html = r#"
        <!DOCTYPE html>
        <html>
            <head>
                <meta charset="utf-8">
                <!-- Comentário de teste -->
                <title>Albedo Streaming Tokenizer</title>
            </head>
            <body>
                <div class="content">
                    <h1>Olá, Mundo!</h1>
                    <img src="banner.png" alt="Banner" />
                </div>
            </body>
        </html>
    "#;

    let tokens = tokenize_to_compact_tokens(html);
    assert!(!tokens.is_empty());

    // Verifica presença de DOCTYPE
    let has_doctype = tokens.iter().any(|t| matches!(t, CompactHTMLToken::Doctype { .. }));
    assert!(has_doctype);

    // Verifica presença de comentários
    let has_comment = tokens.iter().any(|t| matches!(t, CompactHTMLToken::Comment(c) if c.contains("Comentário")));
    assert!(has_comment);

    // Verifica presença da tag <div class="content">
    let has_div = tokens.iter().any(|t| {
        if let CompactHTMLToken::StartTag { name, attributes, .. } = t {
            name == "div" && attributes.iter().any(|a| a.name == "class" && a.value == "content")
        } else {
            false
        }
    });
    assert!(has_div);

    // Verifica tag auto-fechável <img>
    let has_img = tokens.iter().any(|t| {
        if let CompactHTMLToken::StartTag { name, self_closing, attributes } = t {
            name == "img" && *self_closing && attributes.iter().any(|a| a.name == "src" && a.value == "banner.png")
        } else {
            false
        }
    });
    assert!(has_img);

    // Verifica fechamento EOF
    assert_eq!(tokens.last(), Some(&CompactHTMLToken::Eof));
}
