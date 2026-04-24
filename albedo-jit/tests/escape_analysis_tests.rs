//! Integration tests for escape analysis (v1 conservative).

use albedo_jit::bytecode::{
    AirBlock, AirBlockId, AirConstantPool, AirFunction, AirOpcode, AirReg, AirTerminator,
};
use albedo_jit::compiler::escape_analysis::{run_field_sensitive, EscapeStatus};

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
fn test_scalar_replaceable_local_object() {
    let air = create_test_air(
        vec![
            AirOpcode::CreateObj { dst: AirReg(0) },
            AirOpcode::LoadString {
                dst: AirReg(1),
                str_id: 10,
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

    let res = run_field_sensitive(&air);
    assert_eq!(
        res.statuses.get(&AirReg(0)),
        Some(&EscapeStatus::ScalarReplaceable)
    );
    assert!(res.scalar_replaceable.contains(&AirReg(0)));
}

#[test]
fn test_stack_only_dynamic_property() {
    let air = create_test_air(
        vec![
            AirOpcode::CreateObj { dst: AirReg(0) },
            AirOpcode::LoadInt32 {
                dst: AirReg(1),
                value: 99,
            }, // dynamic prop key
            AirOpcode::LoadInt32 {
                dst: AirReg(2),
                value: 1,
            },
            AirOpcode::SetProp {
                obj: AirReg(0),
                prop: AirReg(1),
                value: AirReg(2),
            },
        ],
        AirReg(2),
    );

    let res = run_field_sensitive(&air);
    assert_eq!(res.statuses.get(&AirReg(0)), Some(&EscapeStatus::StackOnly));
    assert!(res.stack_allocatable.contains(&AirReg(0)));
}

#[test]
fn test_escaping_via_return() {
    let air = create_test_air(vec![AirOpcode::CreateObj { dst: AirReg(0) }], AirReg(0));
    let res = run_field_sensitive(&air);
    assert_eq!(res.statuses.get(&AirReg(0)), Some(&EscapeStatus::Escaping));
}

#[test]
fn test_escaping_via_call_argument() {
    let air = create_test_air(
        vec![
            AirOpcode::CreateObj { dst: AirReg(0) },
            AirOpcode::LoadInt32 {
                dst: AirReg(1),
                value: 0,
            },
            AirOpcode::Call {
                dst: AirReg(2),
                func: AirReg(1),
                this: AirReg(0),
                arg_start: AirReg(0),
                num_args: 0,
                ic_slot: 0,
            },
        ],
        AirReg(2),
    );
    let res = run_field_sensitive(&air);
    assert_eq!(res.statuses.get(&AirReg(0)), Some(&EscapeStatus::Escaping));
}
