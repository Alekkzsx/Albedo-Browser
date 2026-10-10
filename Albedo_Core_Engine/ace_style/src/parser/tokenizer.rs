//! # Tokenizer CSS Syntax Level 3 com Fast-Path SIMD
//!
//! FSM de streaming com SIMD (`memchr`) para varredura de identificadores, aspas e comentários.

use super::token::Token;
use memchr::memchr;
use smol_str::SmolStr;

/// Tokenizer de CSS que produz um fluxo de tokens conformes com CSS Syntax Level 3.
pub struct Tokenizer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    /// Cria um novo tokenizer para o texto CSS fornecido.
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    /// Retorna o restante da string não consumida.
    #[inline]
    fn remaining(&self) -> &'a str {
        if self.pos < self.input.len() {
            &self.input[self.pos..]
        } else {
            ""
        }
    }

    /// Retorna o próximo byte/caractere sem avançar a posição.
    #[inline]
    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    /// Retorna os próximos dois caracteres sem avançar.
    #[inline]
    #[allow(dead_code)]
    fn peek2(&self) -> (Option<char>, Option<char>) {
        let mut chars = self.remaining().chars();
        (chars.next(), chars.next())
    }

    /// Avança a posição por `n` bytes.
    #[inline]
    fn advance_bytes(&mut self, n: usize) {
        self.pos = (self.pos + n).min(self.input.len());
    }

    /// Avança 1 caractere UTF-8 e retorna-o.
    fn next_char(&mut self) -> Option<char> {
        let ch = self.remaining().chars().next()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    /// Tokeniza todo o input em um vetor de tokens.
    pub fn tokenize_all(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let is_eof = tok.is_eof();
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }

    /// Extrai o próximo token conforme as regras do CSS Syntax L3 §4.3.
    pub fn next_token(&mut self) -> Token {
        let rem = self.remaining();
        if rem.is_empty() {
            return Token::Eof;
        }

        let first = rem.chars().next().unwrap();

        // 1. Espaços em branco
        if first.is_ascii_whitespace() {
            return self.consume_whitespace();
        }

        // 2. Comentários /* ... */
        if rem.starts_with("/*") {
            return self.consume_comment();
        }

        // 3. Delimitadores HTML <!-- e -->
        if rem.starts_with("<!--") {
            self.advance_bytes(4);
            return Token::CDO;
        }
        if rem.starts_with("-->") {
            self.advance_bytes(3);
            return Token::CDC;
        }

        // 4. Strings com aspas duplas ou simples
        if first == '"' || first == '\'' {
            return self.consume_string(first);
        }

        // 5. Hash (#)
        if first == '#' {
            self.advance_bytes(1);
            let rem2 = self.remaining();
            if self.starts_identifier(rem2) {
                let ident = self.consume_name();
                return Token::Hash {
                    value: SmolStr::new(ident),
                    is_id: true,
                };
            } else {
                let name = self.consume_name();
                return Token::Hash {
                    value: SmolStr::new(name),
                    is_id: false,
                };
            }
        }

        // 6. At-Keyword (@)
        if first == '@' {
            self.advance_bytes(1);
            let name = self.consume_name();
            return Token::AtKeyword(SmolStr::new(name));
        }

        // 7. Pontuação simples
        match first {
            ':' => {
                self.advance_bytes(1);
                return Token::Colon;
            }
            ';' => {
                self.advance_bytes(1);
                return Token::Semicolon;
            }
            ',' => {
                self.advance_bytes(1);
                return Token::Comma;
            }
            '(' => {
                self.advance_bytes(1);
                return Token::OpenParen;
            }
            ')' => {
                self.advance_bytes(1);
                return Token::CloseParen;
            }
            '[' => {
                self.advance_bytes(1);
                return Token::OpenSquare;
            }
            ']' => {
                self.advance_bytes(1);
                return Token::CloseSquare;
            }
            '{' => {
                self.advance_bytes(1);
                return Token::OpenCurly;
            }
            '}' => {
                self.advance_bytes(1);
                return Token::CloseCurly;
            }
            _ => {}
        }

        // 8. Números, dimensões e porcentagens
        if self.starts_number(rem) {
            return self.consume_numeric();
        }

        // 9. Identificadores ou Funções
        if self.starts_identifier(rem) {
            return self.consume_ident_or_function();
        }

        // 10. Delimitador geral de 1 caractere
        let ch = self.next_char().unwrap();
        // Null byte sanitization conforme CSS Syntax L3
        if ch == '\0' {
            Token::Delim('\u{FFFD}')
        } else {
            Token::Delim(ch)
        }
    }

    fn consume_whitespace(&mut self) -> Token {
        while let Some(ch) = self.peek() {
            if ch.is_ascii_whitespace() {
                self.next_char();
            } else {
                break;
            }
        }
        Token::Whitespace
    }

    fn consume_comment(&mut self) -> Token {
        self.advance_bytes(2); // Pula '/*'
        let rem_bytes = self.remaining().as_bytes();

        // Busca rápida SIMD pelo término '*/'
        let mut idx = 0;
        while idx < rem_bytes.len() {
            if let Some(pos) = memchr(b'*', &rem_bytes[idx..]) {
                let abs_pos = idx + pos;
                if abs_pos + 1 < rem_bytes.len() && rem_bytes[abs_pos + 1] == b'/' {
                    let comment_content = &self.remaining()[..abs_pos];
                    self.advance_bytes(abs_pos + 2);
                    return Token::Comment(SmolStr::new(comment_content));
                }
                idx = abs_pos + 1;
            } else {
                break;
            }
        }

        // Comentário não fechado até o fim do arquivo (EOF)
        let comment_content = self.remaining();
        self.advance_bytes(comment_content.len());
        Token::Comment(SmolStr::new(comment_content))
    }

    fn consume_string(&mut self, quote: char) -> Token {
        self.next_char(); // Pula a aspa de abertura
        let mut result = String::new();

        while let Some(ch) = self.next_char() {
            if ch == quote {
                return Token::String(SmolStr::new(result));
            } else if ch == '\n' || ch == '\r' {
                // Quebra de linha não escapada dentro de string é erro
                return Token::BadString;
            } else if ch == '\\' {
                if let Some(escaped) = self.consume_escape() {
                    result.push(escaped);
                }
            } else if ch == '\0' {
                result.push('\u{FFFD}');
            } else {
                result.push(ch);
            }
        }

        // String não fechada até o fim do arquivo é tratada como Token::String conforme W3C §4.3.5
        Token::String(SmolStr::new(result))
    }

    fn consume_escape(&mut self) -> Option<char> {
        let ch = self.next_char()?;
        if ch.is_ascii_hexdigit() {
            let mut hex = String::new();
            hex.push(ch);
            for _ in 0..5 {
                if let Some(next_c) = self.peek() {
                    if next_c.is_ascii_hexdigit() {
                        hex.push(self.next_char().unwrap());
                    } else {
                        break;
                    }
                }
            }
            if let Some(next_c) = self.peek() {
                if next_c.is_ascii_whitespace() {
                    self.next_char();
                }
            }
            if let Ok(code) = u32::from_str_radix(&hex, 16) {
                if code == 0 || (0xD800..=0xDFFF).contains(&code) || code > 0x10FFFF {
                    Some('\u{FFFD}')
                } else {
                    char::from_u32(code)
                }
            } else {
                Some('\u{FFFD}')
            }
        } else if ch == '\0' {
            Some('\u{FFFD}')
        } else {
            Some(ch)
        }
    }

    fn starts_identifier(&self, s: &str) -> bool {
        let mut chars = s.chars();
        match chars.next() {
            Some(c) if c == '-' => match chars.next() {
                Some(c2) => c2 == '-' || c2 == '_' || c2.is_alphabetic() || c2 == '\\' || c2 as u32 >= 0x80,
                None => false,
            },
            Some(c) => c == '_' || c.is_alphabetic() || c == '\\' || c as u32 >= 0x80,
            None => false,
        }
    }

    fn starts_number(&self, s: &str) -> bool {
        let mut chars = s.chars();
        match chars.next() {
            Some('+' | '-') => match chars.next() {
                Some(c) => c.is_ascii_digit() || (c == '.' && chars.next().is_some_and(|c3| c3.is_ascii_digit())),
                None => false,
            },
            Some('.') => chars.next().is_some_and(|c| c.is_ascii_digit()),
            Some(c) => c.is_ascii_digit(),
            None => false,
        }
    }

    fn consume_name(&mut self) -> String {
        let mut name = String::new();
        while let Some(ch) = self.peek() {
            if ch == '_' || ch.is_alphanumeric() || ch == '-' || ch as u32 >= 0x80 {
                name.push(self.next_char().unwrap());
            } else if ch == '\\' {
                self.next_char();
                if let Some(escaped) = self.consume_escape() {
                    name.push(escaped);
                }
            } else {
                break;
            }
        }
        name
    }

    fn consume_numeric(&mut self) -> Token {
        let mut num_str = String::new();
        let mut has_sign = false;
        let mut is_integer = true;

        if let Some(ch) = self.peek() {
            if ch == '+' || ch == '-' {
                has_sign = true;
                num_str.push(self.next_char().unwrap());
            }
        }

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                num_str.push(self.next_char().unwrap());
            } else {
                break;
            }
        }

        if let Some('.') = self.peek() {
            let mut chars = self.remaining().chars();
            chars.next(); // '.'
            if chars.next().is_some_and(|c| c.is_ascii_digit()) {
                is_integer = false;
                num_str.push(self.next_char().unwrap()); // '.'
                while let Some(ch) = self.peek() {
                    if ch.is_ascii_digit() {
                        num_str.push(self.next_char().unwrap());
                    } else {
                        break;
                    }
                }
            }
        }

        // Exponencial e/E
        if let Some('e' | 'E') = self.peek() {
            let mut chars = self.remaining().chars();
            chars.next();
            let mut valid_exp = false;
            match chars.next() {
                Some('+' | '-') => {
                    if chars.next().is_some_and(|c| c.is_ascii_digit()) {
                        valid_exp = true;
                    }
                }
                Some(c) if c.is_ascii_digit() => {
                    valid_exp = true;
                }
                _ => {}
            }
            if valid_exp {
                is_integer = false;
                num_str.push(self.next_char().unwrap()); // e/E
                if let Some(c) = self.peek() {
                    if c == '+' || c == '-' {
                        num_str.push(self.next_char().unwrap());
                    }
                }
                while let Some(ch) = self.peek() {
                    if ch.is_ascii_digit() {
                        num_str.push(self.next_char().unwrap());
                    } else {
                        break;
                    }
                }
            }
        }

        let value = num_str.parse::<f64>().unwrap_or(0.0);

        // Verifica se é porcentagem
        if let Some('%') = self.peek() {
            self.next_char();
            return Token::Percentage(value);
        }

        // Verifica se é dimensão com unidade
        if self.starts_identifier(self.remaining()) {
            let unit = self.consume_name();
            return Token::Dimension {
                value,
                unit: SmolStr::new(unit),
            };
        }

        Token::Number {
            value,
            has_sign,
            is_integer,
        }
    }

    fn consume_ident_or_function(&mut self) -> Token {
        let name = self.consume_name();

        // Se for seguido por '(', é função
        if let Some('(') = self.peek() {
            self.next_char();
            if name.eq_ignore_ascii_case("url") {
                return self.consume_url();
            }
            return Token::Function(SmolStr::new(name));
        }

        Token::Ident(SmolStr::new(name))
    }

    fn consume_url(&mut self) -> Token {
        // Pula espaços em branco após url(
        while let Some(ch) = self.peek() {
            if ch.is_ascii_whitespace() {
                self.next_char();
            } else {
                break;
            }
        }

        if let Some(quote) = self.peek() {
            if quote == '"' || quote == '\'' {
                // URL delimitada por string
                let string_tok = self.consume_string(quote);
                // Consome espaços em branco até fechar parêntese
                while let Some(ch) = self.peek() {
                    if ch.is_ascii_whitespace() {
                        self.next_char();
                    } else {
                        break;
                    }
                }
                if let Some(')') = self.peek() {
                    self.next_char();
                }
                return match string_tok {
                    Token::String(url_str) => Token::Url(url_str),
                    _ => Token::BadUrl,
                };
            }
        }

        let mut url_val = String::new();
        while let Some(ch) = self.next_char() {
            if ch == ')' {
                return Token::Url(SmolStr::new(url_val.trim()));
            } else if ch.is_ascii_whitespace() {
                while let Some(next) = self.peek() {
                    if next.is_ascii_whitespace() {
                        self.next_char();
                    } else {
                        break;
                    }
                }
                if let Some(')') = self.peek() {
                    self.next_char();
                    return Token::Url(SmolStr::new(url_val.trim()));
                } else {
                    return Token::BadUrl;
                }
            } else if ch == '"' || ch == '\'' || ch == '(' || ch < '\x20' || ch == '\x7F' {
                return Token::BadUrl;
            } else if ch == '\\' {
                if let Some(escaped) = self.consume_escape() {
                    url_val.push(escaped);
                }
            } else {
                url_val.push(ch);
            }
        }

        Token::Url(SmolStr::new(url_val.trim()))
    }
}
