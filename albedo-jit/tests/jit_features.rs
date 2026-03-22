use albedo_jit::{
    compiler::air_interpreter::{AirInterpreter, OsrManager},
    runtime::builtins::BuiltinId,
    runtime::{object_model, runtime_helpers},
    AirBuilder, AirOpcode, AlbedoJitEngine, JsValue, Tier2Compiler,
};


#[test]
fn test_deopt_type_change() {
    let mut engine = AlbedoJitEngine::new().unwrap();

    let mut b = AirBuilder::new("add", 3, 0);
    let _this = b.param(0);
    let a = b.param(1);
    let bparam = b.param(2);
    let sum = b.emit_add(a, bparam);
    b.emit_return(sum);
    let air = b.build();

    // Aquecer feedback para int32
    let mut ic_slot = 0;
    if let AirOpcode::Add { ic_slot: s, .. } = &air.blocks[0].insts[0] {
        ic_slot = *s;
    }
    for _ in 0..64 {
        let _ =
            runtime_helpers::js_add_ic(JsValue::int32(1).0, JsValue::int32(2).0, ic_slot as u64);
    }

    let dummy_registry = albedo_jit::engine::jit_bridge::BytecodeRegistry::new();
    let mut compiler = Tier2Compiler::new(&mut engine, &dummy_registry);
    let func_id = compiler.compile(&air).unwrap();
    engine.finalize_definitions().unwrap();
    let ptr = engine.get_finalized_function(func_id);
    let func: extern "C" fn(u64, u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };

    let res = JsValue(func(JsValue::undefined().0, JsValue::float64(1.5).0, JsValue::float64(2.25).0));
    assert!(res.is_float64());
    assert_eq!(res.as_float64(), 3.75);
}

#[test]
fn test_osr_loop_tier_up() {
    let mut b = AirBuilder::new("sum_to_n", 2, 0);
    let _this = b.param(0);
    let n = b.param(1);
    let zero = b.emit_load_int32(0);

    let i = b.new_reg();
    b.emit_move(i, zero);
    let sum = b.new_reg();
    b.emit_move(sum, zero);

    let loop_header = b.create_block();
    let loop_body = b.create_block();
    let exit = b.create_block();

    b.emit_jump(loop_header);

    b.switch_block(loop_header);
    let cond = b.emit_lt(i, n);
    b.emit_jump_if(cond, loop_body, exit);

    b.switch_block(loop_body);
    let new_sum = b.emit_add(sum, i);
    b.emit_move(sum, new_sum);
    let one = b.emit_load_int32(1);
    let new_i = b.emit_add(i, one);
    b.emit_move(i, new_i);
    b.emit_jump(loop_header);

    b.switch_block(exit);
    b.emit_return(sum);

    let air = b.build();

    let mut regs = vec![JsValue::undefined(); air.registers_count as usize];
    regs[1] = JsValue::int32(500);

    let mut osr = OsrManager::new(5);
    let res = AirInterpreter::execute_with_osr(&air, &mut regs, &mut osr);
    assert_eq!(res.as_int32(), (0..500).sum::<i32>());
    assert!(osr.compiled_count() >= 1);
}

#[test]
fn test_builtins_math_array_string_json() {
    // Math.sqrt
    let func = JsValue::builtin(BuiltinId::MathSqrt as u64);
    let args = [JsValue::float64(9.0).0];
    let res = JsValue(runtime_helpers::js_call_ic(
        func.0,
        JsValue::undefined().0,
        args.as_ptr() as u64,
        1,
        0,
    ));
    assert!(res.is_float64());
    assert_eq!(res.as_float64(), 3.0);

    // Array.push / pop
    let arr = object_model::alloc_array();
    let push = JsValue::builtin(BuiltinId::ArrayPush as u64);
    let args_push = [arr.0, JsValue::int32(10).0, JsValue::int32(20).0];
    let len = JsValue(runtime_helpers::js_call_ic(
        push.0,
        arr.0,
        args_push.as_ptr() as u64,
        3,
        0,
    ));
    assert_eq!(len.as_int32(), 2);

    let pop = JsValue::builtin(BuiltinId::ArrayPop as u64);
    let args_pop = [arr.0];
    let v = JsValue(runtime_helpers::js_call_ic(
        pop.0,
        arr.0,
        args_pop.as_ptr() as u64,
        1,
        0,
    ));
    assert_eq!(v.as_int32(), 20);

    // String.charAt
    let s_id = object_model::intern_string("albedo".to_string());
    let s_val = JsValue::string(s_id as u64);
    let char_at = JsValue::builtin(BuiltinId::StringCharAt as u64);
    let args_char = [s_val.0, JsValue::int32(1).0];
    let c = JsValue(runtime_helpers::js_call_ic(
        char_at.0,
        s_val.0,
        args_char.as_ptr() as u64,
        2,
        0,
    ));
    let c_str = object_model::get_string(c.as_string_id() as u32).unwrap();
    assert_eq!(c_str, "l");

    // JSON.parse
    let json = object_model::intern_string("{\"a\":2,\"b\":[1,2]}".to_string());
    let json_val = JsValue::string(json as u64);
    let json_parse = JsValue::builtin(BuiltinId::JsonParse as u64);
    let args_json = [json_val.0];
    let obj = JsValue(runtime_helpers::js_call_ic(
        json_parse.0,
        JsValue::undefined().0,
        args_json.as_ptr() as u64,
        1,
        0,
    ));
    let key_a = object_model::intern_string("a".to_string());
    let a_val = object_model::get_prop(obj, JsValue::string(key_a as u64));
    assert_eq!(a_val.as_int32(), 2);
}
