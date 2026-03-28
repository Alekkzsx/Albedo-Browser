use crate::ace::crypto::aes::AesCipher;

pub struct AesGcm {
    cipher: AesCipher,
    h: u128,
}

impl AesGcm {
    pub fn new(key: &[u8]) -> Result<Self, String> {
        let cipher = AesCipher::new(key)?;
        let zero = [0u8; 16];
        let h = u128::from_be_bytes(cipher.encrypt_block(&zero));
        Ok(Self { cipher, h })
    }

    pub fn encrypt(
        &self,
        nonce: &[u8],
        aad: &[u8],
        plaintext: &[u8],
    ) -> Result<(Vec<u8>, [u8; 16]), String> {
        let j0 = self.compute_j0(nonce)?;
        let ciphertext = self.ctr_crypt(j0, plaintext);
        let tag = self.compute_tag(j0, aad, &ciphertext);
        Ok((ciphertext, tag))
    }

    pub fn decrypt(
        &self,
        nonce: &[u8],
        aad: &[u8],
        ciphertext: &[u8],
        tag: &[u8; 16],
    ) -> Result<Vec<u8>, String> {
        let j0 = self.compute_j0(nonce)?;
        let expected = self.compute_tag(j0, aad, ciphertext);
        if &expected != tag {
            return Err("AES-GCM authentication failed".to_string());
        }
        Ok(self.ctr_crypt(j0, ciphertext))
    }

    fn ctr_crypt(&self, j0: [u8; 16], input: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; input.len()];
        let mut counter = inc32(j0);

        for (chunk_idx, chunk) in input.chunks(16).enumerate() {
            let stream = self.cipher.encrypt_block(&counter);
            let start = chunk_idx * 16;
            for i in 0..chunk.len() {
                out[start + i] = chunk[i] ^ stream[i];
            }
            counter = inc32(counter);
        }

        out
    }

    fn compute_j0(&self, nonce: &[u8]) -> Result<[u8; 16], String> {
        if nonce.len() == 12 {
            let mut j0 = [0u8; 16];
            j0[..12].copy_from_slice(nonce);
            j0[15] = 1;
            return Ok(j0);
        }

        Ok(ghash(self.h, &[], nonce).to_be_bytes())
    }

    fn compute_tag(&self, j0: [u8; 16], aad: &[u8], ciphertext: &[u8]) -> [u8; 16] {
        let s = ghash(self.h, aad, ciphertext);
        let e = self.cipher.encrypt_block(&j0);
        (u128::from_be_bytes(e) ^ s).to_be_bytes()
    }
}

fn ghash(h: u128, aad: &[u8], ciphertext: &[u8]) -> u128 {
    let mut y = 0u128;
    for chunk in aad.chunks(16) {
        y ^= block_to_u128(chunk);
        y = gf_mul_gcm(y, h);
    }
    for chunk in ciphertext.chunks(16) {
        y ^= block_to_u128(chunk);
        y = gf_mul_gcm(y, h);
    }
    y ^= make_len_block(aad.len() as u64 * 8, ciphertext.len() as u64 * 8);
    gf_mul_gcm(y, h)
}

fn gf_mul_gcm(mut x: u128, mut y: u128) -> u128 {
    let r = 0xe100_0000_0000_0000_0000_0000_0000_0000u128;
    let mut z = 0u128;

    for _ in 0..128 {
        if (x & (1u128 << 127)) != 0 {
            z ^= y;
        }
        let lsb = y & 1;
        y >>= 1;
        if lsb != 0 {
            y ^= r;
        }
        x <<= 1;
    }
    z
}

fn block_to_u128(chunk: &[u8]) -> u128 {
    let mut block = [0u8; 16];
    block[..chunk.len()].copy_from_slice(chunk);
    u128::from_be_bytes(block)
}

fn make_len_block(a_bits: u64, c_bits: u64) -> u128 {
    let mut block = [0u8; 16];
    block[..8].copy_from_slice(&a_bits.to_be_bytes());
    block[8..].copy_from_slice(&c_bits.to_be_bytes());
    u128::from_be_bytes(block)
}

fn inc32(mut block: [u8; 16]) -> [u8; 16] {
    let mut ctr = u32::from_be_bytes([block[12], block[13], block[14], block[15]]);
    ctr = ctr.wrapping_add(1);
    block[12..16].copy_from_slice(&ctr.to_be_bytes());
    block
}

#[cfg(test)]
mod tests {
    use super::AesGcm;
    use crate::ace::util::hex::decode;

    #[test]
    fn gcm_empty_vector() {
        let key = [0u8; 16];
        let iv = [0u8; 12];
        let gcm = AesGcm::new(&key).unwrap();
        let (ciphertext, tag) = gcm.encrypt(&iv, &[], &[]).unwrap();
        assert!(ciphertext.is_empty());
        assert_eq!(
            tag.as_slice(),
            decode("58e2fccefa7e3061367f1d57a4e7455a")
                .unwrap()
                .as_slice()
        );
    }

    #[test]
    fn gcm_known_vector_encrypt_decrypt() {
        let key = [0u8; 16];
        let iv = [0u8; 12];
        let plaintext = [0u8; 16];
        let gcm = AesGcm::new(&key).unwrap();
        let (ciphertext, tag) = gcm.encrypt(&iv, &[], &plaintext).unwrap();

        assert_eq!(
            ciphertext.as_slice(),
            decode("0388dace60b6a392f328c2b971b2fe78")
                .unwrap()
                .as_slice()
        );
        assert_eq!(
            tag.as_slice(),
            decode("ab6e47d42cec13bdf53a67b21257bddf")
                .unwrap()
                .as_slice()
        );

        let decrypted = gcm.decrypt(&iv, &[], &ciphertext, &tag).unwrap();
        assert_eq!(decrypted.as_slice(), plaintext.as_slice());
    }
}
