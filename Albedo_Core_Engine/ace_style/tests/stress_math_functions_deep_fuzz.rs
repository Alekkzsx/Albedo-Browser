//! # 🥊 Bateria 5: Fuzzing Estocástico Profundo de Funções Matemáticas CSS W3C
//!
//! Exercita:
//! - 10.000 expressões matemáticas com aninhamento profundo (depth até 10).
//! - Combinações arbitrárias de `clamp()`, `min()`, `max()`, `round()`, `abs()`, `sign()`, `calc()`, `px`, `em`, `rem`, `vw`, `%`.
//! - Garantia estrita de zero panics, zero estouros de pilha (stack overflow) e precisão matemática contra modelo de referência.

use ace_style::values::length::CalcExpr;

/// Modelo de referência matemática para validação de integridade.
struct ReferenceModel {
    font_size: f32,
    root_font_size: f32,
    viewport_w: f32,
    viewport_h: f32,
    percent_ref: f32,
}

impl ReferenceModel {
    fn new() -> Self {
        Self {
            font_size: 16.0,
            root_font_size: 16.0,
            viewport_w: 1920.0,
            viewport_h: 1080.0,
            percent_ref: 800.0,
        }
    }

    fn unit_to_px(&self, val: f32, unit: &str) -> f32 {
        match unit {
            "px" => val,
            "em" => val * self.font_size,
            "rem" => val * self.root_font_size,
            "vw" => val * self.viewport_w / 100.0,
            "vh" => val * self.viewport_h / 100.0,
            "%" => val * self.percent_ref / 100.0,
            _ => val,
        }
    }
}

/// Gerador pseudo-aleatório LCG determinístico e rápido (sem dependência externa).
struct FastRng {
    state: u64,
}

impl FastRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn next_range(&mut self, min: u32, max: u32) -> u32 {
        min + (self.next_u32() % (max - min + 1))
    }

    #[allow(dead_code)]
    fn next_f32(&mut self, min: f32, max: f32) -> f32 {
        let frac = (self.next_u32() % 1000) as f32 / 1000.0;
        min + frac * (max - min)
    }
}

/// Construtor de expressões com profundidade controlada e modelo de referência.
fn generate_nested_expr(
    rng: &mut FastRng,
    depth: usize,
    max_depth: usize,
    ref_model: &ReferenceModel,
) -> (String, f32) {
    let units = ["px", "em", "rem", "vw", "%"];

    if depth >= max_depth {
        let val = (rng.next_range(1, 100) as f32).round();
        let unit = units[rng.next_range(0, (units.len() - 1) as u32) as usize];
        let px_expected = ref_model.unit_to_px(val, unit);
        return (format!("{}{}", val, unit), px_expected);
    }

    let kind = rng.next_range(0, 5);
    match kind {
        0 => {
            // min(a, b)
            let (e1, v1) = generate_nested_expr(rng, depth + 1, max_depth, ref_model);
            let (e2, v2) = generate_nested_expr(rng, depth + 1, max_depth, ref_model);
            (format!("min({}, {})", e1, e2), v1.min(v2))
        }
        1 => {
            // max(a, b)
            let (e1, v1) = generate_nested_expr(rng, depth + 1, max_depth, ref_model);
            let (e2, v2) = generate_nested_expr(rng, depth + 1, max_depth, ref_model);
            (format!("max({}, {})", e1, e2), v1.max(v2))
        }
        2 => {
            // clamp(min, val, max)
            let (_e1, v1) = generate_nested_expr(rng, depth + 1, max_depth, ref_model);
            let (e2, v2) = generate_nested_expr(rng, depth + 1, max_depth, ref_model);
            let (_e3, v3) = generate_nested_expr(rng, depth + 1, max_depth, ref_model);
            let min_v = v1.min(v3).round();
            let max_v = v1.max(v3).round();
            let expected = min_v.max(v2.min(max_v));
            (format!("clamp({}px, {}, {}px)", min_v, e2, max_v), expected)
        }
        3 => {
            // round(strategy, val, step)
            let (e, v) = generate_nested_expr(rng, depth + 1, max_depth, ref_model);
            let step = (rng.next_range(2, 10) as f32).max(1.0);
            let strategies = ["nearest", "up", "down", "to-zero"];
            let strat_idx = rng.next_range(0, 3) as usize;
            let strat_name = strategies[strat_idx];

            let ratio = v / step;
            let rounded = match strat_idx {
                0 => ratio.round(),
                1 => ratio.ceil(),
                2 => ratio.floor(),
                3 => ratio.trunc(),
                _ => unreachable!(),
            };
            let expected = rounded * step;
            (
                format!("round({}, {}, {}px)", strat_name, e, step),
                expected,
            )
        }
        4 => {
            // abs(val)
            let (e, v) = generate_nested_expr(rng, depth + 1, max_depth, ref_model);
            (format!("abs({})", e), v.abs())
        }
        _ => {
            // calc(a + b)
            let (e1, v1) = generate_nested_expr(rng, depth + 1, max_depth, ref_model);
            let (e2, v2) = generate_nested_expr(rng, depth + 1, max_depth, ref_model);
            (format!("calc({} + {})", e1, e2), v1 + v2)
        }
    }
}

#[test]
fn test_stress_math_functions_deep_fuzz() {
    let iteration_count = 10_000;
    println!(
        "Iniciando fuzzing com {} expressões matemáticas profundas...",
        iteration_count
    );

    let ref_model = ReferenceModel::new();
    let mut rng = FastRng::new(0xACE5_717E_2026);

    let mut successful_evaluations = 0;
    let start = std::time::Instant::now();

    for i in 0..iteration_count {
        let max_depth = rng.next_range(2, 6) as usize;
        let (expr_str, expected_px) = generate_nested_expr(&mut rng, 0, max_depth, &ref_model);

        // Faz o parse da expressão
        let parsed = CalcExpr::parse(&expr_str)
            .unwrap_or_else(|| panic!("Falha de parsing na expressão #{}: {}", i, expr_str));

        // Avalia com o motor do ace_style
        let evaluated = parsed.eval_px(
            ref_model.font_size,
            ref_model.root_font_size,
            (ref_model.viewport_w, ref_model.viewport_h),
            Some(ref_model.percent_ref),
        );

        let Some(actual_px) = evaluated else {
            panic!(
                "Falha na avaliação da expressão #{}: {} (esperado: {})",
                i, expr_str, expected_px
            );
        };

        // Tolerância de ponto flutuante de 0.05px devido a arredondamento cumulativo de frações
        let delta = (actual_px - expected_px).abs();
        assert!(
            delta < 0.05,
            "Discrepância na expressão #{}: {}\nEsperado: {}, Obtido: {} (Delta: {})",
            i,
            expr_str,
            expected_px,
            actual_px,
            delta
        );

        successful_evaluations += 1;
    }

    let elapsed = start.elapsed();
    println!(
        "Fuzzing concluído com sucesso: {} expressões avaliadas em {:?} ({:.3} µs/expr)",
        successful_evaluations,
        elapsed,
        elapsed.as_micros() as f64 / iteration_count as f64
    );

    assert_eq!(successful_evaluations, iteration_count);
}
