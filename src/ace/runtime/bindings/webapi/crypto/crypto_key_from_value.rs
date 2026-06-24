use super::*;
use crate::ace::crypto::ecdh::{
    p256_generate_keypair, P256PublicKey, P256SecretKey, X25519PublicKey, X25519SecretKey,
};
use crate::ace::crypto::gcm::AesGcm;
use crate::ace::crypto::hmac::{hmac_sha1, hmac_sha256, hmac_sha512};
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{ArrayBuffer, Class, Ctx, IntoJs, Object, Result, Value};



pub(crate) fn crypto_key_from_value<'js>(value: Value<'js>) -> Result<CryptoKey> {
    let obj = value
        .as_object()
        .ok_or_else(|| rquickjs::Error::new_from_js("Crypto", "Expected CryptoKey object"))?;
    Ok(Class::<CryptoKey>::from_object(obj)
        .ok_or_else(|| rquickjs::Error::new_from_js("Crypto", "Invalid CryptoKey object"))?
        .borrow()
        .clone())
}
