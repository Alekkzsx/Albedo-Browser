//! # AlbedoJitEngine — Motor JIT principal
//!
//! Gerencia o pipeline de compilação JIT:
//! Bytecode → Cranelift IR → Código de máquina nativo (x86-64 / ARM64).
//!
//! Este módulo é o coração do AlbedoJIT. Ele inicializa o backend Cranelift,
//! compila funções para código nativo e gerencia o cache de código compilado.

use crate::compiler::code_cache::{CachedCode, CodeCache, JitTier, NativeCodePtr};
use crate::infra::executable_memory::{CodePool, CodePoolStats, MemoryError};
use std::sync::Arc;
use crate::engine::profiler::FunctionId;
use cranelift_codegen::ir::types::I64;
use cranelift_codegen::ir::{AbiParam, Function, InstBuilder, UserFuncName};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::Context;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use parking_lot::RwLock;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Erros
// ---------------------------------------------------------------------------

/// Erros que podem ocorrer durante a compilação JIT.
#[derive(Debug, Error)]
pub enum JitError {
    #[error("Falha ao criar ISA nativa: {0}")]
    IsaCreation(String),

    #[error("Falha ao declarar função: {0}")]
    FuncDeclaration(#[from] cranelift_module::ModuleError),

    #[error("Falha na compilação Cranelift: {0}")]
    Compilation(String),

    #[error("Função '{0}' não encontrada no cache")]
    FunctionNotFound(String),

    #[error("Erro de memória JIT: {0}")]
    MemoryError(#[from] MemoryError),
}

// ---------------------------------------------------------------------------
// Estrutura principal
// ---------------------------------------------------------------------------

/// Entrada no cache de código compilado.
struct _CompiledFunction {
    /// ID da função no módulo Cranelift.
    _func_id: FuncId,
    /// Ponteiro para o código de máquina nativo (após finalize_definitions).
    _native_ptr: *const u8,
}

// SAFETY: Os ponteiros nativos são gerenciados pelo JITModule que garante
// que o código permanece válido enquanto o módulo existe.
unsafe impl Send for _CompiledFunction {}
unsafe impl Sync for _CompiledFunction {}

/// AlbedoJitEngine — Motor JIT principal do Albedo Browser.
///
/// Responsável por:
/// - Inicializar o backend Cranelift para a ISA nativa (x86-64 ou ARM64)
/// - Compilar funções de Cranelift IR para código de máquina nativo
/// - Gerenciar cache de funções compiladas
/// - Prover API para executar código JIT compilado
pub struct AlbedoJitEngine {
    /// Módulo JIT do Cranelift (gerencia memória executável).
    pub(crate) module: JITModule,
    /// Cache avançado de funções compiladas (metadados + invalidação).
    pub(crate) code_cache: Arc<CodeCache>,
    /// Pool de memória executável própria para stubs e trampolines.
    code_pool: RwLock<CodePool>,
}

impl AlbedoJitEngine {
    /// Cria uma nova instância do AlbedoJitEngine com budget padrão de 64MB.
    pub fn new() -> Result<Self, JitError> {
        Self::with_budget(64 * 1024 * 1024)
    }

    /// Cria uma nova instância com um budget de memória JIT específico.
    pub fn with_budget(budget_bytes: usize) -> Result<Self, JitError> {
        // Configurações do Cranelift
        let mut flag_builder = settings::builder();

        // Otimizações habilitadas por padrão
        flag_builder
            .set("opt_level", "speed")
            .map_err(|e| JitError::IsaCreation(e.to_string()))?;

        let isa_builder =
            cranelift_native::builder().map_err(|msg| JitError::IsaCreation(msg.to_string()))?;

        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| JitError::IsaCreation(e.to_string()))?;

        // Criar o módulo JIT — gerencia alocação de memória executável
        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        // Registrar as C-ABI helper functions do runtime para serem resolvidas pelo JIT Module
        builder.symbol("js_add", crate::runtime::runtime_helpers::js_add as *const u8);
        builder.symbol("js_add_ic", crate::runtime::runtime_helpers::js_add_ic as *const u8);
        builder.symbol("js_sub", crate::runtime::runtime_helpers::js_sub as *const u8);
        builder.symbol("js_mul", crate::runtime::runtime_helpers::js_mul as *const u8);
        builder.symbol(
            "js_strict_eq",
            crate::runtime::runtime_helpers::js_strict_eq as *const u8,
        );
        builder.symbol("js_eq", crate::runtime::runtime_helpers::js_eq as *const u8);
        builder.symbol("js_lt", crate::runtime::runtime_helpers::js_lt as *const u8);
        builder.symbol(
            "js_to_bool",
            crate::runtime::runtime_helpers::js_to_bool as *const u8,
        );
        builder.symbol(
            "js_get_prop_ic",
            crate::runtime::runtime_helpers::js_get_prop_ic as *const u8,
        );
        builder.symbol(
            "js_call_ic",
            crate::runtime::runtime_helpers::js_call_ic as *const u8,
        );
        builder.symbol(
            "js_create_obj",
            crate::runtime::runtime_helpers::js_create_obj as *const u8,
        );
        builder.symbol(
            "js_create_array",
            crate::runtime::runtime_helpers::js_create_array as *const u8,
        );
        builder.symbol(
            "js_set_prop",
            crate::runtime::runtime_helpers::js_set_prop as *const u8,
        );
        builder.symbol(
            "js_deopt_bailout",
            crate::compiler::deopt::js_deopt_bailout as *const u8,
        );
        builder.symbol(
            "js_has_prop",
            crate::runtime::object_model::has_prop as *const u8,
        );
        builder.symbol(
            "js_delete_prop",
            crate::runtime::object_model::delete_prop as *const u8,
        );
        builder.symbol(
            "js_type_of",
            crate::runtime::runtime_helpers::js_type_of as *const u8,
        );
        builder.symbol(
            "js_instance_of",
            crate::runtime::runtime_helpers::js_instance_of as *const u8,
        );

        // Builtins Rápidos (Fase 1)
        builder.symbol(
            "fast_math_abs",
            crate::runtime::fast_builtins::fast_math_abs as *const u8,
        );
        builder.symbol(
            "fast_math_sqrt",
            crate::runtime::fast_builtins::fast_math_sqrt as *const u8,
        );
        builder.symbol(
            "fast_math_floor",
            crate::runtime::fast_builtins::fast_math_floor as *const u8,
        );
        builder.symbol(
            "fast_math_ceil",
            crate::runtime::fast_builtins::fast_math_ceil as *const u8,
        );
        builder.symbol(
            "fast_math_acos",
            crate::runtime::fast_builtins::fast_math_acos as *const u8,
        );
        builder.symbol(
            "fast_math_acosh",
            crate::runtime::fast_builtins::fast_math_acosh as *const u8,
        );
        builder.symbol(
            "fast_math_asin",
            crate::runtime::fast_builtins::fast_math_asin as *const u8,
        );
        builder.symbol(
            "fast_math_asinh",
            crate::runtime::fast_builtins::fast_math_asinh as *const u8,
        );
        builder.symbol(
            "fast_math_atan",
            crate::runtime::fast_builtins::fast_math_atan as *const u8,
        );
        builder.symbol(
            "fast_math_atan2",
            crate::runtime::fast_builtins::fast_math_atan2 as *const u8,
        );
        builder.symbol(
            "fast_math_atanh",
            crate::runtime::fast_builtins::fast_math_atanh as *const u8,
        );
        builder.symbol(
            "fast_math_cos",
            crate::runtime::fast_builtins::fast_math_cos as *const u8,
        );
        builder.symbol(
            "fast_math_cosh",
            crate::runtime::fast_builtins::fast_math_cosh as *const u8,
        );
        builder.symbol(
            "fast_math_sin",
            crate::runtime::fast_builtins::fast_math_sin as *const u8,
        );
        builder.symbol(
            "fast_math_sinh",
            crate::runtime::fast_builtins::fast_math_sinh as *const u8,
        );
        builder.symbol(
            "fast_math_tan",
            crate::runtime::fast_builtins::fast_math_tan as *const u8,
        );
        builder.symbol(
            "fast_math_tanh",
            crate::runtime::fast_builtins::fast_math_tanh as *const u8,
        );
        builder.symbol(
            "fast_math_exp",
            crate::runtime::fast_builtins::fast_math_exp as *const u8,
        );
        builder.symbol(
            "fast_math_expm1",
            crate::runtime::fast_builtins::fast_math_expm1 as *const u8,
        );
        builder.symbol(
            "fast_math_log",
            crate::runtime::fast_builtins::fast_math_log as *const u8,
        );
        builder.symbol(
            "fast_math_log1p",
            crate::runtime::fast_builtins::fast_math_log1p as *const u8,
        );
        builder.symbol(
            "fast_math_log10",
            crate::runtime::fast_builtins::fast_math_log10 as *const u8,
        );
        builder.symbol(
            "fast_math_log2",
            crate::runtime::fast_builtins::fast_math_log2 as *const u8,
        );
        builder.symbol(
            "fast_math_cbrt",
            crate::runtime::fast_builtins::fast_math_cbrt as *const u8,
        );
        builder.symbol(
            "fast_math_clz32",
            crate::runtime::fast_builtins::fast_math_clz32 as *const u8,
        );
        builder.symbol(
            "fast_math_fround",
            crate::runtime::fast_builtins::fast_math_fround as *const u8,
        );
        builder.symbol(
            "fast_math_hypot",
            crate::runtime::fast_builtins::fast_math_hypot as *const u8,
        );
        builder.symbol(
            "fast_math_imul",
            crate::runtime::fast_builtins::fast_math_imul as *const u8,
        );
        builder.symbol(
            "fast_math_pow",
            crate::runtime::fast_builtins::fast_math_pow as *const u8,
        );
        builder.symbol(
            "fast_math_random",
            crate::runtime::fast_builtins::fast_math_random as *const u8,
        );
        builder.symbol(
            "fast_math_round",
            crate::runtime::fast_builtins::fast_math_round as *const u8,
        );
        builder.symbol(
            "fast_math_sign",
            crate::runtime::fast_builtins::fast_math_sign as *const u8,
        );
        builder.symbol(
            "fast_math_trunc",
            crate::runtime::fast_builtins::fast_math_trunc as *const u8,
        );
        builder.symbol(
            "fast_math_max",
            crate::runtime::fast_builtins::fast_math_max as *const u8,
        );
        builder.symbol(
            "fast_math_min",
            crate::runtime::fast_builtins::fast_math_min as *const u8,
        );


        builder.symbol(
            "fast_array_push",
            crate::runtime::fast_builtins::fast_array_push as *const u8,
        );
        builder.symbol(
            "fast_array_pop",
            crate::runtime::fast_builtins::fast_array_pop as *const u8,
        );
        builder.symbol(
            "fast_string_char_at",
            crate::runtime::fast_builtins::fast_string_char_at as *const u8,
        );
        builder.symbol(
            "fast_json_parse",
            crate::runtime::fast_builtins::fast_json_parse as *const u8,
        );

        // Dummy placeholder pra coisas não-feitas que crashariam de unresolved symbol exception
        extern "C" fn js_unimplemented_mock() -> u64 {
            crate::runtime::js_value::JsValue::undefined().0
        }
        builder.symbol("js_unimplemented", js_unimplemented_mock as *const u8);

        let module = JITModule::new(builder);
        let code_cache = Arc::new(CodeCache::new());

        // Registrar o CodeCache globalmente para que o runtime possa acessá-lo
        crate::compiler::code_cache::set_global_code_cache(Arc::clone(&code_cache));

        Ok(Self {
            module,
            code_cache,
            code_pool: RwLock::new(CodePool::new(budget_bytes)),
        })
    }

    /// Compila uma função de demonstração: `add(a: i64, b: i64) -> i64`
    ///
    /// Esta função serve como prova de conceito do pipeline completo:
    /// Cranelift IR → código de máquina nativo → cache.
    ///
    /// # Pipeline
    /// 1. Declara a assinatura da função (dois i64 → um i64)
    /// 2. Constrói o corpo via Cranelift FunctionBuilder
    /// 3. Compila para código de máquina nativo da CPU atual
    /// 4. Armazena no cache por nome
    pub fn compile_add_function(&mut self) -> Result<(), JitError> {
        let func_name = "jit_add";

        // 1. Declarar a assinatura da função
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(I64)); // param a: i64
        sig.params.push(AbiParam::new(I64)); // param b: i64
        sig.returns.push(AbiParam::new(I64)); // return: i64

        // 2. Declarar a função no módulo
        let func_id = self
            .module
            .declare_function(func_name, Linkage::Export, &sig)?;

        // 3. Construir o corpo da função com Cranelift IR
        let mut func =
            Function::with_name_signature(UserFuncName::user(0, func_id.as_u32()), sig.clone());

        let mut func_builder_ctx = FunctionBuilderContext::new();
        {
            let mut builder = FunctionBuilder::new(&mut func, &mut func_builder_ctx);

            // Criar bloco de entrada (entry block)
            let entry_block = builder.create_block();

            // Adicionar parâmetros do bloco (correspondem aos params da sig)
            builder.append_block_params_for_function_params(entry_block);

            // Posicionar o builder no bloco de entrada
            builder.switch_to_block(entry_block);
            builder.seal_block(entry_block);

            // Obter os valores dos parâmetros
            let param_a = builder.block_params(entry_block)[0];
            let param_b = builder.block_params(entry_block)[1];

            // Gerar instrução: result = a + b (instrução nativa IADD da CPU)
            let sum = builder.ins().iadd(param_a, param_b);

            // Retornar o resultado
            builder.ins().return_(&[sum]);

            // Finalizar a construção da função
            builder.finalize();
        }

        // 4. Compilar para código de máquina nativo
        let mut ctx = Context::for_function(func);
        self.module
            .define_function(func_id, &mut ctx)
            .map_err(|e| JitError::Compilation(e.to_string()))?;

        // 5. Materializar o código na memória executável
        self.module.clear_context(&mut ctx);
        self.module
            .finalize_definitions()
            .map_err(|e| JitError::Compilation(e.to_string()))?;

        // 6. Obter ponteiro para o código de máquina nativo
        let native_ptr = self.module.get_finalized_function(func_id);

        // TODO: No futuro, o JITModule deve nos dar o tamanho real em bytes do código.
        // Por enquanto usamos um placeholder de 128 bytes para estatísticas.
        let code_size = 128;

        // 7. Armazenar no Code Cache
        let id = FunctionId(func_name.to_string());
        let entry = CachedCode::new(id, NativeCodePtr(native_ptr), func_id, code_size, JitTier::Baseline);
        self.code_cache.insert(entry);

        Ok(())
    }

    /// Executa a função `jit_add` compilada com os argumentos fornecidos.
    ///
    /// # Safety
    ///
    /// Esta função é segura porque:
    /// - O código nativo foi gerado pelo Cranelift com tipagem verificada
    /// - O ponteiro é válido enquanto o AlbedoJitEngine existir
    /// - A calling convention é a padrão do sistema (SystemV / Windows)
    pub fn execute_add(&self, a: i64, b: i64) -> Result<i64, JitError> {
        let id = FunctionId("jit_add".to_string());
        let compiled = self
            .code_cache
            .lookup(&id)
            .ok_or_else(|| JitError::FunctionNotFound("jit_add".to_string()))?;

        // Incrementar contador de execução do cache
        compiled.increment_execution();

        // SAFETY: O código nativo foi compilado pelo Cranelift com assinatura
        // (i64, i64) -> i64. O ponteiro é válido enquanto o módulo JIT existir.
        let func_ptr: fn(i64, i64) -> i64 = unsafe { std::mem::transmute(compiled.native_ptr) };

        Ok(func_ptr(a, b))
    }

    /// Retorna o número de funções compiladas no cache.
    pub fn compiled_function_count(&self) -> usize {
        self.code_cache.len()
    }

    pub fn finalize_definitions(&mut self) -> Result<(), JitError> {
        self.module
            .finalize_definitions()
            .map_err(|e| JitError::Compilation(e.to_string()))
    }

    pub fn get_finalized_function(&self, id: FuncId) -> *const u8 {
        self.module.get_finalized_function(id)
    }

    /// Aloca uma nova região de memória executável no pool do engine.
    /// Útil para trampolines, stubs e código gerado manualmente.
    pub fn allocate_code_region(&self, size: usize) -> Result<*const u8, JitError> {
        let mut pool = self.code_pool.write();
        let region = pool.allocate(size)?;
        Ok(region.as_ptr())
    }

    /// Retorna as estatísticas atuais de uso de memória executável (Pool).
    pub fn code_pool_stats(&self) -> CodePoolStats {
        self.code_pool.read().stats().clone()
    }
}

// ---------------------------------------------------------------------------
// Testes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Teste de sanidade: comprova que o pipeline completo funciona.
    ///
    /// Cranelift IR → código de máquina nativo x86-64/ARM64 → execução → resultado correto.
    #[test]
    fn test_cranelift_add() {
        // Criar engine JIT
        let mut engine = AlbedoJitEngine::new().expect("Falha ao criar AlbedoJitEngine");

        // Compilar função add(a, b) -> a + b
        engine
            .compile_add_function()
            .expect("Falha ao compilar função add");

        // Verificar que está no cache
        assert_eq!(engine.compiled_function_count(), 1);

        // Executar código nativo e verificar resultado
        let result = engine
            .execute_add(10, 20)
            .expect("Falha ao executar jit_add");
        assert_eq!(result, 30, "add(10, 20) deve retornar 30");

        // Testar com zero
        let result = engine.execute_add(0, 0).expect("Falha ao executar jit_add");
        assert_eq!(result, 0, "add(0, 0) deve retornar 0");

        // Testar com negativos
        let result = engine
            .execute_add(-100, 50)
            .expect("Falha ao executar jit_add");
        assert_eq!(result, -50, "add(-100, 50) deve retornar -50");

        // Testar com números grandes
        let result = engine
            .execute_add(i64::MAX - 1, 1)
            .expect("Falha ao executar jit_add");
        assert_eq!(result, i64::MAX, "add(MAX-1, 1) deve retornar MAX");
    }

    /// Teste: função não encontrada no cache retorna erro adequado.
    #[test]
    fn test_function_not_found() {
        let engine = AlbedoJitEngine::new().expect("Falha ao criar AlbedoJitEngine");
        let result = engine.execute_add(1, 2);
        assert!(
            result.is_err(),
            "Deve retornar erro para função não compilada"
        );
    }

    /// Teste: engine inicia com cache vazio.
    #[test]
    fn test_empty_cache() {
        let engine = AlbedoJitEngine::new().expect("Falha ao criar AlbedoJitEngine");
        assert_eq!(engine.compiled_function_count(), 0);
    }

    /// Teste: Integração com o CodePool.
    #[test]
    fn test_code_pool_integration() {
        let engine = AlbedoJitEngine::new().expect("Falha ao criar AlbedoJitEngine");
        let stats_before = engine.code_pool_stats();

        // Aloca 1KB
        let ptr = engine
            .allocate_code_region(1024)
            .expect("Falha ao alocar região");
        assert!(!ptr.is_null());

        let stats_after = engine.code_pool_stats();
        assert_eq!(stats_after.allocations_count, 1);
        assert!(stats_after.current_usage >= 1024);
        assert!(stats_after.total_allocated >= 1024);
        assert!(stats_after.total_allocated > stats_before.total_allocated);
    }
}
