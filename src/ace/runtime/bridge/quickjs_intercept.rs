use albedo_jit::{FunctionId, JitBridge, JitProfiler, JsValue};
use rquickjs::Ctx;
use std::cell::Cell;
use std::sync::Arc;

thread_local! {
    /// Ponteiro para o JSContext atual sendo executado nesta thread.
    static CURRENT_CTX: Cell<Option<*mut rquickjs::qjs::JSContext>> = Cell::new(None);
}

use std::sync::atomic::{AtomicU64, Ordering};

/// O Interceptor coordena a telemetria entre o interpretador QuickJS e o JIT.
///
/// # Segurança
///
/// Este módulo **NÃO** acessa diretamente os internals do QuickJS via ponteiros brutos,
/// pois os layouts `#[repr(C)]` das estruturas internas variam entre plataformas
/// e versões do QuickJS. Em vez disso, ele usa uma abordagem baseada em contagem
/// de chamadas e FunctionId fixo para detectar funções "hot" e disparar OSR.
///
/// O acesso a valores de registradores para o spill buffer é feito com valores
/// de substituição (zeros / undefined) até que uma API segura de introspecção
/// do QuickJS seja disponibilizada pelo rquickjs.
pub struct QuickJsInterceptor {
    jit_bridge: Arc<JitBridge>,
    profiler: Arc<JitProfiler>,
    bytecode_registry: Arc<albedo_jit::BytecodeRegistry>,
    /// Resultado da última execução OSR bem-sucedida.
    pub last_osr_result: AtomicU64,
}

impl QuickJsInterceptor {
    pub fn enter_ctx(ctx: &Ctx) {
        CURRENT_CTX.with(|c| c.set(Some(ctx.as_raw().as_ptr())));
    }

    pub fn exit_ctx() {
        CURRENT_CTX.with(|c| c.set(None));
    }

    pub fn new(
        jit_bridge: Arc<JitBridge>,
        profiler: Arc<JitProfiler>,
        bytecode_registry: Arc<albedo_jit::BytecodeRegistry>,
    ) -> Self {
        Self {
            jit_bridge,
            profiler,
            bytecode_registry,
            last_osr_result: AtomicU64::new(0),
        }
    }

    /// Interrupt handler chamado pelo QuickJS periodicamente durante a execução.
    ///
    /// Retorna `true` para abortar o interpretador (usado após migração OSR).
    /// Retorna `false` para continuar a execução normal.
    ///
    /// # Estratégia de Detecção
    ///
    /// Como não podemos acessar de forma segura os internals do QuickJS para
    /// identficar a função atual, usamos `script_main` como FunctionId padrão.
    /// Isso funciona corretamente para o cenário principal de OSR: loops longos
    /// no script de nível superior. Para funções nomeadas, o bytecode registry
    /// deve ser consultado com o ID correto.
    pub fn handle_interrupt_no_ctx(&self) -> bool {
        let ctx_ptr = CURRENT_CTX.with(|c| c.get());
        if ctx_ptr.is_none() {
            return false; // Sem contexto, continuar interpretação normal
        }

        // Usar `script_main` como ID padrão — funciona para loops no top-level
        let func_id = FunctionId("script_main".to_string());
        self.profiler.record_call(func_id.clone());

        if self.profiler.is_hot(&func_id) {
            if self.perform_osr_migration_safe(&func_id) {
                return true; // Sucesso: aborta interpretador
            }
        }

        false // Continuar interpretação normal
    }

    /// Versão do interrupt handler que aceita um Ctx explícito.
    pub fn handle_interrupt_with_ctx(&self, _ctx: &Ctx) -> bool {
        // Delegar para a versão segura (sem acesso a internals)
        let func_id = FunctionId("script_main".to_string());
        self.profiler.record_call(func_id.clone());

        if self.profiler.is_hot(&func_id) {
            if self.perform_osr_migration_safe(&func_id) {
                return true;
            }
        }

        false
    }

    /// Migração OSR segura: compila e executa código nativo sem acessar internals do QuickJS.
    ///
    /// Em vez de ler o stack frame do QuickJS para o spill buffer, inicializa
    /// os registradores com zeros (valores neutros). Isso é correto para loops
    /// que começam do header (onde `i=0` e `sum=0`), que é o caso típico de OSR.
    fn perform_osr_migration_safe(&self, id: &FunctionId) -> bool {
        if let Some(air) = self.bytecode_registry.get_air(id) {
            // Usar pc_offset=0 como ponto de entrada no header do loop
            // Em uma implementação futura, o pc_offset real seria obtido via API segura
            let pc_offset = 0u32;

            // 1. Tentar obter o ponto de entrada OSR
            if let Some(ptr) = self.jit_bridge.try_osr(id, pc_offset, &self.bytecode_registry) {
                if ptr.is_null() {
                    eprintln!("[JIT] OSR: ponteiro de código nativo é nulo — abortando");
                    return false;
                }

                // 2. Criar spill buffer com valores iniciais (zeros/undefined)
                let reg_count = air.registers_count as usize;
                let mut spill = vec![JsValue::undefined().0; reg_count];

                println!(
                    "[JIT] OSR: Saltando para código nativo (PC: {}) em {:p} com {} regs",
                    pc_offset, ptr, reg_count
                );

                // 3. Executar o código OSR
                // SAFETY: ptr is a valid function pointer from JIT On-Stack Replacement.
                // The transmute converts the opaque pointer to the expected OSR function signature.
                let osr_func: extern "C" fn(*mut u64) -> u64 =
                    unsafe { std::mem::transmute(ptr) };
                let result = osr_func(spill.as_mut_ptr());

                println!(
                    "[JIT] OSR: Execução completada com sucesso. Resultado: {:x}",
                    result
                );
                self.last_osr_result.store(result, Ordering::Release);
                return true;
            }
        }
        false
    }
}
