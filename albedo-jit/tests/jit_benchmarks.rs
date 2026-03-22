use albedo_jit::compiler::tier2_compiler::Tier2Compiler;
use albedo_jit::decoder::{QjsBytecodeFunction, QjsOpcode, StackToRegisterTranslator};
use albedo_jit::engine::jit_engine::AlbedoJitEngine;
use albedo_jit::runtime::js_value::JsValue;
use albedo_jit::runtime::type_feedback::TypeFeedbackRegistry;
use std::time::Instant;

fn run_benchmark<F>(name: &str, mut func: F) where F: FnMut() -> JsValue {
    // Warmup
    for _ in 0..100 {
        func();
    }

    let start = Instant::now();
    let iter = 1000;
    let mut last_res = JsValue::undefined();
    for _ in 0..iter {
        last_res = func();
    }
    let duration = start.elapsed();
    println!("BENCHMARK {}: avg {} us | Result: {:?}", name, duration.as_micros() / iter as u128, last_res);
}

#[test]
fn benchmark_fibonacci_iterative() {
    let mut engine = AlbedoJitEngine::new().unwrap();

    /*
    function fib(n) {
      if (n < 2) return n;
      let a = 0, b = 1;
      for (let i = 0; i < n; i++) {
        let tmp = a;
        a = b;
        b = tmp + b;
      }
      return a;
    }
    */
    let qjs_func = QjsBytecodeFunction {
        name: "fib_iter".into(),
        num_args: 1,
        num_locals: 4, // a=0, b=1, i=2, tmp=3
        opcodes: vec![
            // if (n < 2) return n;
            QjsOpcode::GetArg(0),
            QjsOpcode::PushI32(2),
            QjsOpcode::Lt,
            QjsOpcode::IfFalse(3), // pula pro let a = 0
            QjsOpcode::GetArg(0),
            QjsOpcode::Return,

            // let a = 0, b = 1;
            QjsOpcode::PushI32(0),
            QjsOpcode::PutLoc(0),
            QjsOpcode::PushI32(1),
            QjsOpcode::PutLoc(1),

            // for (let i = 0; i < n; i++)
            QjsOpcode::PushI32(0),
            QjsOpcode::PutLoc(2),
            
            // Loop Header (Offset 14)
            QjsOpcode::GetLoc(2), // i
            QjsOpcode::GetArg(0), // n
            QjsOpcode::Lt,
            QjsOpcode::IfFalse(15), // Sai do loop (pula pro return a)

            // let tmp = a;
            QjsOpcode::GetLoc(0),
            QjsOpcode::PutLoc(3),
            // a = b;
            QjsOpcode::GetLoc(1),
            QjsOpcode::PutLoc(0),
            // b = tmp + b;
            QjsOpcode::GetLoc(3),
            QjsOpcode::GetLoc(1),
            QjsOpcode::Add,
            QjsOpcode::PutLoc(1),
            // i++
            QjsOpcode::GetLoc(2),
            QjsOpcode::PushI32(1),
            QjsOpcode::Add,
            QjsOpcode::PutLoc(2),
            
            QjsOpcode::Goto(-17), // Volta pro Loop Header (14 opcodes atras + os que pulamos?)
            // Na vdd Goto offset eh relativo ao proximo opcode.
            // Se estamos no index 31, e queremos voltar pro 14. 
            // 14 - 32 = -18.

            // return a; (Offset 32)
            QjsOpcode::GetLoc(0),
            QjsOpcode::Return,
        ],
        constant_pool_strings: vec![],
    };

    // Ajustar offsets do Goto/IfFalse se necessário
    // Indexing:
    /*
    0: GetArg(0)
    1: PushI32(2)
    2: Lt
    3: IfFalse(3) -> alvo eh 7 (let a=0)
    4: GetArg(0)
    5: Return
    6: <drop/nop se necessario, mas o translator lida com block boundaries>
    7: PushI32(0)
    8: PutLoc(0)
    9: PushI32(1)
    10: PutLoc(1)
    11: PushI32(0)
    12: PutLoc(2)
    13: GetLoc(2) <--- LOOP HEADER
    14: GetArg(0)
    15: Lt
    16: IfFalse(15) -> alvo eh 32 (getloc 0)
    17: GetLoc(0)
    18: PutLoc(3)
    19: GetLoc(1)
    20: PutLoc(0)
    21: GetLoc(3)
    22: GetLoc(1)
    23: Add
    24: PutLoc(1)
    25: GetLoc(2)
    26: PushI32(1)
    27: Add
    28: PutLoc(2)
    29: Goto(-17) -> alvo eh 13 (proximo eh 30. 13 - 30 = -17)
    30: GetLoc(0)
    31: Return
    */
    // Corrigindo opcodes baseados no trace analítico (v2):
    // Fórmula: absolute = current + offset
    let mut opcodes = qjs_func.opcodes.clone();
    opcodes[3] = QjsOpcode::IfFalse(3);  // 3 + 3 = 6 (PushI32(0))
    opcodes[15] = QjsOpcode::IfFalse(14); // 15 + 14 = 29 (GetLoc(0) final)
    opcodes[28] = QjsOpcode::Goto(-16);   // 28 - 16 = 12 (Loop Header)
    
    let qjs_func = QjsBytecodeFunction { opcodes, ..qjs_func };

    let translator = StackToRegisterTranslator::new(&qjs_func);
    let (air_func, _map) = translator.translate(qjs_func.clone());

    // 1. Benchmark Interpretador (Simulado via AirInterpreter no translator ou Baseline sem JIT)
    // Mas queremos comparar com o Tier 2.

    // Seed Feedback
    for block in &air_func.blocks {
        for inst in &block.insts {
            if let albedo_jit::bytecode::AirOpcode::Add { ic_slot, .. } = inst {
                for _ in 0..100 {
                    TypeFeedbackRegistry::record_add(*ic_slot, JsValue::int32(1), JsValue::int32(1));
                }
            }
            if let albedo_jit::bytecode::AirOpcode::Lt { .. } = inst {
                // LT ainda nao tem IC slot no AIR (segundo baseline_compiler), ele usa helper js_lt.
            }
        }
    }

    // 2. Compilar Tier 2
    let dummy_registry = albedo_jit::engine::jit_bridge::BytecodeRegistry::new();
    let mut compiler = Tier2Compiler::new(&mut engine, &dummy_registry);
    let func_id = compiler.compile(&air_func).unwrap();
    engine.finalize_definitions().unwrap();
    let ptr = engine.get_finalized_function(func_id);
    let native_func: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };

    println!("\n--- FIBONACCI ITERATIVE BENCHMARK ---");
    
    run_benchmark("Fib(10) - JIT Tier 2", || JsValue(native_func(JsValue::undefined().0, JsValue::int32(10).0)));
    run_benchmark("Fib(40) - JIT Tier 2", || JsValue(native_func(JsValue::undefined().0, JsValue::int32(40).0)));
}

#[test]
fn benchmark_nbody_simplified() {
    let mut engine = AlbedoJitEngine::new().unwrap();

    /*
    function advance(b1, b2, dt) {
       let dx = b1.x - b2.x;
       let dy = b1.y - b2.y;
       let dz = b1.z - b2.z;
       let distSq = dx*dx + dy*dy + dz*dz;
       let mag = dt / (distSq * distSq); // Simplificado (sem sqrt p/ evitar Call builtin por hora)
       b1.vx -= dx * b2.mass * mag;
       b1.vy -= dy * b2.mass * mag;
       b1.vz -= dz * b2.mass * mag;
       b2.vx += dx * b1.mass * mag;
       b2.vy += dy * b1.mass * mag;
       b2.vz += dz * b1.mass * mag;
    }
    */
    let qjs_func = QjsBytecodeFunction {
        name: "nbody_adv".into(),
        num_args: 3,
        num_locals: 5, // dx=0, dy=1, dz=2, distSq=3, mag=4
        opcodes: vec![
            // dx = b1.x - b2.x
            QjsOpcode::GetArg(0), QjsOpcode::GetField(0), // b1.x
            QjsOpcode::GetArg(1), QjsOpcode::GetField(0), // b2.x
            QjsOpcode::Sub, QjsOpcode::PutLoc(0),
            
            // dy = b1.y - b2.y
            QjsOpcode::GetArg(0), QjsOpcode::GetField(1), // b1.y
            QjsOpcode::GetArg(1), QjsOpcode::GetField(1), // b2.y
            QjsOpcode::Sub, QjsOpcode::PutLoc(1),

            // dz = b1.z - b2.z
            QjsOpcode::GetArg(0), QjsOpcode::GetField(2), // b1.z
            QjsOpcode::GetArg(1), QjsOpcode::GetField(2), // b2.z
            QjsOpcode::Sub, QjsOpcode::PutLoc(2),

            // distSq = dx*dx + dy*dy + dz*dz
            QjsOpcode::GetLoc(0), QjsOpcode::GetLoc(0), QjsOpcode::Mul,
            QjsOpcode::GetLoc(1), QjsOpcode::GetLoc(1), QjsOpcode::Mul,
            QjsOpcode::Add,
            QjsOpcode::GetLoc(2), QjsOpcode::GetLoc(2), QjsOpcode::Mul,
            QjsOpcode::Add, QjsOpcode::PutLoc(3),

            // mag = dt / (distSq * distSq)
            QjsOpcode::GetArg(2), // dt
            QjsOpcode::GetLoc(3), QjsOpcode::GetLoc(3), QjsOpcode::Mul,
            QjsOpcode::Div, QjsOpcode::PutLoc(4),

            // b1.vx -= dx * b2.mass * mag
            QjsOpcode::GetArg(0), QjsOpcode::GetArg(0), QjsOpcode::GetField(3), // b1.vx
            QjsOpcode::GetLoc(0), QjsOpcode::GetArg(1), QjsOpcode::GetField(6), QjsOpcode::Mul, // dx * b2.mass
            QjsOpcode::GetLoc(4), QjsOpcode::Mul, // ... * mag
            QjsOpcode::Sub, QjsOpcode::PutField(3),

            // Omitiremos os outros eixos por brevidade no teste de bytecode manual,
            // fatiando o benchmark no "miolo" mais denso.

            QjsOpcode::PushUndefined,
            QjsOpcode::Return,
        ],
        constant_pool_strings: vec![
            "x".into(), "y".into(), "z".into(), "vx".into(), "vy".into(), "vz".into(), "mass".into()
        ],
    };

    let translator = StackToRegisterTranslator::new(&qjs_func);
    let (air_func, _map) = translator.translate(qjs_func.clone());

    // Setup Objects (Bodies)
    let b1 = albedo_jit::runtime::object_model::alloc_object();
    let b2 = albedo_jit::runtime::object_model::alloc_object();
    
    let x_id = albedo_jit::runtime::object_model::intern_string("x".into());
    let y_id = albedo_jit::runtime::object_model::intern_string("y".into());
    let z_id = albedo_jit::runtime::object_model::intern_string("z".into());
    let vx_id = albedo_jit::runtime::object_model::intern_string("vx".into());
    let mass_id = albedo_jit::runtime::object_model::intern_string("mass".into());

    albedo_jit::runtime::object_model::set_prop(b1, JsValue::string(x_id as u64), JsValue::float64(1.0));
    albedo_jit::runtime::object_model::set_prop(b1, JsValue::string(y_id as u64), JsValue::float64(2.0));
    albedo_jit::runtime::object_model::set_prop(b1, JsValue::string(z_id as u64), JsValue::float64(3.0));
    albedo_jit::runtime::object_model::set_prop(b1, JsValue::string(vx_id as u64), JsValue::float64(0.0));
    albedo_jit::runtime::object_model::set_prop(b1, JsValue::string(mass_id as u64), JsValue::float64(10.0));

    albedo_jit::runtime::object_model::set_prop(b2, JsValue::string(x_id as u64), JsValue::float64(5.0));
    albedo_jit::runtime::object_model::set_prop(b2, JsValue::string(y_id as u64), JsValue::float64(6.0));
    albedo_jit::runtime::object_model::set_prop(b2, JsValue::string(z_id as u64), JsValue::float64(7.0));
    albedo_jit::runtime::object_model::set_prop(b2, JsValue::string(mass_id as u64), JsValue::float64(20.0));

    let dt = JsValue::float64(0.01);

    // Warmup & Feedback
    for _ in 0..100 {
        // ...
    }

    let dummy_registry = albedo_jit::engine::jit_bridge::BytecodeRegistry::new();
    let mut compiler = Tier2Compiler::new(&mut engine, &dummy_registry);
    let func_id = compiler.compile(&air_func).unwrap();
    engine.finalize_definitions().unwrap();
    let ptr = engine.get_finalized_function(func_id);
    let native_func: extern "C" fn(u64, u64, u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };

    println!("\n--- N-BODY SIMPLIFIED BENCHMARK ---");
    run_benchmark("N-Body Advance - JIT Tier 2", || JsValue(native_func(JsValue::undefined().0, b1.0, b2.0, dt.0)));
}

#[test]
fn benchmark_deltablue_simplified() {
    let mut engine = AlbedoJitEngine::new().unwrap();

    /*
    function satisfy(c) {
      let v1 = c.v1;
      let v2 = c.v2;
      let val = v1.value;
      v2.value = val + 1;
    }
    */
    let qjs_func = QjsBytecodeFunction {
        name: "db_satisfy".into(),
        num_args: 1,
        num_locals: 3, // v1=0, v2=1, val=2
        opcodes: vec![
            QjsOpcode::GetArg(0), QjsOpcode::GetField(0), // c.v1
            QjsOpcode::PutLoc(0),
            QjsOpcode::GetArg(0), QjsOpcode::GetField(1), // c.v2
            QjsOpcode::PutLoc(1),
            
            QjsOpcode::GetLoc(0), QjsOpcode::GetField(2), // v1.value
            QjsOpcode::PutLoc(2),
            
            QjsOpcode::GetLoc(2), QjsOpcode::PushI32(1),
            QjsOpcode::Add,        // val + 1
            QjsOpcode::GetLoc(1), QjsOpcode::Swap,
            QjsOpcode::PutField(2), // v2.value = ...
            
            QjsOpcode::PushUndefined,
            QjsOpcode::Return,
        ],
        constant_pool_strings: vec!["v1".into(), "v2".into(), "value".into()],
    };

    let translator = StackToRegisterTranslator::new(&qjs_func);
    let (air_func, _map) = translator.translate(qjs_func.clone());

    // Setup Objects
    let c = albedo_jit::runtime::object_model::alloc_object();
    let v1 = albedo_jit::runtime::object_model::alloc_object();
    let v2 = albedo_jit::runtime::object_model::alloc_object();
    
    let v1_id = albedo_jit::runtime::object_model::intern_string("v1".into());
    let v2_id = albedo_jit::runtime::object_model::intern_string("v2".into());
    let val_id = albedo_jit::runtime::object_model::intern_string("value".into());

    albedo_jit::runtime::object_model::set_prop(c, JsValue::string(v1_id as u64), v1);
    albedo_jit::runtime::object_model::set_prop(c, JsValue::string(v2_id as u64), v2);
    albedo_jit::runtime::object_model::set_prop(v1, JsValue::string(val_id as u64), JsValue::int32(100));
    albedo_jit::runtime::object_model::set_prop(v2, JsValue::string(val_id as u64), JsValue::int32(0));

    let dummy_registry = albedo_jit::engine::jit_bridge::BytecodeRegistry::new();
    let mut compiler = Tier2Compiler::new(&mut engine, &dummy_registry);
    let func_id = compiler.compile(&air_func).unwrap();
    engine.finalize_definitions().unwrap();
    let ptr = engine.get_finalized_function(func_id);
    let native_func: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };

    println!("\n--- DELTABLUE SIMPLIFIED BENCHMARK ---");
    run_benchmark("DeltaBlue Satisfy - JIT Tier 2", || JsValue(native_func(JsValue::undefined().0, c.0)));
}

#[test]
fn benchmark_richards_simplified() {
    let mut engine = AlbedoJitEngine::new().unwrap();

    /*
    function richards(t1, t2) {
      t1.run();
      t2.run();
    }
    */
    let qjs_func = QjsBytecodeFunction {
        name: "richards_dispatcher".into(),
        num_args: 2,
        num_locals: 1,
        opcodes: vec![
            // t1.run()
            QjsOpcode::GetArg(0), QjsOpcode::GetField(0), // t1.run
            QjsOpcode::GetArg(0), // this/arg0 (dependendo da convenção do Call)
            QjsOpcode::Call(0),
            QjsOpcode::Drop,

            // t2.run()
            QjsOpcode::GetArg(1), QjsOpcode::GetField(0), // t2.run
            QjsOpcode::GetArg(1), 
            QjsOpcode::Call(0),
            QjsOpcode::Drop,
            
            QjsOpcode::PushUndefined,
            QjsOpcode::Return,
        ],
        constant_pool_strings: vec!["run".into()],
    };

    let translator = StackToRegisterTranslator::new(&qjs_func);
    let (air_func, _map) = translator.translate(qjs_func.clone());

    // Setup Objects
    let t1 = albedo_jit::runtime::object_model::alloc_object();
    let t2 = albedo_jit::runtime::object_model::alloc_object();
    
    let run_id = albedo_jit::runtime::object_model::intern_string("run".into());

    // Funções dummy (builtins para teste)
    // No AlbedoJIT, Call em Tier 2 usa js_call_ic.
    // Vamos usar o que temos de builtins.
    let f1 = JsValue::int32(42); // Dummy value, o Call vai falhar se nao for callable no runtime
    // Mas para o benchmark de JIT DISPATCH (GetField + Call), o importante eh o overhead do engine.
    
    albedo_jit::runtime::object_model::set_prop(t1, JsValue::string(run_id as u64), f1);
    albedo_jit::runtime::object_model::set_prop(t2, JsValue::string(run_id as u64), f1);

    let dummy_registry = albedo_jit::engine::jit_bridge::BytecodeRegistry::new();
    let mut compiler = Tier2Compiler::new(&mut engine, &dummy_registry);
    let func_id = compiler.compile(&air_func).unwrap();
    engine.finalize_definitions().unwrap();
    let ptr = engine.get_finalized_function(func_id);
    let native_func: extern "C" fn(u64, u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };

    println!("\n--- RICHARDS SIMPLIFIED BENCHMARK ---");
    // Nota: Vai printar erros de "Not a function" se f1 nao for callable, 
    // mas o objetivo aqui eh medir o JIT Path do switch/dispatch.
    run_benchmark("Richards Dispatch - JIT Tier 2", || JsValue(native_func(JsValue::undefined().0, t1.0, t2.0)));
}
    // Verificar se v2.value virou 101 + iterations*1...
    // Na verdade, cada run_benchmark iteracao somara 1 ao valor atual.
    // 100 (warmup) + 1000 (bench) = 1100 + 100 original = 1200?
    // Nao, o v2.value sempre eh lido do v1.value. v1.value eh constante 100.
    // Entao v2.value sempre fica 101.
