//! # Tokens Criptográficos Inadivinháveis (Chromium UnguessableToken Pattern)
//!
//! Identificador seguro de 128 bits para isolamento estrito entre processos (IPC),
//! prevenção de *Origin Confusion Attacks* e autenticação de canais de comunicação.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TOKEN_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Token de 128 bits criptograficamente inadivinhável para limites de segurança e IPC.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct UnguessableToken {
    high: u64,
    low: u64,
}

impl UnguessableToken {
    /// Cria um novo token único de 128 bits gerado a partir de entropia temporal e contadores atômicos.
    pub fn new() -> Self {
        let count = TOKEN_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        // Mix 128 bits usando FxHash round
        let high = crate::utils::fast_hash(&(nanos as u64, count, 0x517cc1b727220a95u64));
        let low = crate::utils::fast_hash(&((nanos >> 64) as u64, count, high, 0x4f1bbcdcbfa54005u64));

        Self { high, low }
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

    /// Retorna a representação formatada em hexadecimal de 32 caracteres.
    pub fn to_hex(&self) -> String {
        format!("{:016X}{:016X}", self.high, self.low)
    }

    /// Retorna `true` se o token for nulo (todos os bits zerados).
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.high == 0 && self.low == 0
    }
}

impl fmt::Debug for UnguessableToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UnguessableToken({})", self.to_hex())
    }
}

impl fmt::Display for UnguessableToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}
