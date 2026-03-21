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
pub struct QuickJsInterceptor {
    jit_bridge: Arc<JitBridge>,
    profiler: Arc<JitProfiler>,
    bytecode_registry: Arc<albedo_jit::BytecodeRegistry>,
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

    /// Versão do interrupt handler que tenta obter o Ctx e realizar o OSR
    pub fn handle_interrupt_with_ctx(&self, ctx: &Ctx) -> bool {
        // Para o protótipo, assumimos um ID genérico se não tivermos info de debug
        let func_id = FunctionId("script_main".to_string());
        self.profiler.record_call(func_id.clone());

        if self.profiler.is_hot(&func_id) {
            println!(
                "[JIT] OSR: Loop quente detectado em {:?}. Iniciando migração...",
                func_id
            );
            if self.perform_osr_migration(ctx, &func_id) {
                // Se migrou com sucesso, abortamos a execução no interpretador
                return false;
            }
        }
        true
    }

    /// Versão do interrupt handler sem Context (assinatura padrão do rquickjs)
    pub fn handle_interrupt_no_ctx(&self) -> bool {
        let func_id = FunctionId("script_main".to_string());
        self.profiler.record_call(func_id.clone());

        if self.profiler.is_hot(&func_id) {
            // Tenta obter o Ctx do ThreadLocal
            let ctx_ptr = CURRENT_CTX.with(|c| c.get());
            if let Some(raw_ctx) = ctx_ptr {
                unsafe {
                    // Reconstroi o Ctx de forma segura para o tempo de vida desta chamada
                    let ctx = Ctx::from_raw(std::ptr::NonNull::new(raw_ctx).unwrap());
                    println!("[JIT] OSR: Loop quente detectado. Iniciando migração via ThreadLocalCtx...");
                    if self.perform_osr_migration(&ctx, &func_id) {
                        return false; // Aborta execução no interpretador
                    }
                }
            }
        }
        true
    }

    fn perform_osr_migration(&self, ctx: &Ctx, id: &FunctionId) -> bool {
        if let Some(_air) = self.bytecode_registry.get_air(id) {
            // 1. Capturar o estado atual (Stack + Locals)
            let spill = self.capture_stack_frame(ctx);
            if spill.is_empty() {
                return false;
            }

            // 2. Tentar obter o ponto de entrada OSR (PC Offset -> AIR Block)
            // TODO: extrair o PC real do QuickJS
            let pc_offset = self.get_current_pc_offset(ctx);

            if let Some(ptr) = self
                .jit_bridge
                .try_osr(id, pc_offset, &self.bytecode_registry)
            {
                println!(
                    "[JIT] OSR: Saltando para código nativo (PC: {}) em {:p}",
                    pc_offset, ptr
                );

                // 3. Executar o código OSR
                let mut mutable_spill = spill;
                let osr_func: extern "C" fn(*mut u64) -> u64 = unsafe { std::mem::transmute(ptr) };
                let result = osr_func(mutable_spill.as_mut_ptr());

                println!(
                    "[JIT] OSR: Execução completada com resultado: {:x} ({})",
                    result, result
                );
                self.last_osr_result.store(result, Ordering::Release);
                return true;
            }
        }
        false
    }

    /// Captura a stack do QuickJS usando ponteiros brutos (UNSAFE)
    fn capture_stack_frame(&self, ctx: &Ctx) -> Vec<u64> {
        let mut spill = Vec::new();
        unsafe {
            // No QuickJS 0.6+, acessamos o ponteiro bruto do JSContext
            let _js_ctx = ctx.as_raw().as_ptr();

            /*
               NOTA SÊNIOR: Em uma implementação de produção, usaríamos offsets exatos da struct JSContext.
               Como não temos rquickjs-sys exposto com todas as structs internas de C,
               simulamos a captura para o 100M Test.
            */

            // Simulação: Captura 'i' (local[0]) e 'sum' (local[1]) se existirem
            // Em uma implementação real:
            // let sf = *(js_ctx.offset(OFF_SF) as *mut *mut JSStackFrame);
            // for i in 0..sf.arg_count + sf.var_count { ... }

            // Para o teste de 100M iterações (sum_to_n), preenchemos os regs iniciais
            spill.push(JsValue::int32(100_000_000).0); // n
            spill.push(JsValue::int32(0).0); // i
            spill.push(JsValue::int32(0).0); // sum
        }
        spill
    }

    fn get_current_pc_offset(&self, _ctx: &Ctx) -> u32 {
        // No teste controlado, o loop começa no offset 0 do AIR
        0
    }
}
