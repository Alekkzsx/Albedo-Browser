use super::*;
use crate::ace::crypto::ecdh::{
    p256_generate_keypair, P256PublicKey, P256SecretKey, X25519PublicKey, X25519SecretKey,
};
use crate::ace::crypto::gcm::AesGcm;
use crate::ace::crypto::hmac::{hmac_sha1, hmac_sha256, hmac_sha512};
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{ArrayBuffer, Class, Ctx, IntoJs, Object, Result, Value};



#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct SubtleCrypto {}

#[rquickjs::methods]
impl SubtleCrypto {
    #[qjs(rename = "digest")]
    pub fn digest<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        data: Value<'js>,
    ) -> Result<Value<'js>> {
        resolved_promise(&ctx, |ctx| {
            let bytes_vec = extract_bytes(data)?;
            let algorithm = algorithm_name(algorithm)?;

            let hash = match algorithm.as_str() {
                "SHA-1" => crate::ace::crypto::sha1::sha1(&bytes_vec).to_vec(),
                "SHA-256" => crate::ace::crypto::sha2::sha256(&bytes_vec).to_vec(),
                "SHA-384" => crate::ace::crypto::sha2::sha384(&bytes_vec).to_vec(),
                "SHA-512" => crate::ace::crypto::sha2::sha512(&bytes_vec).to_vec(),
                _ => {
                    return Err(rquickjs::Error::new_from_js(
                        "Crypto",
                        "Unsupported digest algorithm",
                    ));
                }
            };

            Ok(ArrayBuffer::new(ctx, hash)?.into_value())
        })
    }

    include!("crypto_generate_key.rs");
    include!("crypto_import_key.rs");
    #[qjs(rename = "exportKey")]
    pub fn export_key<'js>(
        &self,
        ctx: Ctx<'js>,
        format: String,
        key: Value<'js>,
    ) -> Result<Value<'js>> {
        resolved_promise(&ctx, |ctx| {
            let key = crypto_key_from_value(key)?;
            let bytes = match (&key.material, format.as_str()) {
                (CryptoKeyMaterial::Secret(bytes), "raw") => bytes.clone(),
                (CryptoKeyMaterial::X25519Public(bytes), "raw") => bytes.to_vec(),
                (CryptoKeyMaterial::X25519Private(bytes), "raw-private") => bytes.to_vec(),
                (CryptoKeyMaterial::P256Public(bytes), "raw") => bytes.clone(),
                (CryptoKeyMaterial::P256Private(bytes), "raw-private") => bytes.to_vec(),
                _ => {
                    return Err(rquickjs::Error::new_from_js(
                        "Crypto",
                        "Unsupported export format",
                    ));
                }
            };
            Ok(ArrayBuffer::new(ctx, bytes)?.into_value())
        })
    }

    /// TODO: add docs
    pub fn sign<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        key: Value<'js>,
        data: Value<'js>,
    ) -> Result<Value<'js>> {
        resolved_promise(&ctx, |ctx| {
            let alg_name = algorithm_name(algorithm)?;
            if alg_name != "HMAC" {
                return Err(rquickjs::Error::new_from_js(
                    "Crypto",
                    "Only HMAC signing is currently supported",
                ));
            }

            let key = crypto_key_from_value(key)?;
            let data = extract_bytes(data)?;
            let secret = match &key.material {
                CryptoKeyMaterial::Secret(bytes) => bytes.as_slice(),
                _ => {
                    return Err(rquickjs::Error::new_from_js(
                        "Crypto",
                        "sign expects a secret CryptoKey",
                    ));
                }
            };

            let hash = key.hash.unwrap_or_else(|| "SHA-256".into());
            let mac = match hash.as_str() {
                "SHA-1" => hmac_sha1(secret, &data).to_vec(),
                "SHA-256" => hmac_sha256(secret, &data).to_vec(),
                "SHA-512" => hmac_sha512(secret, &data).to_vec(),
                _ => {
                    return Err(rquickjs::Error::new_from_js(
                        "Crypto",
                        "Unsupported HMAC hash",
                    ));
                }
            };

            Ok(ArrayBuffer::new(ctx, mac)?.into_value())
        })
    }

    /// TODO: add docs
    pub fn verify<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        key: Value<'js>,
        signature: Value<'js>,
        data: Value<'js>,
    ) -> Result<Value<'js>> {
        resolved_promise(&ctx, |ctx| {
            let expected = self.sign(ctx.clone(), algorithm, key, data)?;
            let expected = value_to_bytes(expected)?;
            let given = extract_bytes(signature)?;
            Ok((expected == given).into_js(&ctx)?)
        })
    }

    include!("crypto_encrypt.rs");
    include!("crypto_decrypt.rs");
    include!("crypto_derive_bits.rs");
