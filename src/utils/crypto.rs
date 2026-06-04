use rand::RngCore;
use sha2::{Digest, Sha256};

/// Preenche o buffer com bytes aleatórios gerados de forma criptograficamente segura.
pub fn get_random_bytes(buf: &mut [u8]) {
    rand::thread_rng().fill_bytes(buf);
}

/// Gera o hash SHA-256 de um slice de bytes.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}
