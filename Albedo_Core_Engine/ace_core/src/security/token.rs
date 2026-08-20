//! # Tokens Criptográficos Inadivinháveis (Chromium UnguessableToken Pattern)
//!
//! Identificador seguro de 128 bits para isolamento estrito entre processos (IPC),
//! prevenção de *Origin Confusion Attacks* e autenticação de canais de comunicação.

use std::fmt;

/// Token de 128 bits criptograficamente inadivinhável para limites de segurança e IPC.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct UnguessableToken {
    high: u64,
    low: u64,
}

impl UnguessableToken {
    /// Cria um novo token único de 128 bits gerado a partir de CSPRNG do sistema operacional.
    pub fn new() -> Self {
        loop {
            let mut bytes = [0u8; 16];
            getrandom::getrandom(&mut bytes)
                .expect("Falha crítica ao obter entropia CSPRNG do sistema operacional");
            let high = u64::from_ne_bytes(bytes[0..8].try_into().unwrap());
            let low = u64::from_ne_bytes(bytes[8..16].try_into().unwrap());
            if high != 0 || low != 0 {
                return Self { high, low };
            }
        }
    }

    /// Cria um token a partir de suas partes brutas de 64 bits (ex: para deserialização IPC).
    #[inline]
    pub const fn from_raw(high: u64, low: u64) -> Self {
        Self { high, low }
    }

    /// Retorna a metade superior de 64 bits.
    #[inline]
    pub const fn high(&self) -> u64 {
        self.high
    }

    /// Retorna a metade inferior de 64 bits.
    #[inline]
    pub const fn low(&self) -> u64 {
        self.low
    }

    /// Retorna a representação formatada em hexadecimal minúsculo de 32 caracteres.
    pub fn to_hex(&self) -> String {
        format!("{:016x}{:016x}", self.high, self.low)
    }

    /// Retorna `true` se o token for nulo (todos os bits zerados).
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.high == 0 && self.low == 0
    }
}

impl fmt::Debug for UnguessableToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UnguessableToken({:016x}{:016x})", self.high, self.low)
    }
}

impl fmt::Display for UnguessableToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}{:016x}", self.high, self.low)
    }
}

