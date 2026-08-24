//! # StringBuilder e Concatenação Zero-Allocation (Chromium base::StrCat & WebKit WTF::StringBuilder Pattern)
//!
//! Em pipelines de formatação de CSS, serialização de HTML e construção de URLs,
//! encadear operadores `+` ou chamadas repetidas a `format!` gera dezenas de alocações intermediárias
//! temporárias no heap, sobrecarregando o alocador do sistema.
//!
//! Este módulo provê `StringBuilder` e a macro `str_cat!` que:
//! 1. Calculam previamente o comprimento exato agregado de todos os fragmentos em uma única passagem.
//! 2. Realizam **uma única alocação de memória** no heap.
//! 3. Copiam todos os bytes de forma contígua e instantânea.

use std::fmt;

/// Construtor de strings de alta velocidade com pré-alocação estrita de capacidade.
#[derive(Default, Clone)]
pub struct StringBuilder {
    buffer: String,
}

impl StringBuilder {
    /// Cria um novo `StringBuilder` vazio.
    #[inline]
    pub const fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Cria um novo `StringBuilder` com capacidade pré-alocada em bytes.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: String::with_capacity(capacity),
        }
    }

    /// Concatena uma lista de fatias de string (`&str`) em uma única `String` com exatamente UMA alocação.
    pub fn concat(pieces: &[&str]) -> String {
        let total_len: usize = pieces.iter().map(|p| p.len()).sum();
        let mut result = String::with_capacity(total_len);
        for piece in pieces {
            result.push_str(piece);
        }
        result
    }

    /// Adiciona uma fatia de texto ao buffer.
    #[inline]
    pub fn append(&mut self, text: &str) -> &mut Self {
        self.buffer.push_str(text);
        self
    }

    /// Adiciona um caractere Unicode ao buffer.
    #[inline]
    pub fn append_char(&mut self, ch: char) -> &mut Self {
        self.buffer.push(ch);
        self
    }

    /// Adiciona um número inteiro formatado sem alocações no heap.
    #[inline]
    pub fn append_u64(&mut self, val: u64) -> &mut Self {
        use std::io::Write;
        let mut buf = [0u8; 32];
        let mut cursor = std::io::Cursor::new(&mut buf[..]);
        let _ = write!(cursor, "{}", val);
        let len = cursor.position() as usize;
        let s = unsafe { std::str::from_utf8_unchecked(&buf[..len]) };
        self.buffer.push_str(s);
        self
    }

    /// Retorna o comprimento atual do buffer em bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Retorna `true` se o buffer estiver vazio.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Retorna a capacidade total alocada em bytes.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Limpa o conteúdo do buffer, preservando a capacidade de memória para reúso.
    #[inline]
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Retorna uma referência `&str` ao texto acumulado.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.buffer
    }

    /// Consome o builder e retorna a `String` resultante final.
    #[inline]
    pub fn finish(self) -> String {
        self.buffer
    }
}

impl fmt::Display for StringBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.buffer)
    }
}

impl fmt::Debug for StringBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("StringBuilder").field(&self.buffer).finish()
    }
}

/// Macro que executa concatenação de múltiplas strings com uma única alocação de heap (Chromium `base::StrCat`).
///
/// # Exemplo
/// ```rust
/// use ace_core::str_cat;
/// let url = str_cat!("https://", "example.com", ":", "8080", "/index.html");
/// assert_eq!(url, "https://example.com:8080/index.html");
/// ```
#[macro_export]
macro_rules! str_cat {
    ($($piece:expr),* $(,)?) => {
        $crate::text::string_builder::StringBuilder::concat(&[
            $(::core::convert::AsRef::<str>::as_ref(&$piece)),*
        ])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_builder_append() {
        let mut builder = StringBuilder::with_capacity(32);
        builder.append("Hello");
        builder.append_char(' ');
        builder.append("World #");
        builder.append_u64(42);

        assert_eq!(builder.as_str(), "Hello World #42");
        assert_eq!(builder.len(), 15);

        let final_str = builder.finish();
        assert_eq!(final_str, "Hello World #42");
    }

    #[test]
    fn test_str_cat_macro() {
        let proto = "https://";
        let host = "albedo.dev";
        let path = "/engine";

        let result = str_cat!(proto, host, path, "?version=", "1");
        assert_eq!(result, "https://albedo.dev/engine?version=1");
    }
}
