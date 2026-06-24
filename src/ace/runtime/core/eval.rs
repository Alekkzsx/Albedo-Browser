use super::runtime::{JsResult, JsRuntime};
use rquickjs::Value;

/// TODO: add docs
pub fn execute_script(rt: &JsRuntime, code: &str) -> JsResult<String> {
    tracing::info!(len = code.len(), "Executing script");

    // AlbedoJIT Profiler Hook (Top-level script invocation)
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    code.hash(&mut hasher);
    let script_id = albedo_jit::FunctionId(format!("script_{}", hasher.finish()));
    rt.profiler.record_call(script_id.clone());

    // AlbedoJIT Bridge: Tenta rodar código nativo
    rt.jit_bridge.compile_pending(&rt.bytecode_registry);
    if let Some(ptr) = rt.jit_bridge.try_native(&script_id) {
        tracing::info!(?script_id, "Executing JIT native version");
        // SAFETY: Execução direta de função JIT sem argumentos (top-level script)
        let func: extern "C" fn() = unsafe { std::mem::transmute(ptr) };
        func();
        return Ok("JIT_NATIVE_EXECUTION_SUCCESS".to_string());
    }

    let ctx = rt.context.lock().unwrap_or_else(|e| e.into_inner());
    ctx.with(|ctx| {
        // Registrar contexto para OSR
        crate::ace::runtime::bridge::quickjs_intercept::QuickJsInterceptor::enter_ctx(&ctx);

        let result = match ctx.eval::<Value, _>(code) {
            Ok(result) => {
                tracing::info!("Script execution success");
                // Try to convert to string, fallback to debug
                if let Some(s) = result.as_string() {
                    Ok(s.to_string()?)
                } else if result.is_null() {
                    Ok("null".to_string())
                } else if result.is_undefined() {
                    Ok("undefined".to_string())
                } else {
                    // Try JSON stringify
                    match ctx.json_stringify(result.clone()) {
                        Ok(Some(s)) => Ok(s.to_string()?),
                        _ => Ok(format!("{:?}", result)),
                    }
                }
            }
            Err(e) => {
                tracing::error!("Script execution FAILED");
                let exception_val = ctx.catch();
                if let Some(obj) = exception_val.as_object() {
                    let mut msg = String::new();
                    if let Ok(message) = obj.get::<_, String>("message") {
                        msg.push_str(&message);
                    } else {
                        msg.push_str(&format!("{:?}", exception_val));
                    }

                    if let Ok(stack) = obj.get::<_, String>("stack") {
                        msg.push_str("\nStack:\n");
                        msg.push_str(&stack);
                    }
                    tracing::error!(msg = %msg, "JS exception");
                } else {
                    tracing::error!(?exception_val, "JS exception");
                }
                Err(e)
            }
        };

        // Limpar contexto
        crate::ace::runtime::bridge::quickjs_intercept::QuickJsInterceptor::exit_ctx();
        result
    })
}

/// Avalia um módulo ES inline no contexto QuickJS.
///
/// Usa `Module::evaluate` (static method) para processar código
/// com `import`/`export` statements.
///
/// # Arguments
/// * `rt` - Referência ao JsRuntime
/// * `code` - Código-fonte do módulo
/// * `module_name` - Nome canônico do módulo (URL absoluta ou nome inline)
pub fn execute_module(rt: &JsRuntime, code: &str, module_name: &str) -> JsResult<String> {
    tracing::info!(module = module_name, len = code.len(), "Executing ES Module");

    // AlbedoJIT Profiler Hook (Módulos usando o nome canônico)
    let mod_id = albedo_jit::FunctionId(format!("module_{}", module_name));
    // AlbedoJIT Bridge: Tenta rodar código nativo
    rt.jit_bridge.compile_pending(&rt.bytecode_registry);
    if let Some(ptr) = rt.jit_bridge.try_native(&mod_id) {
        tracing::info!(?mod_id, "Executing JIT native module");
        // SAFETY: ptr is a valid function pointer from JIT compilation.
        // The transmute converts the opaque pointer to the expected function signature.
        let func: extern "C" fn() = unsafe { std::mem::transmute(ptr) };
        func();
        return Ok("JIT_NATIVE_MODULE_SUCCESS".to_string());
    }

    let ctx = rt.context.lock().unwrap_or_else(|e| e.into_inner());
    ctx.with(|ctx| {
        // Registrar contexto para OSR
        crate::ace::runtime::bridge::quickjs_intercept::QuickJsInterceptor::enter_ctx(&ctx);

        // Module::evaluate é o método estático que declara E avalia o módulo
        let result = match rquickjs::Module::evaluate(ctx.clone(), module_name, code) {
            Ok(promise) => {
                // Executar microtasks pendentes para resolver a promise do módulo
                while ctx.execute_pending_job() {}

                // .finish::<()>() tenta resolver a promise e obter o resultado
                match promise.finish::<()>() {
                    Ok(_) => {
                        tracing::info!(module = module_name, "ES Module evaluated successfully");
                        Ok("ok".to_string())
                    }
                    Err(e) => {
                        tracing::error!(module = module_name, ?e, "ES Module evaluation error");
                        // Capturar exceção detalhada
                        let exception_val = ctx.catch();
                        log_js_exception(&exception_val);
                        Err(e)
                    }
                }
            }
            Err(e) => {
                tracing::error!(module = module_name, "ES Module evaluate() failed");
                let exception_val = ctx.catch();
                log_js_exception(&exception_val);
                Err(e)
            }
        };

        // Limpar contexto
        crate::ace::runtime::bridge::quickjs_intercept::QuickJsInterceptor::exit_ctx();
        result
    })
}

/// Avalia um módulo ES externo a partir de sua URL.
/// Primeiro tenta o cache do ModuleRegistry, depois faz fetch HTTP.
pub fn execute_module_from_url(rt: &JsRuntime, url: &str) -> JsResult<String> {
    tracing::info!(url = %url, "Loading ES Module from URL");

    // Verificar se já foi avaliado
    {
        let registry = rt.module_registry.lock().unwrap_or_else(|e| e.into_inner());
        if registry.is_evaluated(url) {
            tracing::info!(url = %url, "Module already evaluated, skipping");
            return Ok("already_evaluated".to_string());
        }
    }

    // Verificar se o código já está no cache
    let source = {
        let registry = rt.module_registry.lock().unwrap_or_else(|e| e.into_inner());
        registry.get_source(url)
    };

    if let Some(code) = source {
        let result = execute_module(rt, &code, url);
        if result.is_ok() {
            let registry = rt.module_registry.lock().unwrap_or_else(|e| e.into_inner());
            registry.mark_evaluated(url);
        }
        return result;
    }

    // Se não está no cache, tentar fazer download
    if let Some(code) = crate::ace::runtime::core::module_loader::fetch_module_source_public(url) {
        // Registrar no cache
        {
            let registry = rt.module_registry.lock().unwrap_or_else(|e| e.into_inner());
            registry.register_external(url, code.clone());
        }
        let result = execute_module(rt, &code, url);
        if result.is_ok() {
            let registry = rt.module_registry.lock().unwrap_or_else(|e| e.into_inner());
            registry.mark_evaluated(url);
        }
        return result;
    }

    tracing::error!(url = %url, "Failed to fetch module from URL");
    Err(rquickjs::Error::new_loading(url))
}

/// Log detalhado de exceções JS
fn log_js_exception(exception_val: &Value) {
    if let Some(obj) = exception_val.as_object() {
        let mut msg = String::new();
        if let Ok(message) = obj.get::<_, String>("message") {
            msg.push_str(&message);
        } else {
            msg.push_str(&format!("{:?}", exception_val));
        }
        if let Ok(stack) = obj.get::<_, String>("stack") {
            msg.push_str("\nStack:\n");
            msg.push_str(&stack);
        }
        tracing::error!(msg = %msg, "JS module exception");
    } else {
        tracing::error!(?exception_val, "JS module exception");
    }
}
