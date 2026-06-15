//! # Builtins Rápidos
//!
//! Implementações nativas para Math.*, Array.push/pop, String.charAt, JSON.parse.

use crate::runtime::js_value::JsValue;
use crate::runtime::object_model;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinId {
    // Math.*
    MathAbs = 1,
    MathAcos,
    MathAcosh,
    MathAsin,
    MathAsinh,
    MathAtan,
    MathAtan2,
    MathAtanh,
    MathCbrt,
    MathCeil,
    MathClz32,
    MathCos,
    MathCosh,
    MathExp,
    MathExpm1,
    MathFloor,
    MathFround,
    MathHypot,
    MathImul,
    MathLog,
    MathLog1p,
    MathLog10,
    MathLog2,
    MathMax,
    MathMin,
    MathPow,
    MathRandom,
    MathRound,
    MathSign,
    MathSin,
    MathSinh,
    MathSqrt,
    MathTan,
    MathTanh,
    MathTrunc,

    // Array.*
    ArrayPush,
    ArrayPop,

    // String.*
    StringCharAt,

    // JSON.parse
    JsonParse,
}

impl BuiltinId {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "Math.abs" => BuiltinId::MathAbs,
            "Math.acos" => BuiltinId::MathAcos,
            "Math.acosh" => BuiltinId::MathAcosh,
            "Math.asin" => BuiltinId::MathAsin,
            "Math.asinh" => BuiltinId::MathAsinh,
            "Math.atan" => BuiltinId::MathAtan,
            "Math.atan2" => BuiltinId::MathAtan2,
            "Math.atanh" => BuiltinId::MathAtanh,
            "Math.cbrt" => BuiltinId::MathCbrt,
            "Math.ceil" => BuiltinId::MathCeil,
            "Math.clz32" => BuiltinId::MathClz32,
            "Math.cos" => BuiltinId::MathCos,
            "Math.cosh" => BuiltinId::MathCosh,
            "Math.exp" => BuiltinId::MathExp,
            "Math.expm1" => BuiltinId::MathExpm1,
            "Math.floor" => BuiltinId::MathFloor,
            "Math.fround" => BuiltinId::MathFround,
            "Math.hypot" => BuiltinId::MathHypot,
            "Math.imul" => BuiltinId::MathImul,
            "Math.log" => BuiltinId::MathLog,
            "Math.log1p" => BuiltinId::MathLog1p,
            "Math.log10" => BuiltinId::MathLog10,
            "Math.log2" => BuiltinId::MathLog2,
            "Math.max" => BuiltinId::MathMax,
            "Math.min" => BuiltinId::MathMin,
            "Math.pow" => BuiltinId::MathPow,
            "Math.random" => BuiltinId::MathRandom,
            "Math.round" => BuiltinId::MathRound,
            "Math.sign" => BuiltinId::MathSign,
            "Math.sin" => BuiltinId::MathSin,
            "Math.sinh" => BuiltinId::MathSinh,
            "Math.sqrt" => BuiltinId::MathSqrt,
            "Math.tan" => BuiltinId::MathTan,
            "Math.tanh" => BuiltinId::MathTanh,
            "Math.trunc" => BuiltinId::MathTrunc,
            "Array.push" => BuiltinId::ArrayPush,
            "Array.pop" => BuiltinId::ArrayPop,
            "String.charAt" => BuiltinId::StringCharAt,
            "JSON.parse" => BuiltinId::JsonParse,
            _ => return None,
        })
    }

    pub fn from_u32(id: u32) -> Option<Self> {
        use BuiltinId::*;
        Some(match id {
            x if x == MathAbs as u32 => MathAbs,
            x if x == MathAcos as u32 => MathAcos,
            x if x == MathAcosh as u32 => MathAcosh,
            x if x == MathAsin as u32 => MathAsin,
            x if x == MathAsinh as u32 => MathAsinh,
            x if x == MathAtan as u32 => MathAtan,
            x if x == MathAtan2 as u32 => MathAtan2,
            x if x == MathAtanh as u32 => MathAtanh,
            x if x == MathCbrt as u32 => MathCbrt,
            x if x == MathCeil as u32 => MathCeil,
            x if x == MathClz32 as u32 => MathClz32,
            x if x == MathCos as u32 => MathCos,
            x if x == MathCosh as u32 => MathCosh,
            x if x == MathExp as u32 => MathExp,
            x if x == MathExpm1 as u32 => MathExpm1,
            x if x == MathFloor as u32 => MathFloor,
            x if x == MathFround as u32 => MathFround,
            x if x == MathHypot as u32 => MathHypot,
            x if x == MathImul as u32 => MathImul,
            x if x == MathLog as u32 => MathLog,
            x if x == MathLog1p as u32 => MathLog1p,
            x if x == MathLog10 as u32 => MathLog10,
            x if x == MathLog2 as u32 => MathLog2,
            x if x == MathMax as u32 => MathMax,
            x if x == MathMin as u32 => MathMin,
            x if x == MathPow as u32 => MathPow,
            x if x == MathRandom as u32 => MathRandom,
            x if x == MathRound as u32 => MathRound,
            x if x == MathSign as u32 => MathSign,
            x if x == MathSin as u32 => MathSin,
            x if x == MathSinh as u32 => MathSinh,
            x if x == MathSqrt as u32 => MathSqrt,
            x if x == MathTan as u32 => MathTan,
            x if x == MathTanh as u32 => MathTanh,
            x if x == MathTrunc as u32 => MathTrunc,
            x if x == ArrayPush as u32 => ArrayPush,
            x if x == ArrayPop as u32 => ArrayPop,
            x if x == StringCharAt as u32 => StringCharAt,
            x if x == JsonParse as u32 => JsonParse,
            _ => return None,
        })
    }
}

pub fn call_builtin(id: BuiltinId, args: &[JsValue]) -> JsValue {
    match id {
        BuiltinId::MathAbs => JsValue::float64(arg(args, 0).to_number().abs()),
        BuiltinId::MathAcos => JsValue::float64(arg(args, 0).to_number().acos()),
        BuiltinId::MathAcosh => JsValue::float64(arg(args, 0).to_number().acosh()),
        BuiltinId::MathAsin => JsValue::float64(arg(args, 0).to_number().asin()),
        BuiltinId::MathAsinh => JsValue::float64(arg(args, 0).to_number().asinh()),
        BuiltinId::MathAtan => JsValue::float64(arg(args, 0).to_number().atan()),
        BuiltinId::MathAtan2 => {
            JsValue::float64(arg(args, 0).to_number().atan2(arg(args, 1).to_number()))
        }
        BuiltinId::MathAtanh => JsValue::float64(arg(args, 0).to_number().atanh()),
        BuiltinId::MathCbrt => JsValue::float64(arg(args, 0).to_number().cbrt()),
        BuiltinId::MathCeil => JsValue::float64(arg(args, 0).to_number().ceil()),
        BuiltinId::MathClz32 => {
            let v = arg(args, 0).to_int32() as u32;
            JsValue::int32(v.leading_zeros() as i32)
        }
        BuiltinId::MathCos => JsValue::float64(arg(args, 0).to_number().cos()),
        BuiltinId::MathCosh => JsValue::float64(arg(args, 0).to_number().cosh()),
        BuiltinId::MathExp => JsValue::float64(arg(args, 0).to_number().exp()),
        BuiltinId::MathExpm1 => JsValue::float64(arg(args, 0).to_number().exp_m1()),
        BuiltinId::MathFloor => JsValue::float64(arg(args, 0).to_number().floor()),
        BuiltinId::MathFround => JsValue::float64((arg(args, 0).to_number() as f32) as f64),
        BuiltinId::MathHypot => {
            let mut sum = 0.0;
            for v in args {
                let n = v.to_number();
                sum += n * n;
            }
            JsValue::float64(sum.sqrt())
        }
        BuiltinId::MathImul => {
            let a = arg(args, 0).to_int32() as i32;
            let b = arg(args, 1).to_int32() as i32;
            JsValue::int32(a.wrapping_mul(b))
        }
        BuiltinId::MathLog => JsValue::float64(arg(args, 0).to_number().ln()),
        BuiltinId::MathLog1p => JsValue::float64(arg(args, 0).to_number().ln_1p()),
        BuiltinId::MathLog10 => JsValue::float64(arg(args, 0).to_number().log10()),
        BuiltinId::MathLog2 => JsValue::float64(arg(args, 0).to_number().log2()),
        BuiltinId::MathMax => {
            if args.is_empty() {
                JsValue::float64(f64::NEG_INFINITY)
            } else {
                let mut m = f64::NEG_INFINITY;
                for v in args {
                    let n = v.to_number();
                    if n > m {
                        m = n;
                    }
                }
                JsValue::float64(m)
            }
        }
        BuiltinId::MathMin => {
            if args.is_empty() {
                JsValue::float64(f64::INFINITY)
            } else {
                let mut m = f64::INFINITY;
                for v in args {
                    let n = v.to_number();
                    if n < m {
                        m = n;
                    }
                }
                JsValue::float64(m)
            }
        }
        BuiltinId::MathPow => {
            JsValue::float64(arg(args, 0).to_number().powf(arg(args, 1).to_number()))
        }
        BuiltinId::MathRandom => JsValue::float64(crate::runtime::random::next_f64()),
        BuiltinId::MathRound => {
            let x = arg(args, 0).to_number();
            let r = if x >= 0.0 {
                (x + 0.5).floor()
            } else {
                (x - 0.5).ceil()
            };
            JsValue::float64(r)
        }
        BuiltinId::MathSign => {
            let x = arg(args, 0).to_number();
            if x.is_nan() {
                JsValue::float64(f64::NAN)
            } else if x == 0.0 {
                JsValue::float64(x)
            } else if x > 0.0 {
                JsValue::float64(1.0)
            } else {
                JsValue::float64(-1.0)
            }
        }
        BuiltinId::MathSin => JsValue::float64(arg(args, 0).to_number().sin()),
        BuiltinId::MathSinh => JsValue::float64(arg(args, 0).to_number().sinh()),
        BuiltinId::MathSqrt => JsValue::float64(arg(args, 0).to_number().sqrt()),
        BuiltinId::MathTan => JsValue::float64(arg(args, 0).to_number().tan()),
        BuiltinId::MathTanh => JsValue::float64(arg(args, 0).to_number().tanh()),
        BuiltinId::MathTrunc => {
            let x = arg(args, 0).to_number();
            let r = if x >= 0.0 { x.floor() } else { x.ceil() };
            JsValue::float64(r)
        }

        BuiltinId::ArrayPush => {
            if args.is_empty() {
                return JsValue::undefined();
            }
            let arr = args[0];
            let items = &args[1..];
            object_model::array_push(arr, items)
        }
        BuiltinId::ArrayPop => {
            if args.is_empty() {
                return JsValue::undefined();
            }
            object_model::array_pop(args[0])
        }
        BuiltinId::StringCharAt => {
            let s = arg(args, 0);
            let idx = arg(args, 1).to_int32() as usize;
            object_model::string_char_at(s, idx)
        }
        BuiltinId::JsonParse => {
            let s = arg(args, 0);
            object_model::json_parse(s)
        }
    }
}

fn arg(args: &[JsValue], idx: usize) -> JsValue {
    args.get(idx).copied().unwrap_or_else(JsValue::undefined)
}
