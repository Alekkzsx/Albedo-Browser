//! # 🥊 Suíte 2: Explosão de Variáveis CSS, Resolução de Ciclos em O(V+E) e Billion Laughs
//!
//! Exercita:
//! - Detecção de ciclos diretos, mútuos e anéis de 500 nós em O(V+E)
//! - Mitigação do ataque Billion Laughs CSS (expansão exponencial bloqueada)
//! - Cadeias de fallback aninhadas com 100 níveis

use ace_core::intern::Atom;
use ace_style::computed::variables::VariableResolver;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;

#[test]
fn test_stress_direct_and_mutual_cycles() {
    let mut env = FxHashMap::default();
    // Ciclo direto
    env.insert(Atom::new("--direct"), SmolStr::new("var(--direct)"));

    // Ciclo mútuo de 2 nós
    env.insert(Atom::new("--mut-a"), SmolStr::new("var(--mut-b)"));
    env.insert(Atom::new("--mut-b"), SmolStr::new("var(--mut-a)"));

    let mut resolver = VariableResolver::new(&env);
    assert_eq!(resolver.resolve_variable(&Atom::new("--direct"), 0), None);
    assert_eq!(resolver.resolve_variable(&Atom::new("--mut-a"), 0), None);
    assert_eq!(resolver.resolve_variable(&Atom::new("--mut-b"), 0), None);
}

#[test]
fn test_stress_ring_cycle_500_nodes_linear_time() {
    let mut env = FxHashMap::default();
    let num_nodes = 500;

    for i in 0..num_nodes {
        let curr = format!("--ring-{}", i);
        let next = format!("--ring-{}", (i + 1) % num_nodes);
        env.insert(Atom::new(&curr), SmolStr::new(format!("var({})", next)));
    }

    let start = std::time::Instant::now();
    let mut resolver = VariableResolver::new(&env);
    // Deve detectar o ciclo no anel de 500 nós rapidamente em O(V+E)
    let result = resolver.resolve_variable(&Atom::new("--ring-0"), 0);
    let elapsed = start.elapsed();

    assert_eq!(result, None);
    assert!(elapsed.as_millis() < 500, "Ciclo de 500 nós deve ser detectado em milissegundos");
}

#[test]
fn test_stress_billion_laughs_css_attack_mitigation() {
    let mut env = FxHashMap::default();
    env.insert(Atom::new("--x0"), SmolStr::new("lol"));

    // Gera 30 níveis de duplicação: tamanho teórico seria 2^30 bytes (~1 GB de RAM)
    for i in 1..=30 {
        let prev = format!("--x{}", i - 1);
        let curr = format!("--x{}", i);
        env.insert(Atom::new(&curr), SmolStr::new(format!("var({}) var({})", prev, prev)));
    }

    let mut resolver = VariableResolver::new(&env);
    let start = std::time::Instant::now();
    let result = resolver.resolve_variable(&Atom::new("--x30"), 0);
    let elapsed = start.elapsed();

    // A expansão deve ser abortada com segurança devido ao limite de MAX_EXPANDED_TOKEN_LEN
    assert_eq!(result, None);
    assert!(elapsed.as_secs() < 1, "Billion laughs attack deve ser abortado instantaneamente");
}

#[test]
fn test_stress_deep_fallback_chain_100_levels() {
    let mut expr = "success_value".to_string();
    for i in (1..=100).rev() {
        expr = format!("var(--missing-{}, {})", i, expr);
    }

    let env = FxHashMap::default();
    let mut resolver = VariableResolver::new(&env);
    let result = resolver.substitute_vars_in_str(&expr, 0);

    assert_eq!(result, Some(SmolStr::new("success_value")));
}
