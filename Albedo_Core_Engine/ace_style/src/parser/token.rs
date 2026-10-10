//! # Tokens Normativos do CSS (CSS Syntax Module Level 3 §4)
//!
//! Enumera os tipos de tokens gerados pelo Tokenizer conforme a especificação W3C.

use smol_str::SmolStr;

/// Tokens normativos gerados pelo Tokenizer de CSS Syntax Level 3.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Identificador CSS (`Ident`)
    Ident(SmolStr),
    /// Chamada de função CSS (`Function`)
    Function(SmolStr),
    /// At-Keyword (`@media`, `@layer`, etc.)
    AtKeyword(SmolStr),
    /// Hash (`#id` ou valor de cor hexadecimal)
    Hash {
        value: SmolStr,
        is_id: bool,
    },
    /// String delimitada por aspas simples ou duplas
    String(SmolStr),
    /// Literal numérico
    Number {
        value: f64,
        has_sign: bool,
        is_integer: bool,
    },
    /// Porcentagem (`50%`)
    Percentage(f64),
    /// Dimensão com unidade (`12px`, `2em`)
    Dimension {
        value: f64,
        unit: SmolStr,
    },
    /// Espaço em branco sequencial
    Whitespace,
    /// Comentário CSS `/* ... */`
    Comment(SmolStr),
    /// Delimitador de abertura de comentário HTML (`<!--`)
    CDO,
    /// Delimitador de fechamento de comentário HTML (`-->`)
    CDC,
    /// Dois-pontos (`:`)
    Colon,
    /// Ponto e vírgula (`;`)
    Semicolon,
    /// Vírgula (`,`)
    Comma,
    /// Abre colchetes (`[`)
    OpenSquare,
    /// Fecha colchetes (`]`)
    CloseSquare,
    /// Abre parênteses (`(`)
    OpenParen,
    /// Fecha parênteses (`)`)
    CloseParen,
    /// Abre chaves (`{`)
    OpenCurly,
    /// Fecha chaves (`}`)
    CloseCurly,
    /// URL tokenizada `url(...)`
    Url(SmolStr),
    /// Caractere delimitador pontual não categorizado
    Delim(char),
    /// String malformada / não terminada com quebra de linha
    BadString,
    /// URL malformada
    BadUrl,
    /// Fim do arquivo / stream de entrada
    Eof,
}

impl Token {
    /// Retorna `true` se o token for espaço em branco.
    #[inline]
    pub fn is_whitespace(&self) -> bool {
        matches!(self, Self::Whitespace)
    }

    /// Retorna `true` se o token for fim de arquivo.
    #[inline]
    pub fn is_eof(&self) -> bool {
        matches!(self, Self::Eof)
    }
}
