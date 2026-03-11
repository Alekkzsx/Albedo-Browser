//! # Albedo Intermediate Representation (AIR)
//!
//! O AIR é um bytecode registrador (register-based, SSA-like) que representa
//! a lógica de execução do JavaScript num nível mais baixo que a AST/QuickJS,
//! mas superior ao Cranelift IR.
//!
//! Ele é desenhado para:
//! 1. Facilitar a geração a partir do bytecode de pilha do QuickJS
//! 2. Permitir análise e otimização em blocos básicos (type inference, DCE)
//! 3. Ser traduzido quase 1:1 para Cranelift IR no Tier 1 e Tier 2

use std::fmt;

// ---------------------------------------------------------------------------
// Registradores e Blocos
// ---------------------------------------------------------------------------

/// Um registrador virtual do AIR (semelhante ao Static Single Assignment).
/// Cada registrador mantém o valor de uma operação ou variável local.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AirReg(pub u32);

impl fmt::Display for AirReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

/// Identificador de um bloco básico.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AirBlockId(pub u32);

impl fmt::Display for AirBlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Opcodes AIR (~50 Opcodes)
// ---------------------------------------------------------------------------

/// Instruções do Albedo IR.
#[derive(Debug, Clone, PartialEq)]
pub enum AirOpcode {
    // === Variables & Constants ===
    /// Carrega um valor i32
    LoadInt32 { dst: AirReg, value: i32 },
    /// Carrega um valor f64 (double)
    LoadFloat64 { dst: AirReg, value: f64 },
    /// Carrega um booleano (true/false)
    LoadBool { dst: AirReg, value: bool },
    /// Carrega o valor JS `undefined`
    LoadUndefined { dst: AirReg },
    /// Carrega o valor JS `null`
    LoadNull { dst: AirReg },
    /// Carrega uma string (usando o índice do constant pool)
    LoadString { dst: AirReg, str_id: u32 },
    /// Copia o valor de um registrador para outro
    Move { dst: AirReg, src: AirReg },
    
    // === Aritmética e Bitwise ===
    Add { dst: AirReg, lhs: AirReg, rhs: AirReg, ic_slot: u32 },
    Sub { dst: AirReg, lhs: AirReg, rhs: AirReg },
    Mul { dst: AirReg, lhs: AirReg, rhs: AirReg },
    Div { dst: AirReg, lhs: AirReg, rhs: AirReg },
    Mod { dst: AirReg, lhs: AirReg, rhs: AirReg },
    Neg { dst: AirReg, src: AirReg },
    BitAnd { dst: AirReg, lhs: AirReg, rhs: AirReg },
    BitOr { dst: AirReg, lhs: AirReg, rhs: AirReg },
    BitXor { dst: AirReg, lhs: AirReg, rhs: AirReg },
    Shl { dst: AirReg, lhs: AirReg, rhs: AirReg },
    Shr { dst: AirReg, lhs: AirReg, rhs: AirReg },
    UShr { dst: AirReg, lhs: AirReg, rhs: AirReg },

    // === Comparação ===
    Eq { dst: AirReg, lhs: AirReg, rhs: AirReg },          // ==
    StrictEq { dst: AirReg, lhs: AirReg, rhs: AirReg },    // ===
    Lt { dst: AirReg, lhs: AirReg, rhs: AirReg },          // <
    Lte { dst: AirReg, lhs: AirReg, rhs: AirReg },         // <=
    Gt { dst: AirReg, lhs: AirReg, rhs: AirReg },          // >
    Gte { dst: AirReg, lhs: AirReg, rhs: AirReg },         // >=
    Not { dst: AirReg, src: AirReg },                      // !

    // === Objetos, Arrays e Propriedades ===
    CreateObj { dst: AirReg },
    CreateArray { dst: AirReg },
    ArrayPush { arr: AirReg, value: AirReg },
    
    GetProp { dst: AirReg, obj: AirReg, prop: AirReg, ic_slot: u32 },
    SetProp { obj: AirReg, prop: AirReg, value: AirReg },
    DeleteProp { dst: AirReg, obj: AirReg, prop: AirReg },
    HasProp { dst: AirReg, obj: AirReg, prop: AirReg },

    // === Funções e Closures ===
    /// Chama uma função. arg_start contém o primeiro argumento; 
    /// os subsequentes (arg_idx..arg_idx+num_args) estão nos registradores sequenciais.
    Call { dst: AirReg, func: AirReg, arg_start: AirReg, num_args: u32, ic_slot: u32 },
    /// Chamada de construtor (new Func(...))
    NewCall { dst: AirReg, func: AirReg, arg_start: AirReg, num_args: u32 },

    // === Type Conversions ===
    ToNumber { dst: AirReg, src: AirReg },
    ToString { dst: AirReg, src: AirReg },
    ToBool { dst: AirReg, src: AirReg },
    TypeOf { dst: AirReg, src: AirReg },
    InstanceOf { dst: AirReg, obj: AirReg, ctor: AirReg },
}

/// Terminadores de Bloco (Control Flow).
/// Cada bloco *deve* terminar com exatamente um destes.
#[derive(Debug, Clone, PartialEq)]
pub enum AirTerminator {
    /// Salto incondicional para outro bloco
    Jump(AirBlockId),
    /// Salto condicional: if (cond) then_blk else else_blk
    JumpIf {
        cond: AirReg,
        then_blk: AirBlockId,
        else_blk: AirBlockId,
    },
    /// Retorna o valor de um registrador da função inteira
    Return(AirReg),
}

// ---------------------------------------------------------------------------
// Estrutura do Módulo/Função
// ---------------------------------------------------------------------------

/// Um Bloco Básico contém instruções puramente sequenciais.
#[derive(Debug, Clone)]
pub struct AirBlock {
    pub id: AirBlockId,
    pub insts: Vec<AirOpcode>,
    pub terminator: Option<AirTerminator>,
}

impl AirBlock {
    pub fn new(id: u32) -> Self {
        Self {
            id: AirBlockId(id),
            insts: Vec::new(),
            terminator: None,
        }
    }
}

/// Repositório de strings constantes da função
#[derive(Debug, Default, Clone)]
pub struct AirConstantPool {
    pub strings: Vec<String>,
}

impl AirConstantPool {
    pub fn add_string(&mut self, s: String) -> u32 {
        let idx = self.strings.len();
        self.strings.push(s);
        idx as u32
    }
}

/// Representa uma função inteira traduzida para o Albedo IR.
#[derive(Debug, Clone)]
pub struct AirFunction {
    pub name: String,
    pub num_params: u32,
    pub registers_count: u32,
    pub blocks: Vec<AirBlock>,
    pub const_pool: AirConstantPool,
}

impl AirFunction {
    /// Verifica se a função está bem formada
    pub fn is_valid(&self) -> bool {
        // Toda função precisa ter pelo menos o entry block
        if self.blocks.is_empty() {
            return false;
        }

        // Todo bloco (exceto talvez o último durante a construção) 
        // precisa ter um terminador
        for block in &self.blocks {
            if block.terminator.is_none() {
                return false;
            }
        }

        true
    }
}
