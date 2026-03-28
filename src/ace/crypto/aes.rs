use std::sync::LazyLock;

static SBOX: LazyLock<[u8; 256]> = LazyLock::new(build_sbox);
static INV_SBOX: LazyLock<[u8; 256]> = LazyLock::new(build_inv_sbox);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AesVariant {
    Aes128,
    Aes256,
}

#[derive(Clone)]
pub struct AesCipher {
    round_keys: Vec<[u8; 16]>,
    nr: usize,
}

impl AesCipher {
    pub fn new(key: &[u8]) -> Result<Self, String> {
        let (nk, nr, variant) = match key.len() {
            16 => (4, 10, AesVariant::Aes128),
            32 => (8, 14, AesVariant::Aes256),
            _ => return Err("AES key must be 16 or 32 bytes".to_string()),
        };

        let words = expand_key(key, nk, nr)?;
        let mut round_keys = Vec::with_capacity(nr + 1);
        for round in 0..=nr {
            let mut rk = [0u8; 16];
            for i in 0..4 {
                rk[i * 4..(i + 1) * 4].copy_from_slice(&words[round * 4 + i].to_be_bytes());
            }
            round_keys.push(rk);
        }

        let _ = variant;
        Ok(Self { round_keys, nr })
    }

    pub fn encrypt_block(&self, block: &[u8; 16]) -> [u8; 16] {
        let mut state = *block;
        add_round_key(&mut state, &self.round_keys[0]);

        for round in 1..self.nr {
            sub_bytes(&mut state);
            shift_rows(&mut state);
            mix_columns(&mut state);
            add_round_key(&mut state, &self.round_keys[round]);
        }

        sub_bytes(&mut state);
        shift_rows(&mut state);
        add_round_key(&mut state, &self.round_keys[self.nr]);
        state
    }

    pub fn decrypt_block(&self, block: &[u8; 16]) -> [u8; 16] {
        let mut state = *block;
        add_round_key(&mut state, &self.round_keys[self.nr]);

        for round in (1..self.nr).rev() {
            inv_shift_rows(&mut state);
            inv_sub_bytes(&mut state);
            add_round_key(&mut state, &self.round_keys[round]);
            inv_mix_columns(&mut state);
        }

        inv_shift_rows(&mut state);
        inv_sub_bytes(&mut state);
        add_round_key(&mut state, &self.round_keys[0]);
        state
    }
}

fn expand_key(key: &[u8], nk: usize, nr: usize) -> Result<Vec<u32>, String> {
    let n_words = 4 * (nr + 1);
    let mut w = vec![0u32; n_words];

    for i in 0..nk {
        w[i] = u32::from_be_bytes([key[i * 4], key[i * 4 + 1], key[i * 4 + 2], key[i * 4 + 3]]);
    }

    for i in nk..n_words {
        let mut temp = w[i - 1];
        if i % nk == 0 {
            temp = sub_word(rot_word(temp)) ^ ((rcon(i / nk) as u32) << 24);
        } else if nk > 6 && i % nk == 4 {
            temp = sub_word(temp);
        }
        w[i] = w[i - nk] ^ temp;
    }

    Ok(w)
}

fn add_round_key(state: &mut [u8; 16], round_key: &[u8; 16]) {
    for i in 0..16 {
        state[i] ^= round_key[i];
    }
}

fn sub_bytes(state: &mut [u8; 16]) {
    for byte in state.iter_mut() {
        *byte = SBOX[*byte as usize];
    }
}

fn inv_sub_bytes(state: &mut [u8; 16]) {
    for byte in state.iter_mut() {
        *byte = INV_SBOX[*byte as usize];
    }
}

fn shift_rows(state: &mut [u8; 16]) {
    let original = *state;
    for r in 0..4 {
        for c in 0..4 {
            state[c * 4 + r] = original[((c + r) % 4) * 4 + r];
        }
    }
}

fn inv_shift_rows(state: &mut [u8; 16]) {
    let original = *state;
    for r in 0..4 {
        for c in 0..4 {
            state[c * 4 + r] = original[((c + 4 - r) % 4) * 4 + r];
        }
    }
}

fn mix_columns(state: &mut [u8; 16]) {
    for c in 0..4 {
        let i = c * 4;
        let a0 = state[i];
        let a1 = state[i + 1];
        let a2 = state[i + 2];
        let a3 = state[i + 3];
        state[i] = gf_mul(a0, 2) ^ gf_mul(a1, 3) ^ a2 ^ a3;
        state[i + 1] = a0 ^ gf_mul(a1, 2) ^ gf_mul(a2, 3) ^ a3;
        state[i + 2] = a0 ^ a1 ^ gf_mul(a2, 2) ^ gf_mul(a3, 3);
        state[i + 3] = gf_mul(a0, 3) ^ a1 ^ a2 ^ gf_mul(a3, 2);
    }
}

fn inv_mix_columns(state: &mut [u8; 16]) {
    for c in 0..4 {
        let i = c * 4;
        let a0 = state[i];
        let a1 = state[i + 1];
        let a2 = state[i + 2];
        let a3 = state[i + 3];
        state[i] = gf_mul(a0, 14) ^ gf_mul(a1, 11) ^ gf_mul(a2, 13) ^ gf_mul(a3, 9);
        state[i + 1] = gf_mul(a0, 9) ^ gf_mul(a1, 14) ^ gf_mul(a2, 11) ^ gf_mul(a3, 13);
        state[i + 2] = gf_mul(a0, 13) ^ gf_mul(a1, 9) ^ gf_mul(a2, 14) ^ gf_mul(a3, 11);
        state[i + 3] = gf_mul(a0, 11) ^ gf_mul(a1, 13) ^ gf_mul(a2, 9) ^ gf_mul(a3, 14);
    }
}

fn rot_word(word: u32) -> u32 {
    word.rotate_left(8)
}

fn sub_word(word: u32) -> u32 {
    let bytes = word.to_be_bytes();
    u32::from_be_bytes([
        SBOX[bytes[0] as usize],
        SBOX[bytes[1] as usize],
        SBOX[bytes[2] as usize],
        SBOX[bytes[3] as usize],
    ])
}

fn rcon(i: usize) -> u8 {
    let mut c = 1u8;
    if i == 0 {
        return 0;
    }
    for _ in 1..i {
        c = gf_mul(c, 2);
    }
    c
}

fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut p = 0u8;
    for _ in 0..8 {
        if (b & 1) != 0 {
            p ^= a;
        }
        let hi = a & 0x80;
        a <<= 1;
        if hi != 0 {
            a ^= 0x1b;
        }
        b >>= 1;
    }
    p
}

fn gf_pow(mut x: u8, mut exp: u16) -> u8 {
    let mut result = 1u8;
    while exp > 0 {
        if (exp & 1) != 0 {
            result = gf_mul(result, x);
        }
        x = gf_mul(x, x);
        exp >>= 1;
    }
    result
}

fn sbox_value(x: u8) -> u8 {
    let inv = if x == 0 { 0 } else { gf_pow(x, 254) };
    let mut y = inv;
    y ^= inv.rotate_left(1);
    y ^= inv.rotate_left(2);
    y ^= inv.rotate_left(3);
    y ^= inv.rotate_left(4);
    y ^ 0x63
}

fn build_sbox() -> [u8; 256] {
    let mut sbox = [0u8; 256];
    for (i, slot) in sbox.iter_mut().enumerate() {
        *slot = sbox_value(i as u8);
    }
    sbox
}

fn build_inv_sbox() -> [u8; 256] {
    let mut inv = [0u8; 256];
    for i in 0..=255u16 {
        inv[SBOX[i as usize] as usize] = i as u8;
    }
    inv
}

#[cfg(test)]
mod tests {
    use super::AesCipher;
    use crate::ace::util::hex::decode;

    #[test]
    fn aes128_encrypt_decrypt_known_vector() {
        let key = decode("000102030405060708090a0b0c0d0e0f").unwrap();
        let plaintext = decode("00112233445566778899aabbccddeeff").unwrap();
        let expected = decode("69c4e0d86a7b0430d8cdb78070b4c55a").unwrap();

        let cipher = AesCipher::new(&key).unwrap();
        let ct = cipher.encrypt_block(&plaintext.clone().try_into().unwrap());
        assert_eq!(ct.as_slice(), expected.as_slice());
        let pt = cipher.decrypt_block(&ct);
        assert_eq!(pt.as_slice(), plaintext.as_slice());
    }

    #[test]
    fn aes256_encrypt_decrypt_known_vector() {
        let key =
            decode("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f").unwrap();
        let plaintext = decode("00112233445566778899aabbccddeeff").unwrap();
        let expected = decode("8ea2b7ca516745bfeafc49904b496089").unwrap();

        let cipher = AesCipher::new(&key).unwrap();
        let ct = cipher.encrypt_block(&plaintext.clone().try_into().unwrap());
        assert_eq!(ct.as_slice(), expected.as_slice());
        let pt = cipher.decrypt_block(&ct);
        assert_eq!(pt.as_slice(), plaintext.as_slice());
    }
}
