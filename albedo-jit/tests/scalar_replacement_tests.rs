//! Integration tests for scalar replacement (v1 conservative).

use albedo_jit::bytecode::{
    AirBlock, AirBlockId, AirConstantPool, AirFunction, AirOpcode, AirReg, AirTerminator,
};
use albedo_jit::compiler::escape_analysis::ScalarProperty;
use albedo_jit::compiler::scalar_replacement::{ScalarReplacer, ScalarTransformResult};

fn create_test_air(instructions: Vec<AirOpcode>, ret: AirReg) -> AirFunction {
    AirFunction {
        name: "test".to_string(),
        num_params: 0,
        registers_count: (instructions.len() + 20) as u32,
        blocks: vec![AirBlock {
            id: AirBlockId(0),
            insts: instructions,
            terminator: Some(AirTerminator::Return(ret)),
        }],
        const_pool: AirConstantPool::default(),
    }
}

#[test]
fn test_scalar_replacement_basic_applies() {
    let mut air = create_test_air(
        vec![
            AirOpcode::CreateObj { dst: AirReg(0) },
            AirOpcode::LoadString {
                dst: AirReg(1),
                str_id: 100,
            },
            AirOpcode::LoadInt32 {
                dst: AirReg(2),
                value: 42,
            },
            AirOpcode::SetProp {
                obj: AirReg(0),
                prop: AirReg(1),
                value: AirReg(2),
            },
            AirOpcode::GetProp {
                dst: AirReg(3),
                obj: AirReg(0),
                prop: AirReg(1),
                ic_slot: 0,
            },
        ],
        AirReg(3),
    );

    let mut replacer = ScalarReplacer::new(AirReg(20));
    let properties = vec![ScalarProperty {
        prop_id: 100,
        prop_reg: AirReg(0),
        is_writable: true,
    }];

    let res = replacer.replace_object_with_scalars(&mut air, AirReg(0), &properties);
    assert_eq!(res, ScalarTransformResult::Applied);
    assert!(!air.blocks[0]
        .insts
        .iter()
        .any(|i| matches!(i, AirOpcode::CreateObj { dst } if *dst == AirReg(0))));
}

#[test]
fn test_scalar_replacement_rejects_partial_coverage() {
    let mut air = create_test_air(
        vec![
            AirOpcode::CreateObj { dst: AirReg(0) },
            AirOpcode::LoadString {
                dst: AirReg(1),
                str_id: 100,
            },
            AirOpcode::LoadString {
                dst: AirReg(2),
                str_id: 101,
            },
            AirOpcode::LoadInt32 {
                dst: AirReg(3),
                value: 1,
            },
            AirOpcode::SetProp {
                obj: AirReg(0),
                prop: AirReg(1),
                value: AirReg(3),
            },
            AirOpcode::GetProp {
                dst: AirReg(4),
                obj: AirReg(0),
                prop: AirReg(2),
                ic_slot: 0,
            },
        ],
        AirReg(4),
    );

    let mut replacer = ScalarReplacer::new(AirReg(20));
    let properties = vec![ScalarProperty {
        prop_id: 100,
        prop_reg: AirReg(0),
        is_writable: true,
    }];

    let res = replacer.replace_object_with_scalars(&mut air, AirReg(0), &properties);
    assert_eq!(res, ScalarTransformResult::RejectedCoverage);
}

#[test]
fn test_scalar_replacement_next_reg_advances() {
    let mut replacer = ScalarReplacer::new(AirReg(10));
    let mut air = create_test_air(
        vec![
            AirOpcode::CreateObj { dst: AirReg(0) },
            AirOpcode::LoadString {
                dst: AirReg(1),
                str_id: 200,
            },
            AirOpcode::LoadInt32 {
                dst: AirReg(2),
                value: 5,
            },
            AirOpcode::SetProp {
                obj: AirReg(0),
                prop: AirReg(1),
                value: AirReg(2),
            },
        ],
        AirReg(2),
    );
    let props = vec![ScalarProperty {
        prop_id: 200,
        prop_reg: AirReg(0),
        is_writable: true,
    }];
    let _ = replacer.replace_object_with_scalars(&mut air, AirReg(0), &props);
    assert_eq!(replacer.get_next_reg(), AirReg(11));
}
