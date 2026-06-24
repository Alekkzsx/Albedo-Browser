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

