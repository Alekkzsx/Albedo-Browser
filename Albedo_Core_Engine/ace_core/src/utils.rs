//! # Utilitários Globais do Albedo Core Engine
//!
//! Funções transversais para hashing ultra-rápido (`FxHasher`), formatação de dumps binários e sanitização de strings.

use rustc_hash::FxHasher;
use std::hash::{Hash, Hasher};

/// Calcula o hash FxHash de 64 bits em altíssima velocidade para estruturas internas (DOM/CSS).
#[inline]
pub fn fast_hash<T: Hash + ?Sized>(val: &T) -> u64 {
    let mut hasher = FxHasher::default();
    val.hash(&mut hasher);
    hasher.finish()
}

/// Gera uma representação em hexadecimal formatada de um buffer de bytes (útil para IPC, pacotes de rede e fontes).
pub fn hexdump(data: &[u8], max_bytes: usize) -> String {
    let limit = data.len().min(max_bytes);
    let mut out = String::with_capacity(limit * 3 + 32);

    for (i, byte) in data[..limit].iter().enumerate() {
        if i > 0 && i % 16 == 0 {
            out.push('\n');
        } else if i > 0 && i % 8 == 0 {
            out.push(' ');
        }
        out.push_str(&format!("{:02X} ", byte));
    }

    if data.len() > max_bytes {
        out.push_str(&format!("... ({} bytes omitidos)", data.len() - max_bytes));
    }

    out
}

/// Remove caracteres de controle não imprimíveis de uma string ASCII/UTF-8.
pub fn sanitize_ascii(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_control() && c != '\n' && c != '\t' && c != '\r' {
                ' '
            } else {
                c
            }
        })
        .collect()
}
