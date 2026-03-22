//! Entry point e Testes Unitários integrados da Camada de Decoder (QuickJS -> AIR)

pub mod qjs_opcodes;
pub mod source_map;
pub mod translator;

pub use qjs_opcodes::{QjsBytecodeFunction, QjsOpcode};
pub use translator::StackToRegisterTranslator;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{AirOpcode, AirReg, AirTerminator};

    #[test]
    fn test_translate_simple_add() {
        // Pseudo QuickJS: add(a) { return a + 20; }
        let js_func = QjsBytecodeFunction {
            name: "add20".to_string(),
            num_args: 1, // params[0] = a
            num_locals: 0,
            constant_pool_strings: vec![],
            opcodes: vec![
                QjsOpcode::GetArg(0),   // push_arg(0)
                QjsOpcode::PushI32(20), // push_i32(20)
                QjsOpcode::Add,         // add
                QjsOpcode::Return,      // return
            ],
        };

        let translator = StackToRegisterTranslator::new(&js_func);
        let (air_func, source_map) = translator.translate(js_func.clone());

        assert_eq!(air_func.num_params, 2);
        assert_eq!(air_func.blocks.len(), 1);

        let entry_blk = &air_func.blocks[0];
        // this=arg[0]. v1=arg[1]. v2=LoadInt32(20). v3=Add(v1, v2). Return(v3)
        assert_eq!(entry_blk.insts.len(), 2);

        // Verifica as instruções traduzidas
        assert_eq!(
            entry_blk.insts[0],
            AirOpcode::LoadInt32 {
                dst: AirReg(2),
                value: 20
            }
        );
        match &entry_blk.insts[1] {
            AirOpcode::Add {
                dst,
                lhs,
                rhs,
                ic_slot: _,
            } => {
                assert_eq!(*dst, AirReg(3));
                assert_eq!(*lhs, AirReg(1));
                assert_eq!(*rhs, AirReg(2));
            }
            other => panic!("Esperado Add, veio {:?}", other),
        }
        assert_eq!(entry_blk.terminator, Some(AirTerminator::Return(AirReg(3))));

        // Verifica SourceMap (Instrução v2/Add está no index 2 do opcodes QuickJS original)
        assert_eq!(source_map.get_qjs_offset_for_reg(AirReg(3)), Some(2));
    }

    #[test]
    fn test_translate_local_vars() {
        // func() { let x = 5; return x; }
        let js_func = QjsBytecodeFunction {
            name: "returnX".to_string(),
            num_args: 0,
            num_locals: 1,
            constant_pool_strings: vec![],
            opcodes: vec![
                QjsOpcode::PushI32(5),
                QjsOpcode::PutLoc(0),
                QjsOpcode::GetLoc(0), // recupera x
                QjsOpcode::Return,
            ],
        };

        let translator = StackToRegisterTranslator::new(&js_func);
        let (air_func, _) = translator.translate(js_func);

        let blk = &air_func.blocks[0];
        // Com num_locals=1, AirReg(0) é local[0].
        // PushI32(5) aloca AirReg(1) temporário, PutLoc(0) faz Move(AirReg(0), AirReg(1)).
        // GetLoc(0) faz Move(AirReg(2), AirReg(0)), e Return(AirReg(2)).
        assert_eq!(
            blk.insts[0],
            AirOpcode::LoadInt32 {
                dst: AirReg(2),
                value: 5
            }
        );
        // O Return deve ser sobre o registrador que leu o local[0].
        // Move(dst=AirReg(1), src=AirReg(2)) → Move(dst=AirReg(3), src=AirReg(1)) → Return(AirReg(3))
        assert_eq!(blk.terminator, Some(AirTerminator::Return(AirReg(3))));
    }

    #[test]
    fn test_translate_branch() {
        // Pseudo QuickJS:
        // 0: push_bool false
        // 1: if_false 3 (pula pro 4)
        // 2: push_i32 10
        // 3: goto 2 (pula pro 5)
        // 4: push_i32 20  <--- target if_false
        // 5: return       <--- target goto
        let js_func = QjsBytecodeFunction {
            name: "branch".to_string(),
            num_args: 0,
            num_locals: 0,
            constant_pool_strings: vec![],
            opcodes: vec![
                QjsOpcode::PushBool(false), // 0
                QjsOpcode::IfFalse(3),      // 1 (pula 3 opcodes -> alvo é 4)
                QjsOpcode::PushI32(10),     // 2 (Then)
                QjsOpcode::Goto(2),         // 3 (Termina o Then pulando Else -> alvo 5)
                QjsOpcode::PushI32(20),     // 4 (Else/Target do Jump)
                QjsOpcode::Return,          // 5 (Merge Block)
            ],
        };

        let translator = StackToRegisterTranslator::new(&js_func);
        let (air_func, src_map) = translator.translate(js_func);

        // O Translator deve ter criado 4 blocos Básicos:
        // Entry -> (Then, Else) -> Merge Block
        assert_eq!(air_func.blocks.len(), 4);

        let entry_blk = &air_func.blocks[0];

        // Em vez de hardcodar index de block, testamos pelo source map targets!
        // Achar qual block ID foi gerado para o offset 4 (Else)
        let id_else = air_func
            .blocks
            .iter()
            .find(|b| src_map.block_to_qjs_offset.get(&b.id.0) == Some(&4))
            .expect("Deveria haver um bloco mapado pro offset 4")
            .id;

        // Verifica o Terminator gerado na entry (index 0)
        // Lembre-se q o air_func.blocks index não garante ID
        if let Some(AirTerminator::JumpIf {
            cond: _,
            then_blk: _,
            else_blk,
        }) = &entry_blk.terminator
        {
            assert_eq!(*else_blk, id_else);
        } else {
            panic!("Faltando JumpIf terminator!");
        }
    }
}
