use rquickjs::{Class, Ctx, Result, Value, Object, ArrayBuffer};
use crate::js::JsRuntime;
use std::sync::{Arc, Mutex};
use sha2::{Sha256, Sha384, Sha512, Digest};
use sha1::Sha1;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct SubtleCrypto {}

#[rquickjs::methods]
impl SubtleCrypto {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    pub fn digest<'js>(&self, ctx: Ctx<'js>, algorithm: Value<'js>, data: Value<'js>) -> Result<Value<'js>> {
        let algo_str = if algorithm.is_string() {
            algorithm.as_string().unwrap().to_string()?
        } else if algorithm.is_object() {
            algorithm.as_object().unwrap().get::<_, String>("name")?
        } else {
            return Err(rquickjs::Error::new_from_js("Crypto", "Invalid algorithm"));
        };

        let data_vec = if let Some(ab) = data.as_object().and_then(|obj| obj.as_array_buffer()) {
            ab.as_ref().to_vec()
        } else if let Some(ta) = data.as_object().and_then(|obj| obj.as_typed_array::<u8>().ok()) {
            ta.as_ref().to_vec()
        } else {
            return Err(rquickjs::Error::new_from_js("Crypto", "Invalid data type"));
        };

        let (promise, resolve, reject) = rquickjs::Promise::new(&ctx)?;
        let rt = ctx.userdata::<JsRuntime>().expect("JsRuntime").clone();

        tokio::spawn(async move {
            let hash = match algo_str.to_uppercase().as_str() {
                "SHA-1" => {
                    let mut hasher = Sha1::new();
                    hasher.update(&data_vec);
                    hasher.finalize().to_vec()
                }
                "SHA-256" => {
                    let mut hasher = Sha256::new();
                    hasher.update(&data_vec);
                    hasher.finalize().to_vec()
                }
                "SHA-384" => {
                    let mut hasher = Sha384::new();
                    hasher.update(&data_vec);
                    hasher.finalize().to_vec()
                }
                "SHA-512" => {
                    let mut hasher = Sha512::new();
                    hasher.update(&data_vec);
                    hasher.finalize().to_vec()
                }
                _ => {
                    let mut el = rt.event_loop.lock().unwrap();
                    el.queue_macro_task(move || {
                        rt.with_context(|ctx| {
                            ctx.with(|ctx| {
                                let _ = reject.call::<(String,), ()>((format!("Unsupported algorithm: {}", algo_str),));
                            });
                        });
                    });
                    return;
                }
            };

            let mut el = rt.event_loop.lock().unwrap();
            el.queue_macro_task(move || {
                rt.with_context(|ctx| {
                    ctx.with(|ctx| {
                        if let Ok(ab) = ArrayBuffer::new(ctx.clone(), hash) {
                            let _ = resolve.call::<(ArrayBuffer,), ()>((ab,));
                        }
                    });
                });
            });
        });

        Ok(promise.into_value())
    }
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Crypto {}

#[rquickjs::methods]
impl Crypto {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    #[qjs(rename = "getRandomValues")]
    pub fn get_random_values<'js>(&self, ctx: Ctx<'js>, typed_array: Object<'js>) -> Result<Object<'js>> {
        // Simple but safe implementation for common typed arrays
        let mut data = vec![0u8; 0];
        let mut is_valid = false;

        if let Ok(ta) = typed_array.as_typed_array::<u8>() {
            data = vec![0u8; ta.len()];
            is_valid = true;
        } else if let Ok(ta) = typed_array.as_typed_array::<u16>() {
            data = vec![0u8; ta.len() * 2];
            is_valid = true;
        } else if let Ok(ta) = typed_array.as_typed_array::<u32>() {
            data = vec![0u8; ta.len() * 4];
            is_valid = true;
        } else if let Ok(ta) = typed_array.as_typed_array::<i8>() {
             data = vec![0u8; ta.len()];
             is_valid = true;
        } else if let Ok(ta) = typed_array.as_typed_array::<i16>() {
            data = vec![0u8; ta.len() * 2];
            is_valid = true;
        } else if let Ok(ta) = typed_array.as_typed_array::<i32>() {
            data = vec![0u8; ta.len() * 4];
            is_valid = true;
        }

        if !is_valid {
            return Err(rquickjs::Error::new_from_js("Crypto", "Unsupported TypedArray type"));
        }

        if let Err(e) = getrandom::getrandom(&mut data) {
            return Err(rquickjs::Error::new_from_js("Crypto", &format!("Random generation error: {}", e)));
        }

        // Copy back to TypedArray
        if let Ok(mut ta) = typed_array.as_typed_array::<u8>() {
            ta.as_mut().copy_from_slice(&data);
        } else if let Ok(mut ta) = typed_array.as_typed_array::<u16>() {
             let slice: &mut [u16] = ta.as_mut();
             for i in 0..slice.len() {
                 slice[i] = u16::from_ne_bytes([data[i*2], data[i*2+1]]);
             }
        } else if let Ok(mut ta) = typed_array.as_typed_array::<u32>() {
             let slice: &mut [u32] = ta.as_mut();
             for i in 0..slice.len() {
                 slice[i] = u32::from_ne_bytes([data[i*4], data[i*4+1], data[i*4+2], data[i*4+3]]);
             }
        } else if let Ok(mut ta) = typed_array.as_typed_array::<i8>() {
            let slice: &mut [i8] = ta.as_mut();
            for i in 0..slice.len() {
                slice[i] = data[i] as i8;
            }
        } else if let Ok(mut ta) = typed_array.as_typed_array::<i16>() {
            let slice: &mut [i16] = ta.as_mut();
            for i in 0..slice.len() {
                slice[i] = i16::from_ne_bytes([data[i*2], data[i*2+1]]);
            }
        } else if let Ok(mut ta) = typed_array.as_typed_array::<i32>() {
            let slice: &mut [i32] = ta.as_mut();
            for i in 0..slice.len() {
                slice[i] = i32::from_ne_bytes([data[i*4], data[i*4+1], data[i*4+2], data[i*4+3]]);
            }
        }

        Ok(typed_array)
    }

    #[qjs(get)]
    pub fn subtle<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let subtle = SubtleCrypto::new();
        let instance = Class::instance(ctx, subtle)?;
        Ok(instance.into_value())
    }
}

pub fn register(rt: &JsRuntime) -> rquickjs::Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            let globals = ctx.globals();
            Class::<SubtleCrypto>::register(ctx.clone())?;
            Class::<Crypto>::register(ctx.clone())?;
            
            let crypto = Class::instance(ctx.clone(), Crypto::new())?;
            globals.set("crypto", crypto)?;
            
            Ok(())
        })
    })
}
