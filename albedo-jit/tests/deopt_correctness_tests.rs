use albedo_jit::compiler::tier2_compiler::Tier2Compiler;
use albedo_jit::decoder::{QjsBytecodeFunction, QjsOpcode, StackToRegisterTranslator};
use albedo_jit::engine::jit_engine::AlbedoJitEngine;
use albedo_jit::runtime::js_value::JsValue;
use albedo_jit::runtime::type_feedback::{TypeFeedbackRegistry, ValueType};

#[test]
fn test_deopt_arithmetic_overflow() {
    let mut engine = AlbedoJitEngine::new().unwrap();

    // 1. Definir função: add(a, b) { return a + b; }
    let qjs_func = QjsBytecodeFunction {
        name: "test_add_overflow".into(),
        num_args: 2,
        num_locals: 0,
        opcodes: vec![
            QjsOpcode::GetArg(0),
            QjsOpcode::GetArg(1),
            QjsOpcode::Add,
            QjsOpcode::Return,
        ],
        constant_pool_strings: vec![],
    };

    // 2. Decode para AIR
    let translator = StackToRegisterTranslator::new(&qjs_func);
    let (air_func, _map) = translator.translate(qjs_func);

    // 3. Forçar feedback Monomorfico Int32 + Int32 no slot do Add
    // O Add é a instrução 2 no bytecode QJS. O tradutor aloca slots IC.
    // Vamos encontrar o slot IC no AIR.
    let mut ic_slot = 0;
    for block in &air_func.blocks {
        for inst in &block.insts {
            if let albedo_jit::bytecode::AirOpcode::Add { ic_slot: slot, .. } = inst {
                ic_slot = *slot;
            }
        }
    }

    for _ in 0..100 {
        TypeFeedbackRegistry::record_add(ic_slot, JsValue::int32(10), JsValue::int32(20));
    }

    // 4. Compilar via Tier 2 (Turbo)
    let dummy_registry = albedo_jit::engine::jit_bridge::BytecodeRegistry::new();
    let mut compiler = Tier2Compiler::new(&mut engine, &dummy_registry);
    let func_id = compiler
        .compile(&air_func)
        .expect("Falha na compilação Tier 2");
    engine.finalize_definitions().unwrap();
    let ptr = engine.get_finalized_function(func_id);

    // 5. Executar com valores que causam overflow de i32
    // i32::MAX = 2147483647
    let a = JsValue::int32(2147483647);
    let b = JsValue::int32(1);

    let native_func: extern "C" fn(u64, u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };
    let result = JsValue(native_func(JsValue::undefined().0, a.0, b.0));

    // O resultado deve ser 2147483648.0 (Float64) após o deopt
    assert!(result.is_float64());
    assert_eq!(result.as_float64(), 2147483648.0);
    println!(
        "SUCCESS: i32 + i32 overflow deopted correctly to f64 result {}",
        result.as_float64()
    );
}

#[test]
fn test_deopt_type_change() {
    let mut engine = AlbedoJitEngine::new().unwrap();

    // 1. Definir função: identity(x) { return x + 0; }
    // Usamos + 0 para forçar um IC de Add especializado.
    let qjs_func = QjsBytecodeFunction {
        name: "test_type_change".into(),
        num_args: 1,
        num_locals: 0,
        opcodes: vec![
            QjsOpcode::GetArg(0),
            QjsOpcode::PushI32(0),
            QjsOpcode::Add,
            QjsOpcode::Return,
        ],
        constant_pool_strings: vec![],
    };

    let translator = StackToRegisterTranslator::new(&qjs_func);
    let (air_func, _map) = translator.translate(qjs_func);

    let mut ic_slot = 0;
    for block in &air_func.blocks {
        for inst in &block.insts {
            if let albedo_jit::bytecode::AirOpcode::Add { ic_slot: slot, .. } = inst {
                ic_slot = *slot;
            }
        }
    }

    // Aquecer como Int32
    for _ in 0..100 {
        TypeFeedbackRegistry::record_add(ic_slot, JsValue::int32(10), JsValue::int32(0));
    }

    let dummy_registry = albedo_jit::engine::jit_bridge::BytecodeRegistry::new();
    let mut compiler = Tier2Compiler::new(&mut engine, &dummy_registry);
    let func_id = compiler.compile(&air_func).unwrap();
    engine.finalize_definitions().unwrap();
    let ptr = engine.get_finalized_function(func_id);

    // Chamar com FLOAT (deve ocorrer deopt do IADD especializado)
    let a = JsValue::float64(3.5);
    let native_func: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };
    let result = JsValue(native_func(JsValue::undefined().0, a.0));

    assert!(result.is_float64());
    assert_eq!(result.as_float64(), 3.5);
    println!("SUCCESS: type change (int -> float) deopted correctly");
}

#[test]
fn test_deopt_type_change_int_float() {
    let mut engine = AlbedoJitEngine::new().unwrap();

    // identity(x) { return x + 1; }
    let qjs_func = QjsBytecodeFunction {
        name: "test_type_change_alt".into(),
        num_args: 1,
        num_locals: 0,
        opcodes: vec![
            QjsOpcode::GetArg(0),
            QjsOpcode::PushI32(1),
            QjsOpcode::Add,
            QjsOpcode::Return,
        ],
        constant_pool_strings: vec![],
    };

    let translator = StackToRegisterTranslator::new(&qjs_func);
    let (air_func, _map) = translator.translate(qjs_func);

    let mut ic_slot = 0;
    for block in &air_func.blocks {
        for inst in &block.insts {
            if let albedo_jit::bytecode::AirOpcode::Add { ic_slot: slot, .. } = inst {
                ic_slot = *slot;
            }
        }
    }

    // Aquecer como Int32
    for _ in 0..100 {
        TypeFeedbackRegistry::record_add(ic_slot, JsValue::int32(10), JsValue::int32(1));
    }

    let dummy_registry = albedo_jit::engine::jit_bridge::BytecodeRegistry::new();
    let mut compiler = Tier2Compiler::new(&mut engine, &dummy_registry);
    let func_id = compiler.compile(&air_func).unwrap();
    engine.finalize_definitions().unwrap();
    let ptr = engine.get_finalized_function(func_id);

    // Chamar com FLOAT (3.5 + 1 = 4.5)
    let a = JsValue::float64(3.5);
    let native_func: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };
    let result = JsValue(native_func(JsValue::undefined().0, a.0));

    assert!(result.is_float64());
    assert_eq!(result.as_float64(), 4.5);
    println!("SUCCESS: type change int -> float specialized path bailout verified");
}

#[test]
fn test_deopt_type_change_string() {
    let mut engine = AlbedoJitEngine::new().unwrap();

    // 1. Definir função: concat(a, b) { return a + b; }
    let qjs_func = QjsBytecodeFunction {
        name: "test_string_deopt".into(),
        num_args: 2,
        num_locals: 0,
        opcodes: vec![
            QjsOpcode::GetArg(0),
            QjsOpcode::GetArg(1),
            QjsOpcode::Add,
            QjsOpcode::Return,
        ],
        constant_pool_strings: vec![],
    };

    let translator = StackToRegisterTranslator::new(&qjs_func);
    let (air_func, _map) = translator.translate(qjs_func);

    let mut ic_slot = 0;
    for block in &air_func.blocks {
        for inst in &block.insts {
            if let albedo_jit::bytecode::AirOpcode::Add { ic_slot: slot, .. } = inst {
                ic_slot = *slot;
            }
        }
    }

    // Aquecer como Int32 + Int32
    for _ in 0..100 {
        TypeFeedbackRegistry::record_add(ic_slot, JsValue::int32(1), JsValue::int32(2));
    }

    let dummy_registry = albedo_jit::engine::jit_bridge::BytecodeRegistry::new();
    let mut compiler = Tier2Compiler::new(&mut engine, &dummy_registry);
    let func_id = compiler.compile(&air_func).unwrap();
    engine.finalize_definitions().unwrap();
    let ptr = engine.get_finalized_function(func_id);

    // Chamar com STRING: 42 + " items" -> "42 items"
    let a = JsValue::int32(42);
    let s_id = albedo_jit::runtime::object_model::intern_string(" items".to_string());
    let b = JsValue::string(s_id as u64);

    let native_func: extern "C" fn(u64, u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };
    let result = JsValue(native_func(JsValue::undefined().0, a.0, b.0));

    assert!(result.is_string());
    let res_str =
        albedo_jit::runtime::object_model::get_string(result.as_string_id() as u32).unwrap();
    assert_eq!(res_str, "42 items");
    println!(
        "SUCCESS: type change (int -> string) deopted and concatenated correctly: '{}'",
        res_str
    );
}

#[test]
fn test_deopt_stack_reconstruction() {
    let mut engine = AlbedoJitEngine::new().unwrap();

    // 1. f(a, b, c) { x = a + b; y = x + c; return y; }
    let qjs_func = QjsBytecodeFunction {
        name: "test_stack_recon".into(),
        num_args: 3,
        num_locals: 1, // x
        opcodes: vec![
            QjsOpcode::GetArg(0), // a
            QjsOpcode::GetArg(1), // b
            QjsOpcode::Add,       // a + b
            QjsOpcode::PutLoc(0), // x = a + b
            QjsOpcode::GetLoc(0), // x
            QjsOpcode::GetArg(2), // c
            QjsOpcode::Add,       // x + c  <-- DEOPT HERE if c is string
            QjsOpcode::Return,
        ],
        constant_pool_strings: vec![],
    };

    let translator = StackToRegisterTranslator::new(&qjs_func);
    let (air_func, _map) = translator.translate(qjs_func);

    // Encontrar os slots IC para os dois Adds
    let mut slots = Vec::new();
    for block in &air_func.blocks {
        for inst in &block.insts {
            if let albedo_jit::bytecode::AirOpcode::Add { ic_slot, .. } = inst {
                slots.push(*ic_slot);
            }
        }
    }
    assert_eq!(slots.len(), 2);

    // Aquecer ambos como Int32 + Int32
    for &slot in &slots {
        for _ in 0..100 {
            TypeFeedbackRegistry::record_add(slot, JsValue::int32(1), JsValue::int32(1));
        }
    }

    let dummy_registry = albedo_jit::engine::jit_bridge::BytecodeRegistry::new();
    let mut compiler = Tier2Compiler::new(&mut engine, &dummy_registry);
    let func_id = compiler.compile(&air_func).unwrap();
    engine.finalize_definitions().unwrap();
    let ptr = engine.get_finalized_function(func_id);

    // Chamar com: a=10, b=20, c=" apples"
    // x = 10 + 20 = 30 (JIT OK)
    // y = 30 + " apples" (DEOPT!)
    let a = JsValue::int32(10);
    let b = JsValue::int32(20);
    let s_id = albedo_jit::runtime::object_model::intern_string(" apples".to_string());
    let c = JsValue::string(s_id as u64);

    let native_func: extern "C" fn(u64, u64, u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };
    let result = JsValue(native_func(JsValue::undefined().0, a.0, b.0, c.0));

    assert!(result.is_string());
    let res_str =
        albedo_jit::runtime::object_model::get_string(result.as_string_id() as u32).unwrap();
    assert_eq!(res_str, "30 apples");
    println!(
        "SUCCESS: stack reconstruction verified. Result after bailout: '{}'",
        res_str
    );
}
