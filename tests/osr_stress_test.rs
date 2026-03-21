use albedo::runtime::core::runtime::JsRuntime;
use albedo_jit::{
    AirBuilder, BytecodeRegistry, FunctionId, JitBridge, JitProfiler, ProfilerConfig,
};
use std::sync::Arc;

#[tokio::test]
async fn test_osr_100m_iterations() {
    println!("\n=== INICIANDO TESTE DE ESTRESSE OSR (100M) ===");

    // 1. Setup JIT e Runtime
    let profiler = Arc::new(JitProfiler::new(ProfilerConfig { hot_threshold: 5 }));
    let bridge = Arc::new(JitBridge::new(Arc::clone(&profiler)).expect("Falha ao criar Bridge"));
    let registry = Arc::new(BytecodeRegistry::new());

    let mut rt = JsRuntime::new_with_jit(
        Arc::clone(&bridge),
        Arc::clone(&profiler),
        Arc::clone(&registry),
    )
    .expect("Falha ao criar JsRuntime");

    // 2. Preparar o AIR (Albedo Intermediate Representation) para o loop
    // Isso simula o que o Decoder faria.
    let script_id = FunctionId("script_main".to_string());
    let mut b = AirBuilder::new("sum_100m", 0, 0);

    let n = b.emit_load_int32(100_000_000);
    let zero = b.emit_load_int32(0);
    let i = b.new_reg();
    b.emit_move(i, zero);
    let sum = b.new_reg();
    b.emit_move(sum, zero);

    let loop_header = b.create_block();
    let loop_body = b.create_block();
    let exit = b.create_block();

    b.emit_jump(loop_header);
    b.switch_block(loop_header);
    let cond = b.emit_lt(i, n);
    b.emit_jump_if(cond, loop_body, exit);

    b.switch_block(loop_body);
    let new_sum = b.emit_add(sum, i);
    b.emit_move(sum, new_sum);
    let one = b.emit_load_int32(1);
    let new_i = b.emit_add(i, one);
    b.emit_move(i, new_i);
    b.emit_jump(loop_header);

    b.switch_block(exit);
    b.emit_return(sum);

    let air = b.build();
    // Registrar o AIR no registry para ser encontrado pelo interceptor
    registry.register_air(script_id.clone(), air);

    // 3. Script JS que executa o loop pesado
    let code = r#"
        console.log("Iniciando loop de 100M no interpretador...");
        let n = 100000000;
        let i = 0;
        let sum = 0;
        while (i < n) {
            sum += i;
            i++;
            // O interrupt handler será chamado periodicamente durante o loop
        }
        console.log("Loop finalizado!");
        sum;
    "#;

    // 4. Executar e medir
    let start = std::time::Instant::now();
    let result = rt.execute_script(code);
    let duration = start.elapsed();

    println!("[TEST] Execute result: {:?}", result);
    println!("[TEST] Duração total: {:?}", duration);

    // 5. Verificações OSR
    // Como a migração OSR aborta silenciosamente o interpretador JIT para não rodar novamente,
    // o JS resultará num Erro "Exception" (interrupted).
    assert!(
        result.is_err(),
        "Deveria ter retornado uma exceção de interrupção após o OSR"
    );

    // Lemos o resultado salvo pelo Interceptor (comunicação limpa de C -> Rust -> Bridge)
    let final_result = rt
        .interceptor
        .last_osr_result
        .load(std::sync::atomic::Ordering::Acquire);
    println!(
        "[TEST] Resultado salvo pelo OSR Interceptor: {}",
        final_result
    );

    // O resultado de sum(0..100M) deve ser correto. S = (n * (n-1)) / 2
    // S = (100,000,000 * 99,999,999) / 2 = 4,999,999,950,000,000
    assert_eq!(
        final_result, 4999999950000000,
        "O resultado matemático do JIT OSR está incorreto!"
    );

    // Se o OSR funcionou, o tempo deve ser em torno de 1 a 2 segundos em debug/baseline
    assert!(
        duration.as_secs() < 5,
        "OSR parece não ter acelerado o loop o suficiente!"
    );
}
