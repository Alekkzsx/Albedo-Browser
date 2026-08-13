// ============================================================================
// Albedo Core Engine (ACE)
// File: string.rs
// Description: AceString - Implementação Híbrida de Strings da Web.
//              Suporta Small String Optimization (SSO) e Compact Strings (Latin1/UTF-16).
//              Zero dependências, focado em alta performance e economia de memória.
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::fmt;

/// O limite máximo de bytes para a otimização de string pequena (SSO).
/// Em uma arquitetura 64-bits, um enum `AceString` pode usar 24 bytes (igual a um `String` nativo).
/// 1 byte para a tag/tamanho, restando 23 bytes para dados embutidos (inline).
const INLINE_CAPACITY: usize = 23;

/// Representação de String otimizada para o padrão da Web (DOM / ECMAScript).
///
/// A especificação da web e do Javascript assume indexação UTF-16. 
/// Usar o `String` padrão do Rust (UTF-8) causa gargalos $O(N)$ na busca por índices de caracteres
/// e desperdiça memória para textos ASCII puros.
///
/// `AceString` resolve isso usando armazenamento híbrido:
/// - Textos curtos (< 24 chars) vão para a Stack (Inline/SSO) sem alocação no Heap.
/// - Textos ASCII/Latin1 puros usam 1 byte por caractere no Heap.
/// - Textos complexos (CJK, Emojis) promovem automaticamente para UTF-16 no Heap.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum AceString {
    /// Small String Optimization. O primeiro campo é o tamanho, o array contém os dados Latin1.
    Inline { len: u8, data: [u8; INLINE_CAPACITY] },
    /// Armazenamento Latin-1 no Heap (1 byte por caractere).
    Latin1(Vec<u8>),
    /// Armazenamento UTF-16 no Heap (2 bytes por caractere).
    Utf16(Vec<u16>),
}

impl AceString {
    /// Cria uma `AceString` vazia alocada no Stack.
    #[inline]
    pub fn new() -> Self {
        AceString::Inline {
            len: 0,
            data: [0; INLINE_CAPACITY],
        }
    }

    /// Tenta criar uma `AceString` a partir de uma string nativa do Rust.
    /// Decide automaticamente qual o formato mais eficiente de armazenamento.
    pub fn from_str(s: &str) -> Self {
        let chars_count = s.chars().count();
        let bytes_len = s.len();

        // É puro ASCII e cabe no limite SSO?
        if bytes_len == chars_count && bytes_len <= INLINE_CAPACITY {
            let mut data = [0; INLINE_CAPACITY];
            data[..bytes_len].copy_from_slice(s.as_bytes());
            return AceString::Inline {
                len: bytes_len as u8,
                data,
            };
        }

        // Verifica se podemos comprimir em Latin1
        let mut is_latin1 = true;
        for c in s.chars() {
            if (c as u32) > 0xFF {
                is_latin1 = false;
                break;
            }
        }

        if is_latin1 {
            let mut latin1_data = Vec::with_capacity(chars_count);
            for c in s.chars() {
                latin1_data.push(c as u8);
            }
            AceString::Latin1(latin1_data)
        } else {
            // Promove para UTF-16
            let utf16_data: Vec<u16> = s.encode_utf16().collect();
            AceString::Utf16(utf16_data)
        }
    }

    /// Retorna o comprimento da string em *caracteres* (unidades de código UTF-16),
    /// comportando-se exatamente como o `length` do JavaScript.
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            AceString::Inline { len, .. } => *len as usize,
            AceString::Latin1(vec) => vec.len(),
            AceString::Utf16(vec) => vec.len(),
        }
    }

    /// Verifica se a string está vazia.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Converte a `AceString` de volta para uma `String` nativa do Rust (UTF-8).
    /// Essa operação pode ter custo de conversão.
    pub fn to_string(&self) -> String {
        match self {
            AceString::Inline { len, data } => {
                // Inline Strings só contêm Latin1 válido por construção na nossa engine.
                String::from_utf8_lossy(&data[..(*len as usize)]).into_owned()
            }
            AceString::Latin1(vec) => {
                // Converte Latin1 -> UTF-8
                let mut s = String::with_capacity(vec.len());
                for &byte in vec {
                    s.push(byte as char);
                }
                s
            }
            AceString::Utf16(vec) => {
                String::from_utf16_lossy(vec)
            }
        }
    }
}

impl Default for AceString {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for AceString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AceString({:?})", self.to_string())
    }
}

impl fmt::Display for AceString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

// Final do arquivo. Testes movidos para tests/string_tests.rs
