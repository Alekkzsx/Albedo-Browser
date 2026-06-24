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

