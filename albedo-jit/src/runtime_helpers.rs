//! # Runtime Helpers
//!
//! Funções `extern "C"` chamáveis pelo código JIT nativo para operações
//! dinâmicas. No Tier 1, todas as operações do AlbedoJIT invocam essas
//! funções helper, já que não temos inferência de tipos.
//!
//! Apenas quando formos para o Tier 2 substituiremos as chamadas CALL
//! por instruções CPU nativas (FADD, IADD) via Type Specialization.

use crate::js_value::{JsValue, TAG_MASK};
use crate::type_feedback::TypeFeedbackRegistry;
use crate::builtins::{BuiltinId, call_builtin};
use crate::object_model;

// ---------------------------------------------------------------------------
// Helpers Aritméticos
// ---------------------------------------------------------------------------

/// Adição JS (`a + b`). Coerção de número simples (Strings ainda não tratadas nesta V1).
#[no_mangle]
pub extern "C" fn js_add(lhs: u64, rhs: u64) -> u64 {
    let a = JsValue(lhs);
    let b = JsValue(rhs);

    // Fast-path: ambos são inteiros 32 bits
    if a.is_int32() && b.is_int32() {
        let n1 = a.as_int32() as i64;
        let n2 = b.as_int32() as i64;
        let p = n1 + n2;

        // Se passar do range de um i32, promove para Float64 (IEEE 754)
        if p >= (i32::MIN as i64) && p <= (i32::MAX as i64) {
            return JsValue::int32(p as i32).0;
        } else {
            return JsValue::float64(p as f64).0;
        }
    }

    // Slow-path: Converte ambos para float64 (na V1 pularemos ToPrimitive string)
    let f1 = to_number(a);
    let f2 = to_number(b);
    JsValue::float64(f1 + f2).0
}

/// Adição JS com coleta de type feedback (IC slot).
#[no_mangle]
pub extern "C" fn js_add_ic(lhs: u64, rhs: u64, slot: u64) -> u64 {
    TypeFeedbackRegistry::record_add(slot as u32, JsValue(lhs), JsValue(rhs));
    js_add(lhs, rhs)
}

/// Subtração JS (`a - b`).
#[no_mangle]
pub extern "C" fn js_sub(lhs: u64, rhs: u64) -> u64 {
    let a = JsValue(lhs);
    let b = JsValue(rhs);

    if a.is_int32() && b.is_int32() {
        let p = (a.as_int32() as i64) - (b.as_int32() as i64);
        if p >= (i32::MIN as i64) && p <= (i32::MAX as i64) {
            return JsValue::int32(p as i32).0;
        } else {
            return JsValue::float64(p as f64).0;
        }
    }

    let f1 = to_number(a);
    let f2 = to_number(b);
    JsValue::float64(f1 - f2).0
}

/// Multiplicação JS (`a * b`).
#[no_mangle]
pub extern "C" fn js_mul(lhs: u64, rhs: u64) -> u64 {
    let a = JsValue(lhs);
    let b = JsValue(rhs);

    if a.is_int32() && b.is_int32() {
        let p = (a.as_int32() as i64) * (b.as_int32() as i64);
        if p >= (i32::MIN as i64) && p <= (i32::MAX as i64) {
            return JsValue::int32(p as i32).0;
        }
    }

    let f1 = to_number(a);
    let f2 = to_number(b);
    JsValue::float64(f1 * f2).0
}

/// Comparação Abstrata (==)
#[no_mangle]
pub extern "C" fn js_eq(lhs: u64, rhs: u64) -> u64 {
    let a = JsValue(lhs);
    let b = JsValue(rhs);

    // Se tags forem iguais, comparamos os bits (exceto Float64)
    if (a.0 & TAG_MASK) == (b.0 & TAG_MASK) && !a.is_float64() {
        return JsValue::bool(lhs == rhs).0;
    }

    // Coerção simples numérica (V1)
    let f1 = to_number(a);
    let f2 = to_number(b);
    JsValue::bool(f1 == f2).0
}

/// Comparação Menor Que (<)
#[no_mangle]
pub extern "C" fn js_lt(lhs: u64, rhs: u64) -> u64 {
    let a = JsValue(lhs);
    let b = JsValue(rhs);

    if a.is_int32() && b.is_int32() {
        return JsValue::bool(a.as_int32() < b.as_int32()).0;
    }

    let f1 = to_number(a);
    let f2 = to_number(b);
    JsValue::bool(f1 < f2).0
}

// ---------------------------------------------------------------------------
// Helpers de Resolução Estrutural / Lógica
// ---------------------------------------------------------------------------

/// Comparação Estrita (===) sem coerção
#[no_mangle]
pub extern "C" fn js_strict_eq(lhs: u64, rhs: u64) -> u64 {
    let a = JsValue(lhs);
    let b = JsValue(rhs);

    if a.is_float64() && b.is_float64() {
        // NaNs nunca são iguais a nada
        if a.as_float64().is_nan() || b.as_float64().is_nan() {
            return JsValue::bool(false).0;
        }
        return JsValue::bool(a.as_float64() == b.as_float64()).0;
    }

    // Se ambos forem omesmo Tag (Int32, Bool, Null), os bits serão iguais
    JsValue::bool(lhs == rhs).0
}

/// GetProp com IC slot (modelo de objeto mínimo).
#[no_mangle]
pub extern "C" fn js_get_prop_ic(obj: u64, prop: u64, slot: u64) -> u64 {
    let o = JsValue(obj);
    let p = JsValue(prop);
    TypeFeedbackRegistry::record_get_prop(slot as u32, o, p);
    object_model::get_prop(o, p).0
}

/// Call com IC slot (agora despacha builtins rápidos).
#[no_mangle]
pub extern "C" fn js_call_ic(func: u64, args_ptr: u64, num_args: u64, slot: u64) -> u64 {
    let f = JsValue(func);
    TypeFeedbackRegistry::record_call(slot as u32, f);
    if f.is_builtin() {
        let id = f.as_builtin_id() as u32;
        if let Some(bid) = builtin_from_id(id) {
            let args = unsafe { std::slice::from_raw_parts(args_ptr as *const u64, num_args as usize) };
            let vals: Vec<JsValue> = args.iter().map(|v| JsValue(*v)).collect();
            return call_builtin(bid, &vals).0;
        }
    }
    JsValue::undefined().0
}

fn builtin_from_id(id: u32) -> Option<BuiltinId> {
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

// ---------------------------------------------------------------------------
// Helpers de Objetos (Hidden Classes)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn js_create_obj() -> u64 {
    object_model::alloc_object().0
}

#[no_mangle]
pub extern "C" fn js_create_array() -> u64 {
    object_model::alloc_array().0
}

#[no_mangle]
pub extern "C" fn js_set_prop(obj: u64, prop: u64, value: u64) -> u64 {
    let o = JsValue(obj);
    let p = JsValue(prop);
    let v = JsValue(value);
    object_model::set_prop(o, p, v);
    value
}

/// Helper para JumpIf (conversão para valor Booleano Real) ToBoolean
#[no_mangle]
pub extern "C" fn js_to_bool(val: u64) -> u64 {
    let v = JsValue(val);
    if v.is_bool() {
        return v.0;
    }
    if v.is_int32() {
        return JsValue::bool(v.as_int32() != 0).0;
    }
    if v.is_float64() {
        let f = v.as_float64();
        // Zero, -Zero ou NaN são falso
        return JsValue::bool(!f.is_nan() && f != 0.0).0;
    }
    if v.is_undefined() || v.is_null() {
        return JsValue::bool(false).0;
    }
    
    // Objetos/Strings não vazias = true
    JsValue::bool(true).0
}


/// Coerção abstrata simples JS ToNumber
#[inline]
fn to_number(val: JsValue) -> f64 {
    if val.is_int32() {
        val.as_int32() as f64
    } else if val.is_float64() {
        val.as_float64()
    } else if val.is_bool() {
        if val.as_bool() { 1.0 } else { 0.0 }
    } else if val.is_null() {
        0.0
    } else {
        std::f64::NAN // undefined, objects, etc default for baseline
    }
}

// ---------------------------------------------------------------------------
// Testes de Sanidade Runtime
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_js_add_ints() {
        let r = js_add(JsValue::int32(10).0, JsValue::int32(20).0);
        let val = JsValue(r);
        assert!(val.is_int32());
        assert_eq!(val.as_int32(), 30);
    }

    #[test]
    fn test_runtime_js_add_overflow() {
        let r = js_add(JsValue::int32(i32::MAX).0, JsValue::int32(1).0);
        let val = JsValue(r);
        // JS Integers não devem rodar overlock, viram flotantes
        assert!(val.is_float64());
        assert_eq!(val.as_float64(), (i32::MAX as f64) + 1.0);
    }
    
    #[test]
    fn test_runtime_js_strict_eq() {
        let r1 = js_strict_eq(JsValue::int32(42).0, JsValue::int32(42).0);
        assert!(JsValue(r1).as_bool());

        let r2 = js_strict_eq(JsValue::int32(42).0, JsValue::float64(42.0).0);
        assert!(!JsValue(r2).as_bool()); 
    }

    #[test]
    fn test_runtime_js_eq() {
        // Int == Int
        assert!(JsValue(js_eq(JsValue::int32(1).0, JsValue::int32(1).0)).as_bool());
        // Int == Float
        assert!(JsValue(js_eq(JsValue::int32(1).0, JsValue::float64(1.0).0)).as_bool());
        // Bool == Int
        assert!(JsValue(js_eq(JsValue::bool(true).0, JsValue::int32(1).0)).as_bool());
    }

    #[test]
    fn test_runtime_js_lt() {
        assert!(JsValue(js_lt(JsValue::int32(1).0, JsValue::int32(5).0)).as_bool());
        assert!(!JsValue(js_lt(JsValue::int32(5).0, JsValue::int32(1).0)).as_bool());
        assert!(JsValue(js_lt(JsValue::float64(0.5).0, JsValue::int32(1).0)).as_bool());
    }
}
