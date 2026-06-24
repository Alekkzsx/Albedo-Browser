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
pub struct CryptoKey {
    #[qjs(skip_trace)]
    algorithm: String,
    #[qjs(skip_trace)]
    key_type: String,
    #[qjs(skip_trace)]
    extractable: bool,
    #[qjs(skip_trace)]
    usages: Vec<String>,
    #[qjs(skip_trace)]
    hash: Option<String>,
    #[qjs(skip_trace)]
    named_curve: Option<String>,
    #[qjs(skip_trace)]
    material: CryptoKeyMaterial,
}

#[rquickjs::methods]
impl CryptoKey {
    #[qjs(get, rename = "type")]
    pub fn key_type(&self) -> String {
        self.key_type.clone()
    }

    #[qjs(get)]
    pub fn extractable(&self) -> bool {
        self.extractable
    }

    #[qjs(get)]
    pub fn usages(&self) -> Vec<String> {
        self.usages.clone()
    }

    #[qjs(get)]
    pub fn algorithm<'js>(&self, ctx: Ctx<'js>) -> Result<Object<'js>> {
        let obj = Object::new(ctx.clone())?;
        obj.set("name", self.algorithm.clone())?;
        if let Some(hash) = &self.hash {
            let hash_obj = Object::new(ctx.clone())?;
            hash_obj.set("name", hash.clone())?;
            obj.set("hash", hash_obj)?;
        }
        if let Some(curve) = &self.named_curve {
            obj.set("namedCurve", curve.clone())?;
        }
        if let CryptoKeyMaterial::Secret(bytes) = &self.material {
            if self.algorithm == "AES-GCM" || self.algorithm == "HMAC" {
                obj.set("length", bytes.len() * 8)?;
            }
        }
        Ok(obj)
    }
}
