use crate::ace::crypto::hmac::{hmac_sha1, hmac_sha256};

pub fn hkdf_extract_sha256(salt: &[u8], ikm: &[u8]) -> [u8; 32] {
    let zero_salt = [0u8; 32];
    let actual_salt = if salt.is_empty() {
        &zero_salt[..]
    } else {
        salt
    };
    hmac_sha256(actual_salt, ikm)
}

pub fn hkdf_expand_sha256(prk: &[u8], info: &[u8], len: usize) -> Vec<u8> {
    hkdf_expand(prk, info, len, 32, hmac_sha256_vec)
}

pub fn hkdf_sha256(salt: &[u8], ikm: &[u8], info: &[u8], len: usize) -> Vec<u8> {
    let prk = hkdf_extract_sha256(salt, ikm);
    hkdf_expand_sha256(&prk, info, len)
}

pub fn hkdf_extract_sha1(salt: &[u8], ikm: &[u8]) -> [u8; 20] {
    let zero_salt = [0u8; 20];
    let actual_salt = if salt.is_empty() {
        &zero_salt[..]
    } else {
        salt
    };
    hmac_sha1(actual_salt, ikm)
}

pub fn hkdf_expand_sha1(prk: &[u8], info: &[u8], len: usize) -> Vec<u8> {
    hkdf_expand(prk, info, len, 20, hmac_sha1_vec)
}

fn hkdf_expand<F>(prk: &[u8], info: &[u8], len: usize, hash_len: usize, hmac: F) -> Vec<u8>
where
    F: Fn(&[u8], &[u8]) -> Vec<u8>,
{
    assert!(len <= 255 * hash_len, "HKDF output too large");

    let mut okm = Vec::with_capacity(len);
    let mut previous = Vec::new();
    let blocks = len.div_ceil(hash_len);

    for counter in 1..=blocks {
        let mut input = previous.clone();
        input.extend_from_slice(info);
        input.push(counter as u8);
        previous = hmac(prk, &input);
        okm.extend_from_slice(&previous);
    }

    okm.truncate(len);
    okm
}

fn hmac_sha256_vec(key: &[u8], data: &[u8]) -> Vec<u8> {
    crate::ace::crypto::hmac::hmac_sha256(key, data).to_vec()
}

fn hmac_sha1_vec(key: &[u8], data: &[u8]) -> Vec<u8> {
    crate::ace::crypto::hmac::hmac_sha1(key, data).to_vec()
}

#[cfg(test)]
mod tests {
    use super::{hkdf_expand_sha256, hkdf_extract_sha256};
    use crate::ace::util::hex::decode;

    #[test]
    fn hkdf_sha256_rfc5869_case_1() {
        let ikm = vec![0x0b; 22];
        let salt = decode("000102030405060708090a0b0c").unwrap();
        let info = decode("f0f1f2f3f4f5f6f7f8f9").unwrap();

        let prk = hkdf_extract_sha256(&salt, &ikm);
        assert_eq!(
            crate::ace::util::hex::encode(&prk),
            "077709362c2e32df0ddc3f0dc47bba63\
             90b6c73bb50f9c3122ec844ad7c2b3e5"
                .replace(' ', "")
        );

        let okm = hkdf_expand_sha256(&prk, &info, 42);
        assert_eq!(
            crate::ace::util::hex::encode(&okm),
            "3cb25f25faacd57a90434f64d0362f2a\
             2d2d0a90cf1a5a4c5db02d56ecc4c5bf\
             34007208d5b887185865"
                .replace(' ', "")
        );
    }
}
