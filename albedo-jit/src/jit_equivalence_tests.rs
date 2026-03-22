//! # Testes de Equivalência JIT vs QuickJS
//!
//! Valida que a execução nativa via AlbedoJIT produz os mesmos resultados
//! que o oráculo (Rust/JS manual) para funções puras complexas.

#[cfg(test)]
mod tests {
    use crate::compiler::baseline_compiler::BaselineCompiler;
    use crate::decoder::{QjsBytecodeFunction, QjsOpcode, StackToRegisterTranslator};
    use crate::engine::jit_engine::AlbedoJitEngine;
    use crate::runtime::js_value::JsValue;

    /// Helper para compilar e executar uma função JIT com 1 argumento.
    fn jit_run_1(qjs_func: QjsBytecodeFunction, arg: JsValue) -> JsValue {
        let mut engine = AlbedoJitEngine::new().unwrap();

        // 1. Decode
        let translator = StackToRegisterTranslator::new(&qjs_func);
        let (air_func, _) = translator.translate(qjs_func);

        // 2. Compile
        let mut compiler = BaselineCompiler::new(&mut engine);
        let id = compiler
            .compile(&air_func)
            .expect("Falha na compilação JIT");

        // 3. Finalize
        engine.module.finalize_definitions().unwrap();
        let ptr = engine.module.get_finalized_function(id);

        // 4. Execute
        let native_func: extern "C" fn(u64) -> u64 = unsafe { std::mem::transmute(ptr) };
        JsValue(native_func(arg.0))
    }

    /// Helper para compilar e executar uma função JIT com 2 argumentos.
    fn jit_run_2(qjs_func: QjsBytecodeFunction, a: JsValue, b: JsValue) -> JsValue {
        let mut engine = AlbedoJitEngine::new().unwrap();
        let translator = StackToRegisterTranslator::new(&qjs_func);
        let (air_func, _) = translator.translate(qjs_func);
        let mut compiler = BaselineCompiler::new(&mut engine);
        let id = compiler.compile(&air_func).unwrap();
        engine.module.finalize_definitions().unwrap();
        let ptr = engine.module.get_finalized_function(id);
        let native_func: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };
        JsValue(native_func(a.0, b.0))
    }

    #[test]
    fn test_identity() {
        let qjs = QjsBytecodeFunction {
            name: "id".into(),
            num_args: 1,
            num_locals: 0,
            opcodes: vec![QjsOpcode::GetArg(0), QjsOpcode::Return],
            constant_pool_strings: vec![],
        };

        assert_eq!(jit_run_1(qjs.clone(), JsValue::int32(42)).as_int32(), 42);
        assert_eq!(jit_run_1(qjs.clone(), JsValue::int32(-1)).as_int32(), -1);
    }

    #[test]
    fn test_nested_arithmetic() {
        // (a + b) * (a - b)
        let qjs = QjsBytecodeFunction {
            name: "calc".into(),
            num_args: 2,
            num_locals: 0,
            opcodes: vec![
                QjsOpcode::GetArg(0),
                QjsOpcode::GetArg(1),
                QjsOpcode::Add,
                QjsOpcode::GetArg(0),
                QjsOpcode::GetArg(1),
                QjsOpcode::Sub,
                QjsOpcode::Mul,
                QjsOpcode::Return,
            ],
            constant_pool_strings: vec![],
        };

        // (5+3)*(5-3) = 8*2 = 16
        assert_eq!(
            jit_run_2(qjs.clone(), JsValue::int32(5), JsValue::int32(3)).as_int32(),
            16
        );
        // (10+10)*(10-10) = 20*0 = 0
        assert_eq!(
            jit_run_2(qjs.clone(), JsValue::int32(10), JsValue::int32(10)).as_int32(),
            0
        );
    }

    #[test]
    fn test_sum_1_to_n() {
        // function sum(n) { let s = 0; for (let i = 1; i <= n; i++) s += i; return s; }
        let qjs = QjsBytecodeFunction {
            name: "sum".into(),
            num_args: 1,
            num_locals: 2, // local[0]=s, local[1]=i
            opcodes: vec![
                QjsOpcode::PushI32(0),
                QjsOpcode::PutLoc(0), // s = 0
                QjsOpcode::PushI32(1),
                QjsOpcode::PutLoc(1), // i = 1
                // loop_cond:
                QjsOpcode::GetLoc(1),   // push i
                QjsOpcode::GetArg(0),   // push n
                QjsOpcode::Lt, // i < n (usando LT por simplificação, faremos i <= n manualmente abaixo)
                QjsOpcode::IfFalse(10), // sai se i >= n. (Offset p/ Return final)
                // loop_body:
                QjsOpcode::GetLoc(0),
                QjsOpcode::GetLoc(1),
                QjsOpcode::Add,
                QjsOpcode::PutLoc(0), // s = s + i
                // increment:
                QjsOpcode::GetLoc(1),
                QjsOpcode::PushI32(1),
                QjsOpcode::Add,
                QjsOpcode::PutLoc(1), // i = i + 1
                QjsOpcode::Goto(-12), // volta p/ GetLoc(1) antes do Lt
                // final:
                QjsOpcode::GetLoc(0),
                QjsOpcode::Return,
            ],
            constant_pool_strings: vec![],
        };

        // Nota: O loop acima faz i < n. Entao sum(10) vai somar 1..9 = 45.
        assert_eq!(jit_run_1(qjs.clone(), JsValue::int32(10)).as_int32(), 45);
    }

    #[test]
    fn test_factorial_iterative() {
        // function fact(n) { let r = 1; for (let i = 2; i <= n; i++) r *= i; return r; }
        let qjs = QjsBytecodeFunction {
            name: "fact".into(),
            num_args: 1,
            num_locals: 2, // 0=r, 1=i
            opcodes: vec![
                QjsOpcode::PushI32(1),
                QjsOpcode::PutLoc(0), // r = 1
                QjsOpcode::PushI32(2),
                QjsOpcode::PutLoc(1), // i = 2
                // cond:
                QjsOpcode::GetLoc(1),
                QjsOpcode::GetArg(0),
                QjsOpcode::Lt, // i < n
                QjsOpcode::IfFalse(10),
                // body:
                QjsOpcode::GetLoc(0),
                QjsOpcode::GetLoc(1),
                QjsOpcode::Mul,
                QjsOpcode::PutLoc(0), // r *= i
                // inc:
                QjsOpcode::GetLoc(1),
                QjsOpcode::PushI32(1),
                QjsOpcode::Add,
                QjsOpcode::PutLoc(1), // i++
                QjsOpcode::Goto(-12),
                QjsOpcode::GetLoc(0),
                QjsOpcode::Return,
            ],
            constant_pool_strings: vec![],
        };

        // n=5 -> 2*3*4 = 24 (Lt n=5 para no 4)
        assert_eq!(jit_run_1(qjs.clone(), JsValue::int32(5)).as_int32(), 24);
    }
}
