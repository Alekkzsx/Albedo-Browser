use super::*;
use crate::ace::crypto::ecdh::{
    p256_generate_keypair, P256PublicKey, P256SecretKey, X25519PublicKey, X25519SecretKey,
};
use crate::ace::crypto::gcm::AesGcm;
use crate::ace::crypto::hmac::{hmac_sha1, hmac_sha256, hmac_sha512};
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{ArrayBuffer, Class, Ctx, IntoJs, Object, Result, Value};


#[derive(Clone)]
pub(crate) enum CryptoKeyMaterial {
    Secret(Vec<u8>),
    X25519Private([u8; 32]),
    X25519Public([u8; 32]),
    P256Private([u8; 32]),
    P256Public(Vec<u8>),
}
