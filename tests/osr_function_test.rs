use albedo::ace::runtime::core::runtime::JsRuntime;
use albedo_jit::decoder::qjs_opcodes::QjsOpcode;
use albedo_jit::{
    BytecodeRegistry, FunctionId, JitBridge, JitProfiler, ProfilerConfig, QjsBytecodeFunction,
    StackToRegisterTranslator,
};
use std::sync::Arc;
use std::time::Instant;

#[tokio::test]
#[ignore = "OSR block_to_qjs_offset mapping needs fixing after JIT refactor"]
async fn test_osr_named_function_loop() {
    // 1. Setup JIT e Runtime
    let profiler = Arc::new(JitProfiler::new(ProfilerConfig { hot_threshold: 5 }));
    let bridge = Arc::new(JitBridge::new(Arc::clone(&profiler)).unwrap());
    let registry = Arc::new(BytecodeRegistry::new());

    let mut runtime = JsRuntime::new_with_jit(
        Arc::clone(&bridge),
        Arc::clone(&profiler),
        Arc::clone(&registry),
    )
    .unwrap();

    // 2. Definir uma função "sum(n)" com loop longo em AIR manual (simulando decodificação)
    // function sum(n) { let s = 0; for(let i=0; i<n; i++) { s += i; } return s; }
    let func_name = "sum_func";
    let id = FunctionId(func_name.to_string());

    // Opcodes QJS simulados (simplificados)
    let qjs_func = QjsBytecodeFunction {
        name: func_name.to_string(),
        num_args: 1,
        num_locals: 2, // s=local[0], i=local[1]
        opcodes: vec![
            QjsOpcode::PushI32(0), // 0: s = 0
            QjsOpcode::PutLoc(0),
            QjsOpcode::PushI32(0), // 2: i = 0
            QjsOpcode::PutLoc(1),
            // Loop Header
            QjsOpcode::GetLoc(1),  // 4: i
            QjsOpcode::GetArg(0),  // 5: n
            QjsOpcode::Lt,         // 6: i < n
            QjsOpcode::IfFalse(6), // 7: exit loop (offset 6 -> alvo index 14)
            // Loop Body
            QjsOpcode::GetLoc(0), // 8: s
            QjsOpcode::GetLoc(1), // 9: i
            QjsOpcode::Add,       // 10: s + i
            QjsOpcode::PutLoc(0), // 11: s = s + i
            // Increment
            QjsOpcode::GetLoc(1),  // 12: i
            QjsOpcode::PushI32(1), // 13: 1
            QjsOpcode::Add,        // 14: i + 1
            QjsOpcode::PutLoc(1),  // 15: i = i + 1
            QjsOpcode::Goto(-13), // 16: back to index 4
            // Exit
            QjsOpcode::GetLoc(0), // 17
            QjsOpcode::Return,    // 18
        ],
        constant_pool_strings: vec![],
    };

    registry.register(id.clone(), qjs_func);

    println!("[Test] Iniciando execução com OSR para função nomeada...");
    let start = Instant::now();

    let qjs_func_reg = registry.get(&id).unwrap();
    let translator = StackToRegisterTranslator::new(&qjs_func_reg);
    let (air, map) = translator.translate(qjs_func_reg);

    // O loop header deve estar no offset 4.
    assert!(map.block_to_qjs_offset.values().any(|&v| v == 4));

    let target_block = map
        .block_to_qjs_offset
        .iter()
        .find(|(_, &v)| v == 4)
        .map(|(&k, _)| k)
        .expect("Loop header não mapeado");

    // Tentar compilar OSR para esse ponto
    let ptr = bridge
        .try_osr(&id, 4, &registry)
        .expect("OSR compile falhou");
    assert!(!ptr.is_null());

    println!("[Test] OSR Compilado com sucesso. Entry point: {:p}", ptr);
    println!("[Test] Tempo total: {:?}", start.elapsed());
}
