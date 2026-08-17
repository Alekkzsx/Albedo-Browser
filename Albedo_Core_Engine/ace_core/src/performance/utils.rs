//! # Utilitários Estatísticos e Formatação de Performance
//!
//! Funções para cálculo de percentis de latência (P50, P95, P99) e formatação de durações de benchmarks.

/// Estatísticas de latência e duração calculadas sobre um conjunto de medições.
#[derive(Debug, Clone, PartialEq)]
pub struct Percentiles {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub samples: usize,
}

/// Calcula o resumo estatístico e percentis para uma lista de medições de tempo em milissegundos.
pub fn calculate_percentiles(samples: &[f64]) -> Option<Percentiles> {
    if samples.is_empty() {
        return None;
    }

    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = sorted.len();
    let min = sorted[0];
    let max = sorted[len - 1];
    let sum: f64 = sorted.iter().sum();
    let mean = sum / len as f64;

    let p50 = percentile_of_sorted(&sorted, 0.50);
    let p90 = percentile_of_sorted(&sorted, 0.90);
    let p95 = percentile_of_sorted(&sorted, 0.95);
    let p99 = percentile_of_sorted(&sorted, 0.99);

    Some(Percentiles {
        min,
        max,
        mean,
        p50,
        p90,
        p95,
        p99,
        samples: len,
    })
}

#[inline]
fn percentile_of_sorted(sorted: &[f64], pct: f64) -> f64 {
    let idx = ((sorted.len() as f64 * pct).ceil() as usize).saturating_sub(1);
    sorted[idx.min(sorted.len() - 1)]
}

/// Formata uma duração em milissegundos de maneira legível para humanos (s, ms, µs).
pub fn format_duration_human(duration_ms: f64) -> String {
    if duration_ms >= 1000.0 {
        format!("{:.2} s", duration_ms / 1000.0)
    } else if duration_ms >= 1.0 {
        format!("{:.2} ms", duration_ms)
    } else {
        format!("{:.2} µs", duration_ms * 1000.0)
    }
}
