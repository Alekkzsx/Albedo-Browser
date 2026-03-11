//! # Builtins Rápidos
//!
//! Implementações nativas para Math.*, Array.push/pop, String.charAt, JSON.parse.

use crate::js_value::JsValue;
use crate::object_model;

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
}

pub fn call_builtin(id: BuiltinId, args: &[JsValue]) -> JsValue {
    match id {
        BuiltinId::MathAbs => JsValue::float64(to_number(arg(args, 0)).abs()),
        BuiltinId::MathAcos => JsValue::float64(to_number(arg(args, 0)).acos()),
        BuiltinId::MathAcosh => JsValue::float64(to_number(arg(args, 0)).acosh()),
        BuiltinId::MathAsin => JsValue::float64(to_number(arg(args, 0)).asin()),
        BuiltinId::MathAsinh => JsValue::float64(to_number(arg(args, 0)).asinh()),
        BuiltinId::MathAtan => JsValue::float64(to_number(arg(args, 0)).atan()),
        BuiltinId::MathAtan2 => JsValue::float64(to_number(arg(args, 0)).atan2(to_number(arg(args, 1)))),
        BuiltinId::MathAtanh => JsValue::float64(to_number(arg(args, 0)).atanh()),
        BuiltinId::MathCbrt => JsValue::float64(to_number(arg(args, 0)).cbrt()),
        BuiltinId::MathCeil => JsValue::float64(to_number(arg(args, 0)).ceil()),
        BuiltinId::MathClz32 => {
            let v = to_int32(arg(args, 0)) as u32;
            JsValue::int32(v.leading_zeros() as i32)
        }
        BuiltinId::MathCos => JsValue::float64(to_number(arg(args, 0)).cos()),
        BuiltinId::MathCosh => JsValue::float64(to_number(arg(args, 0)).cosh()),
        BuiltinId::MathExp => JsValue::float64(to_number(arg(args, 0)).exp()),
        BuiltinId::MathExpm1 => JsValue::float64(to_number(arg(args, 0)).exp_m1()),
        BuiltinId::MathFloor => JsValue::float64(to_number(arg(args, 0)).floor()),
        BuiltinId::MathFround => JsValue::float64((to_number(arg(args, 0)) as f32) as f64),
        BuiltinId::MathHypot => {
            let mut sum = 0.0;
            for v in args {
                let n = to_number(*v);
                sum += n * n;
            }
            JsValue::float64(sum.sqrt())
        }
        BuiltinId::MathImul => {
            let a = to_int32(arg(args, 0)) as i32;
            let b = to_int32(arg(args, 1)) as i32;
            JsValue::int32(a.wrapping_mul(b))
        }
        BuiltinId::MathLog => JsValue::float64(to_number(arg(args, 0)).ln()),
        BuiltinId::MathLog1p => JsValue::float64(to_number(arg(args, 0)).ln_1p()),
        BuiltinId::MathLog10 => JsValue::float64(to_number(arg(args, 0)).log10()),
        BuiltinId::MathLog2 => JsValue::float64(to_number(arg(args, 0)).log2()),
        BuiltinId::MathMax => {
            if args.is_empty() {
                JsValue::float64(f64::NEG_INFINITY)
            } else {
                let mut m = f64::NEG_INFINITY;
                for v in args {
                    let n = to_number(*v);
                    if n > m { m = n; }
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
                    let n = to_number(*v);
                    if n < m { m = n; }
                }
                JsValue::float64(m)
            }
        }
        BuiltinId::MathPow => JsValue::float64(to_number(arg(args, 0)).powf(to_number(arg(args, 1)))),
        BuiltinId::MathRandom => {
            JsValue::float64(rand::random::<f64>())
        }
        BuiltinId::MathRound => {
            let x = to_number(arg(args, 0));
            let r = if x >= 0.0 {
                (x + 0.5).floor()
            } else {
                (x - 0.5).ceil()
            };
            JsValue::float64(r)
        }
        BuiltinId::MathSign => {
            let x = to_number(arg(args, 0));
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
        BuiltinId::MathSin => JsValue::float64(to_number(arg(args, 0)).sin()),
        BuiltinId::MathSinh => JsValue::float64(to_number(arg(args, 0)).sinh()),
        BuiltinId::MathSqrt => JsValue::float64(to_number(arg(args, 0)).sqrt()),
        BuiltinId::MathTan => JsValue::float64(to_number(arg(args, 0)).tan()),
        BuiltinId::MathTanh => JsValue::float64(to_number(arg(args, 0)).tanh()),
        BuiltinId::MathTrunc => {
            let x = to_number(arg(args, 0));
            let r = if x >= 0.0 { x.floor() } else { x.ceil() };
            JsValue::float64(r)
        }

        BuiltinId::ArrayPush => {
            if args.is_empty() { return JsValue::undefined(); }
            let arr = args[0];
            let items = &args[1..];
            object_model::array_push(arr, items)
        }
        BuiltinId::ArrayPop => {
            if args.is_empty() { return JsValue::undefined(); }
            object_model::array_pop(args[0])
        }
        BuiltinId::StringCharAt => {
            let s = arg(args, 0);
            let idx = to_int32(arg(args, 1)) as usize;
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

fn to_number(v: JsValue) -> f64 {
    if v.is_int32() {
        v.as_int32() as f64
    } else if v.is_float64() {
        v.as_float64()
    } else if v.is_bool() {
        if v.as_bool() { 1.0 } else { 0.0 }
    } else if v.is_null() {
        0.0
    } else {
        f64::NAN
    }
}

fn to_int32(v: JsValue) -> i32 {
    if v.is_int32() {
        v.as_int32()
    } else {
        to_number(v) as i32
    }
}
