//! Representação em Rust dos Opcodes do QuickJS (Stack-based)
//!
//! Na Fase 1.3, suportamos um subconjunto seguro (~40 opcodes) que representam
//! aproximadamente 95% do código Web comum (matemática, propriedades, branches).

#[derive(Debug, Clone, PartialEq)]
pub enum QjsOpcode {
    // --- Valores/Push ---
    PushI32(i32),
    PushFloat64(f64),
    PushBool(bool),
    PushUndefined,
    PushNull,
    PushString(u32), // Índice no Constant Pool
    
    // --- Manipulação de Pilha ---
    Drop,
    Dup,
    Swap,

    // --- Variáveis Locais ---
    GetLoc(u32),
    PutLoc(u32),
    GetArg(u32),
    PutArg(u32),

    // --- Aritmética e Bitwise ---
    Add, 
    Sub, 
    Mul, 
    Div, 
    Mod,
    Neg,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    UShr,

    // --- Lógica e Comparação ---
    Not,
    Eq,
    StrictEq,
    Lt,
    Lte,
    Gt,
    Gte,

    // --- Acesso a Propriedades ---
    GetField(u32), // String Index
    PutField(u32), 
    GetArrayEl,
    PutArrayEl,

    // --- Controle de Fluxo ---
    Goto(i32),       // Offset relativo no bytecode
    IfTrue(i32),
    IfFalse(i32),
    Return,
    ReturnUndef,

    // --- Objetos e Funções ---
    CreateObj,
    CreateArray,
    Call(u32),       // Número de Argumentos
}

/// Metadados e código bruto capturado do QuickJS (Simulado na Fase 1.3)
#[derive(Debug, Clone)]
pub struct QjsBytecodeFunction {
    pub name: String,
    pub num_args: u32,
    pub num_locals: u32,
    pub opcodes: Vec<QjsOpcode>,
    pub constant_pool_strings: Vec<String>,
}
