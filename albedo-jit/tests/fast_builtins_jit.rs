use albedo_jit::builtins::BuiltinId;
use albedo_jit::bytecode::{
    AirBlock, AirBlockId, AirConstantPool, AirFunction, AirOpcode, AirReg, AirTerminator,
};
use albedo_jit::jit_engine::AlbedoJitEngine;
use albedo_jit::js_value::JsValue;
use albedo_jit::tier2_compiler::Tier2Compiler;
use albedo_jit::type_feedback::{IcKind, TypeFeedbackRegistry};

#[test]
fn test_fast_math_jit_stress() {
    let mut engine = AlbedoJitEngine::with_budget(1024 * 1024).unwrap();

    // 0. Alocar slots IC (precisamos que os IDs batam com o AIR)
    // Se o registry for global, melhor limpar ou garantir a ordem.
    // Como os testes rodam em paralelo, isso pode ser um problema.
    // Mas vamos assumir slots 0, 1, 2, 3.
    let s0 = TypeFeedbackRegistry::alloc_slot(IcKind::Call);
    let s1 = TypeFeedbackRegistry::alloc_slot(IcKind::Call);
    let s2 = TypeFeedbackRegistry::alloc_slot(IcKind::Add);
    let s3 = TypeFeedbackRegistry::alloc_slot(IcKind::Add);

    let mut air = AirFunction {
        name: "stress_math".to_string(),
        num_params: 1,
        registers_count: 20,
        blocks: Vec::new(),
        const_pool: AirConstantPool::default(),
    };

    let r_n = AirReg(0);
    let r_s = AirReg(1);
    let r_i = AirReg(2);
    let r_one = AirReg(3);
    let r_cond = AirReg(4);
    let r_abs = AirReg(5);
    let r_sqrt = AirReg(6);
    let r_math_abs = AirReg(7);
    let r_math_sqrt = AirReg(8);

    let mut b0 = AirBlock::new(0);
    b0.insts.push(AirOpcode::LoadFloat64 {
        dst: r_s,
        value: 0.0,
    });
    b0.insts.push(AirOpcode::LoadInt32 { dst: r_i, value: 0 });
    b0.insts.push(AirOpcode::LoadInt32 {
        dst: r_one,
        value: 1,
    });

    let abs_val = JsValue::builtin(BuiltinId::MathAbs as u64);
    let sqrt_val = JsValue::builtin(BuiltinId::MathSqrt as u64);
    b0.insts.push(AirOpcode::LoadFloat64 {
        dst: r_math_abs,
        value: f64::from_bits(abs_val.0),
    });
    b0.insts.push(AirOpcode::LoadFloat64 {
        dst: r_math_sqrt,
        value: f64::from_bits(sqrt_val.0),
    });
    b0.terminator = Some(AirTerminator::Jump(AirBlockId(1)));

    let mut b1 = AirBlock::new(1);
    b1.insts.push(AirOpcode::Lt {
        dst: r_cond,
        lhs: r_i,
        rhs: r_n,
    });
    b1.terminator = Some(AirTerminator::JumpIf {
        cond: r_cond,
        then_blk: AirBlockId(2),
        else_blk: AirBlockId(3),
    });

    let mut b2 = AirBlock::new(2);
    // Injetar feedback monomórfico nos slots alocados
    TypeFeedbackRegistry::record_call(s0, abs_val);
    TypeFeedbackRegistry::record_call(s1, sqrt_val);

    b2.insts.push(AirOpcode::Call {
        dst: r_abs,
        func: r_math_abs,
        arg_start: r_i,
        num_args: 1,
        ic_slot: s0,
    });
    b2.insts.push(AirOpcode::Call {
        dst: r_sqrt,
        func: r_math_sqrt,
        arg_start: r_abs,
        num_args: 1,
        ic_slot: s1,
    });
    b2.insts.push(AirOpcode::Add {
        dst: r_s,
        lhs: r_s,
        rhs: r_sqrt,
        ic_slot: s2,
    });
    b2.insts.push(AirOpcode::Add {
        dst: r_i,
        lhs: r_i,
        rhs: r_one,
        ic_slot: s3,
    });
    b2.terminator = Some(AirTerminator::Jump(AirBlockId(1)));

    let mut b3 = AirBlock::new(3);
    b3.terminator = Some(AirTerminator::Return(r_s));

    air.blocks.push(b0);
    air.blocks.push(b1);
    air.blocks.push(b2);
    air.blocks.push(b3);

    let mut compiler = Tier2Compiler::new(&mut engine);
    let func_id = compiler.compile(&air).unwrap();
    engine.finalize_definitions().unwrap();
    let ptr = engine.get_finalized_function(func_id);

    let func: fn(u64) -> u64 = unsafe { std::mem::transmute(ptr) };
    let n = JsValue::int32(100).0;
    let res = JsValue(func(n));

    assert!(res.is_float64());
    println!("Res: {:?}", res);
}

#[test]
fn test_fast_array_jit_stress() {
    let mut engine = AlbedoJitEngine::with_budget(1024 * 1024).unwrap();

    let s10 = TypeFeedbackRegistry::alloc_slot(IcKind::Call);
    let s12 = TypeFeedbackRegistry::alloc_slot(IcKind::GetProp);

    let mut air = AirFunction {
        name: "stress_array".to_string(),
        num_params: 1,
        registers_count: 20,
        blocks: Vec::new(),
        const_pool: AirConstantPool::default(),
    };

    let r_n = AirReg(0);
    let r_a = AirReg(1);
    let r_i = AirReg(2);
    let r_one = AirReg(3);
    let r_cond = AirReg(4);
    let r_push = AirReg(5);
    let r_len = AirReg(6);
    let r_len_str = AirReg(7);

    let mut b0 = AirBlock::new(0);
    b0.insts.push(AirOpcode::CreateArray { dst: r_a });
    b0.insts.push(AirOpcode::LoadInt32 { dst: r_i, value: 0 });
    b0.insts.push(AirOpcode::LoadInt32 {
        dst: r_one,
        value: 1,
    });
    let push_builtin = JsValue::builtin(BuiltinId::ArrayPush as u64);
    b0.insts.push(AirOpcode::LoadInt64 {
        dst: r_push,
        value: push_builtin.0 as i64,
    });
    b0.terminator = Some(AirTerminator::Jump(AirBlockId(1)));

    let mut b1 = AirBlock::new(1);
    b1.insts.push(AirOpcode::Lt {
        dst: r_cond,
        lhs: r_i,
        rhs: r_n,
    });
    b1.terminator = Some(AirTerminator::JumpIf {
        cond: r_cond,
        then_blk: AirBlockId(2),
        else_blk: AirBlockId(3),
    });

    let mut b2 = AirBlock::new(2);
    TypeFeedbackRegistry::record_call(s10, push_builtin);

    b2.insts.push(AirOpcode::Call {
        dst: r_len,
        func: r_push,
        arg_start: r_a,
        num_args: 2,
        ic_slot: s10,
    });
    b2.insts.push(AirOpcode::Add {
        dst: r_i,
        lhs: r_i,
        rhs: r_one,
        ic_slot: 99,
    }); // Slot dummy p/ add
    b2.terminator = Some(AirTerminator::Jump(AirBlockId(1)));

    let mut b3 = AirBlock::new(3);
    let mut cp = AirConstantPool::default();
    let len_id = cp.add_string("length".to_string());
    air.const_pool = cp;
    b3.insts.push(AirOpcode::LoadString {
        dst: r_len_str,
        str_id: len_id,
    });
    b3.insts.push(AirOpcode::GetProp {
        dst: r_len,
        obj: r_a,
        prop: r_len_str,
        ic_slot: s12,
    });
    b3.terminator = Some(AirTerminator::Return(r_len));

    air.blocks.push(b0);
    air.blocks.push(b1);
    air.blocks.push(b2);
    air.blocks.push(b3);

    let mut compiler = Tier2Compiler::new(&mut engine);
    let func_id = compiler.compile(&air).unwrap();
    engine.finalize_definitions().unwrap();
    let ptr = engine.get_finalized_function(func_id);

    let func: fn(u64) -> u64 = unsafe { std::mem::transmute(ptr) };
    let n = JsValue::int32(50).0;
    let res = JsValue(func(n));

    assert_eq!(res.as_int32(), 50);
}
