use super::*;
use crate::ace::crypto::ecdh::{
    p256_generate_keypair, P256PublicKey, P256SecretKey, X25519PublicKey, X25519SecretKey,
};
use crate::ace::crypto::gcm::AesGcm;
use crate::ace::crypto::hmac::{hmac_sha1, hmac_sha256, hmac_sha512};
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{ArrayBuffer, Class, Ctx, IntoJs, Object, Result, Value};



pub(crate) fn algorithm_name(value: Value<'_>) -> Result<String> {
    if let Some(s) = value.as_string() {
        return s.to_string();
    }
    if let Some(obj) = value.as_object() {
        return obj.get("name");
    }
    Err(rquickjs::Error::new_from_js("Crypto", "Invalid algorithm"))
}
