use crate::ace::crypto::ecdh::{
    p256_generate_keypair, P256PublicKey, P256SecretKey, X25519PublicKey, X25519SecretKey,
};
use crate::ace::crypto::gcm::AesGcm;
use crate::ace::crypto::hmac::{hmac_sha1, hmac_sha256, hmac_sha512};
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{ArrayBuffer, Class, Ctx, IntoJs, Object, Result, Value};

#[derive(Clone)]
enum CryptoKeyMaterial {
    Secret(Vec<u8>),
    X25519Private([u8; 32]),
    X25519Public([u8; 32]),
    P256Private([u8; 32]),
    P256Public(Vec<u8>),
}

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

    #[qjs(rename = "generateKey")]
    pub fn generate_key<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        extractable: bool,
        usages: Vec<String>,
    ) -> Result<Value<'js>> {
        resolved_promise(&ctx, |ctx| {
            let alg_name = algorithm_name(algorithm.clone())?;
            match alg_name.as_str() {
                "HMAC" => {
                    let length_bits = algorithm_usize(&algorithm, "length").unwrap_or(256);
                    let hash = algorithm_hash_name(&algorithm).unwrap_or_else(|| "SHA-256".into());
                    let mut key = vec![0u8; length_bits / 8];
                    crate::ace::crypto::random::get_random_bytes(&mut key);
                    crypto_key_value(
                        ctx,
                        CryptoKey {
                            algorithm: "HMAC".into(),
                            key_type: "secret".into(),
                            extractable,
                            usages,
                            hash: Some(hash),
                            named_curve: None,
                            material: CryptoKeyMaterial::Secret(key),
                        },
                    )
                }
                "AES-GCM" => {
                    let length_bits = algorithm_usize(&algorithm, "length").unwrap_or(256);
                    if length_bits != 128 && length_bits != 256 {
                        return Err(rquickjs::Error::new_from_js(
                            "Crypto",
                            "AES-GCM key length must be 128 or 256",
                        ));
                    }
                    let mut key = vec![0u8; length_bits / 8];
                    crate::ace::crypto::random::get_random_bytes(&mut key);
                    crypto_key_value(
                        ctx,
                        CryptoKey {
                            algorithm: "AES-GCM".into(),
                            key_type: "secret".into(),
                            extractable,
                            usages,
                            hash: None,
                            named_curve: None,
                            material: CryptoKeyMaterial::Secret(key),
                        },
                    )
                }
                "ECDH" => {
                    let curve = algorithm_named_curve(&algorithm)?;
                    let key_pair = Object::new(ctx.clone())?;
                    match curve.as_str() {
                        "X25519" => {
                            let private = X25519SecretKey::generate();
                            let public = private.public_key();
                            key_pair.set(
                                "privateKey",
                                crypto_key_instance(
                                    ctx.clone(),
                                    CryptoKey {
                                        algorithm: "ECDH".into(),
                                        key_type: "private".into(),
                                        extractable,
                                        usages: usages.clone(),
                                        hash: None,
                                        named_curve: Some(curve.clone()),
                                        material: CryptoKeyMaterial::X25519Private(
                                            private.to_bytes(),
                                        ),
                                    },
                                )?,
                            )?;
                            key_pair.set(
                                "publicKey",
                                crypto_key_instance(
                                    ctx.clone(),
                                    CryptoKey {
                                        algorithm: "ECDH".into(),
                                        key_type: "public".into(),
                                        extractable: true,
                                        usages: vec![],
                                        hash: None,
                                        named_curve: Some(curve),
                                        material: CryptoKeyMaterial::X25519Public(
                                            public.to_bytes(),
                                        ),
                                    },
                                )?,
                            )?;
                        }
                        "P-256" => {
                            let (sk, pk) = p256_generate_keypair().map_err(to_js_crypto_err)?;
                            key_pair.set(
                                "privateKey",
                                crypto_key_instance(
                                    ctx.clone(),
                                    CryptoKey {
                                        algorithm: "ECDH".into(),
                                        key_type: "private".into(),
                                        extractable,
                                        usages: usages.clone(),
                                        hash: None,
                                        named_curve: Some(curve.clone()),
                                        material: CryptoKeyMaterial::P256Private(sk),
                                    },
                                )?,
                            )?;
                            key_pair.set(
                                "publicKey",
                                crypto_key_instance(
                                    ctx.clone(),
                                    CryptoKey {
                                        algorithm: "ECDH".into(),
                                        key_type: "public".into(),
                                        extractable: true,
                                        usages: vec![],
                                        hash: None,
                                        named_curve: Some(curve),
                                        material: CryptoKeyMaterial::P256Public(pk),
                                    },
                                )?,
                            )?;
                        }
                        _ => {
                            return Err(rquickjs::Error::new_from_js(
                                "Crypto",
                                "Unsupported ECDH curve",
                            ));
                        }
                    }
                    Ok(key_pair.into_value())
                }
                _ => Err(rquickjs::Error::new_from_js(
                    "Crypto",
                    "Unsupported key generation algorithm",
                )),
            }
        })
    }

    #[qjs(rename = "importKey")]
    pub fn import_key<'js>(
        &self,
        ctx: Ctx<'js>,
        format: String,
        key_data: Value<'js>,
        algorithm: Value<'js>,
        extractable: bool,
        usages: Vec<String>,
    ) -> Result<Value<'js>> {
        resolved_promise(&ctx, |ctx| {
            let alg_name = algorithm_name(algorithm.clone())?;
            let bytes = extract_bytes(key_data)?;

            match alg_name.as_str() {
                "HMAC" => crypto_key_value(
                    ctx,
                    CryptoKey {
                        algorithm: "HMAC".into(),
                        key_type: "secret".into(),
                        extractable,
                        usages,
                        hash: Some(
                            algorithm_hash_name(&algorithm).unwrap_or_else(|| "SHA-256".into()),
                        ),
                        named_curve: None,
                        material: CryptoKeyMaterial::Secret(bytes),
                    },
                ),
                "AES-GCM" => {
                    if bytes.len() != 16 && bytes.len() != 32 {
                        return Err(rquickjs::Error::new_from_js(
                            "Crypto",
                            "AES-GCM raw key must be 16 or 32 bytes",
                        ));
                    }
                    crypto_key_value(
                        ctx,
                        CryptoKey {
                            algorithm: "AES-GCM".into(),
                            key_type: "secret".into(),
                            extractable,
                            usages,
                            hash: None,
                            named_curve: None,
                            material: CryptoKeyMaterial::Secret(bytes),
                        },
                    )
                }
                "ECDH" => {
                    let curve = algorithm_named_curve(&algorithm)?;
                    match (curve.as_str(), format.as_str()) {
                        ("X25519", "raw") => {
                            let raw: [u8; 32] = bytes.try_into().map_err(|_| {
                                rquickjs::Error::new_from_js(
                                    "Crypto",
                                    "X25519 public key must be 32 bytes",
                                )
                            })?;
                            crypto_key_value(
                                ctx,
                                CryptoKey {
                                    algorithm: "ECDH".into(),
                                    key_type: "public".into(),
                                    extractable,
                                    usages,
                                    hash: None,
                                    named_curve: Some(curve),
                                    material: CryptoKeyMaterial::X25519Public(raw),
                                },
                            )
                        }
                        ("X25519", "raw-private") => {
                            let raw: [u8; 32] = bytes.try_into().map_err(|_| {
                                rquickjs::Error::new_from_js(
                                    "Crypto",
                                    "X25519 private key must be 32 bytes",
                                )
                            })?;
                            crypto_key_value(
                                ctx,
                                CryptoKey {
                                    algorithm: "ECDH".into(),
                                    key_type: "private".into(),
                                    extractable,
                                    usages,
                                    hash: None,
                                    named_curve: Some(curve),
                                    material: CryptoKeyMaterial::X25519Private(
                                        X25519SecretKey::from_bytes(raw).to_bytes(),
                                    ),
                                },
                            )
                        }
                        ("P-256", "raw") => {
                            let pk = P256PublicKey::from_uncompressed_bytes(&bytes)
                                .map_err(to_js_crypto_err)?;
                            crypto_key_value(
                                ctx,
                                CryptoKey {
                                    algorithm: "ECDH".into(),
                                    key_type: "public".into(),
                                    extractable,
                                    usages,
                                    hash: None,
                                    named_curve: Some(curve),
                                    material: CryptoKeyMaterial::P256Public(
                                        pk.to_uncompressed_bytes(),
                                    ),
                                },
                            )
                        }
                        ("P-256", "raw-private") => {
                            let raw: [u8; 32] = bytes.try_into().map_err(|_| {
                                rquickjs::Error::new_from_js(
                                    "Crypto",
                                    "P-256 private key must be 32 bytes",
                                )
                            })?;
                            let sk = P256SecretKey::from_bytes(raw).map_err(to_js_crypto_err)?;
                            crypto_key_value(
                                ctx,
                                CryptoKey {
                                    algorithm: "ECDH".into(),
                                    key_type: "private".into(),
                                    extractable,
                                    usages,
                                    hash: None,
                                    named_curve: Some(curve),
                                    material: CryptoKeyMaterial::P256Private(sk.to_bytes()),
                                },
                            )
                        }
                        _ => Err(rquickjs::Error::new_from_js(
                            "Crypto",
                            "Unsupported key import format for ECDH",
                        )),
                    }
                }
                _ => Err(rquickjs::Error::new_from_js(
                    "Crypto",
                    "Unsupported key import algorithm",
                )),
            }
        })
    }

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

    pub fn encrypt<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        key: Value<'js>,
        data: Value<'js>,
    ) -> Result<Value<'js>> {
        resolved_promise(&ctx, |ctx| {
            let alg_name = algorithm_name(algorithm.clone())?;
            if alg_name != "AES-GCM" {
                return Err(rquickjs::Error::new_from_js(
                    "Crypto",
                    "Only AES-GCM encryption is currently supported",
                ));
            }
            let iv = algorithm_bytes(&algorithm, "iv")?;
            let aad = algorithm_optional_bytes(&algorithm, "additionalData").unwrap_or_default();
            let _tag_length = algorithm_usize(&algorithm, "tagLength").unwrap_or(128);
            let plaintext = extract_bytes(data)?;
            let key = crypto_key_from_value(key)?;
            let secret = match &key.material {
                CryptoKeyMaterial::Secret(bytes) => bytes.as_slice(),
                _ => {
                    return Err(rquickjs::Error::new_from_js(
                        "Crypto",
                        "encrypt expects a secret CryptoKey",
                    ));
                }
            };

            let gcm = AesGcm::new(secret).map_err(to_js_crypto_err)?;
            let (mut ciphertext, tag) = gcm
                .encrypt(&iv, &aad, &plaintext)
                .map_err(to_js_crypto_err)?;
            ciphertext.extend_from_slice(&tag);
            Ok(ArrayBuffer::new(ctx, ciphertext)?.into_value())
        })
    }

    pub fn decrypt<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        key: Value<'js>,
        data: Value<'js>,
    ) -> Result<Value<'js>> {
        resolved_promise(&ctx, |ctx| {
            let alg_name = algorithm_name(algorithm.clone())?;
            if alg_name != "AES-GCM" {
                return Err(rquickjs::Error::new_from_js(
                    "Crypto",
                    "Only AES-GCM decryption is currently supported",
                ));
            }
            let iv = algorithm_bytes(&algorithm, "iv")?;
            let aad = algorithm_optional_bytes(&algorithm, "additionalData").unwrap_or_default();
            let tag_length = algorithm_usize(&algorithm, "tagLength").unwrap_or(128);
            let all = extract_bytes(data)?;
            let tag_bytes = tag_length / 8;
            if all.len() < tag_bytes {
                return Err(rquickjs::Error::new_from_js(
                    "Crypto",
                    "Ciphertext too short for AES-GCM tag",
                ));
            }
            let ciphertext_len = all.len() - tag_bytes;
            let (ciphertext, tag_slice) = all.split_at(ciphertext_len);
            let tag: [u8; 16] = tag_slice.try_into().map_err(|_| {
                rquickjs::Error::new_from_js("Crypto", "AES-GCM tag must be 16 bytes")
            })?;

            let key = crypto_key_from_value(key)?;
            let secret = match &key.material {
                CryptoKeyMaterial::Secret(bytes) => bytes.as_slice(),
                _ => {
                    return Err(rquickjs::Error::new_from_js(
                        "Crypto",
                        "decrypt expects a secret CryptoKey",
                    ));
                }
            };

            let gcm = AesGcm::new(secret).map_err(to_js_crypto_err)?;
            let plaintext = gcm
                .decrypt(&iv, &aad, ciphertext, &tag)
                .map_err(to_js_crypto_err)?;
            Ok(ArrayBuffer::new(ctx, plaintext)?.into_value())
        })
    }

    #[qjs(rename = "deriveBits")]
    pub fn derive_bits<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        base_key: Value<'js>,
        length: usize,
    ) -> Result<Value<'js>> {
        resolved_promise(&ctx, |ctx| {
            let alg_name = algorithm_name(algorithm.clone())?;
            if alg_name != "ECDH" {
                return Err(rquickjs::Error::new_from_js(
                    "Crypto",
                    "Only ECDH deriveBits is currently supported",
                ));
            }
            let public_val = algorithm_object(&algorithm)?
                .get::<_, Value>("public")
                .map_err(|_| {
                    rquickjs::Error::new_from_js("Crypto", "ECDH deriveBits requires public key")
                })?;
            let public_key = crypto_key_from_value(public_val)?;
            let base_key = crypto_key_from_value(base_key)?;
            let bytes_len = length / 8;

            let shared = match (&base_key.material, &public_key.material) {
                (CryptoKeyMaterial::X25519Private(sk), CryptoKeyMaterial::X25519Public(pk)) => {
                    X25519SecretKey::from_bytes(*sk)
                        .diffie_hellman(&X25519PublicKey::from_bytes(*pk))
                        .to_vec()
                }
                (CryptoKeyMaterial::P256Private(sk), CryptoKeyMaterial::P256Public(pk)) => {
                    let sk = P256SecretKey::from_bytes(*sk).map_err(to_js_crypto_err)?;
                    let pk =
                        P256PublicKey::from_uncompressed_bytes(pk).map_err(to_js_crypto_err)?;
                    sk.diffie_hellman(&pk).map_err(to_js_crypto_err)?.to_vec()
                }
                _ => {
                    return Err(rquickjs::Error::new_from_js(
                        "Crypto",
                        "ECDH key types do not match",
                    ));
                }
            };

            let mut out = shared;
            out.truncate(bytes_len.min(out.len()));
            Ok(ArrayBuffer::new(ctx, out)?.into_value())
        })
    }
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Crypto {
    pub subtle: SubtleCrypto,
}

#[rquickjs::methods]
impl Crypto {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            subtle: SubtleCrypto {},
        }
    }

    #[qjs(get)]
    pub fn subtle(&self) -> SubtleCrypto {
        self.subtle.clone()
    }

    #[qjs(rename = "getRandomValues")]
    pub fn get_random_values<'js>(&self, typed_array: Object<'js>) -> Result<Object<'js>> {
        if let Some(ta) = typed_array.as_typed_array::<u8>() {
            let mut buf = vec![0u8; ta.len()];
            crate::ace::crypto::random::get_random_bytes(&mut buf);
            copy_into_typed_array(&ta, &buf);
        } else if let Some(ta) = typed_array.as_typed_array::<u16>() {
            let mut buf = vec![0u8; ta.len() * 2];
            crate::ace::crypto::random::get_random_bytes(&mut buf);
            copy_into_typed_array(&ta, &buf);
        } else if let Some(ta) = typed_array.as_typed_array::<u32>() {
            let mut buf = vec![0u8; ta.len() * 4];
            crate::ace::crypto::random::get_random_bytes(&mut buf);
            copy_into_typed_array(&ta, &buf);
        } else if let Some(ta) = typed_array.as_typed_array::<i8>() {
            let mut buf = vec![0u8; ta.len()];
            crate::ace::crypto::random::get_random_bytes(&mut buf);
            copy_into_typed_array(&ta, &buf);
        } else if let Some(ta) = typed_array.as_typed_array::<i16>() {
            let mut buf = vec![0u8; ta.len() * 2];
            crate::ace::crypto::random::get_random_bytes(&mut buf);
            copy_into_typed_array(&ta, &buf);
        } else if let Some(ta) = typed_array.as_typed_array::<i32>() {
            let mut buf = vec![0u8; ta.len() * 4];
            crate::ace::crypto::random::get_random_bytes(&mut buf);
            copy_into_typed_array(&ta, &buf);
        }

        Ok(typed_array)
    }

    pub fn random_uuid(&self) -> String {
        crate::ace::util::uuid::Uuid::new_v4().to_string()
    }
}

fn copy_into_typed_array<T>(ta: &rquickjs::TypedArray<'_, T>, buf: &[u8]) {
    if let Some(raw) = ta.as_raw() {
        unsafe {
            std::ptr::copy_nonoverlapping(buf.as_ptr(), raw.ptr.as_ptr() as *mut u8, buf.len());
        }
    }
}

fn resolved_promise<'js, F>(ctx: &Ctx<'js>, f: F) -> Result<Value<'js>>
where
    F: FnOnce(Ctx<'js>) -> Result<Value<'js>>,
{
    let (promise, resolve, reject) = rquickjs::Promise::new(ctx)?;
    match f(ctx.clone()) {
        Ok(value) => {
            let _ = resolve.call::<(Value,), ()>((value,));
        }
        Err(err) => {
            let _ = reject.call::<(String,), ()>((format!("{}", err),));
        }
    }
    Ok(promise.into_value())
}

fn algorithm_name(value: Value<'_>) -> Result<String> {
    if let Some(s) = value.as_string() {
        return s.to_string();
    }
    if let Some(obj) = value.as_object() {
        return obj.get("name");
    }
    Err(rquickjs::Error::new_from_js("Crypto", "Invalid algorithm"))
}

fn algorithm_object<'js>(value: &Value<'js>) -> Result<Object<'js>> {
    value
        .as_object()
        .ok_or_else(|| rquickjs::Error::new_from_js("Crypto", "Algorithm must be an object"))
        .cloned()
}

fn algorithm_hash_name(value: &Value<'_>) -> Option<String> {
    let obj = value.as_object()?;
    let hash_value: Value = obj.get("hash").ok()?;
    if let Some(s) = hash_value.as_string() {
        s.to_string().ok()
    } else if let Some(hash_obj) = hash_value.as_object() {
        hash_obj.get("name").ok()
    } else {
        None
    }
}

fn algorithm_named_curve(value: &Value<'_>) -> Result<String> {
    algorithm_object(value)?.get("namedCurve")
}

fn algorithm_usize(value: &Value<'_>, key: &str) -> Option<usize> {
    let obj = value.as_object()?;
    obj.get::<_, i32>(key).ok().map(|v| v as usize)
}

fn algorithm_optional_bytes(value: &Value<'_>, key: &str) -> Option<Vec<u8>> {
    let obj = value.as_object()?;
    let field = obj.get::<_, Value>(key).ok()?;
    extract_bytes(field).ok()
}

fn algorithm_bytes(value: &Value<'_>, key: &str) -> Result<Vec<u8>> {
    let obj = algorithm_object(value)?;
    let field: Value = obj.get(key)?;
    extract_bytes(field)
}

fn extract_bytes(data: Value<'_>) -> Result<Vec<u8>> {
    if let Some(s) = data.as_string() {
        return Ok(s.to_string()?.into_bytes());
    }
    if let Some(obj) = data.as_object() {
        if let Some(ab) = obj.as_array_buffer() {
            let slice: &[u8] = ab.as_ref();
            return Ok(slice.to_vec());
        }
        if let Some(ta) = obj.as_typed_array::<u8>() {
            return Ok(ta.as_bytes().unwrap().to_vec());
        }
    }
    Err(rquickjs::Error::new_from_js("Crypto", "Invalid data type"))
}

fn value_to_bytes(value: Value<'_>) -> Result<Vec<u8>> {
    extract_bytes(value)
}

fn crypto_key_instance<'js>(ctx: Ctx<'js>, key: CryptoKey) -> Result<Value<'js>> {
    Ok(Class::instance(ctx, key)?.into_value())
}

fn crypto_key_value<'js>(ctx: Ctx<'js>, key: CryptoKey) -> Result<Value<'js>> {
    crypto_key_instance(ctx, key)
}

fn crypto_key_from_value<'js>(value: Value<'js>) -> Result<CryptoKey> {
    let obj = value
        .as_object()
        .ok_or_else(|| rquickjs::Error::new_from_js("Crypto", "Expected CryptoKey object"))?;
    Ok(Class::<CryptoKey>::from_object(obj)
        .ok_or_else(|| rquickjs::Error::new_from_js("Crypto", "Invalid CryptoKey object"))?
        .borrow()
        .clone())
}

fn to_js_crypto_err(err: String) -> rquickjs::Error {
    let msg = Box::leak(err.into_boxed_str());
    rquickjs::Error::new_from_js("Crypto", msg)
}

pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            Class::<CryptoKey>::register(&ctx)?;
            Class::<SubtleCrypto>::register(&ctx)?;
            Class::<Crypto>::register(&ctx)?;
            let crypto = Class::instance(ctx.clone(), Crypto::new())?;
            ctx.globals().set("crypto", crypto)?;
            Ok(())
        })
    })
}
