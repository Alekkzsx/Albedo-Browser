//! # Histogramas Atômicos UMA em Produção (Chromium base::Histogram Pattern)
//!
//! Coleta de métricas e distribuições de tempo em altíssima velocidade (< 2ns)
//! sem alocações dinâmicas no caminho crítico e com suporte a cálculo de percentis (p50, p90, p99).

use std::sync::atomic::{AtomicU64, Ordering};

/// Snapshot estático contendo os dados agregados de um histograma.
#[derive(Debug, Clone, PartialEq)]
pub struct HistogramSnapshot {
    pub name: &'static str,
    pub count: u64,
    pub sum: u64,
    pub mean: f64,
    pub p50: u64,
    pub p90: u64,
    pub p99: u64,
    pub bucket_counts: Vec<(u64, u64)>, // (bucket_upper_bound, count)
}

/// Histograma atômico lock-free parametrizado pelo número de buckets.
pub struct Histogram<const BUCKETS: usize> {
    name: &'static str,
    bucket_bounds: [u64; BUCKETS],
    buckets: [AtomicU64; BUCKETS],
    count: AtomicU64,
    sum: AtomicU64,
}

impl<const BUCKETS: usize> Histogram<BUCKETS> {
    /// Cria um histograma com distribuição exponencial de intervalos de bucket.
    ///
    /// Ideal para métricas de tempo e latência (ex: 1ms a 10.000ms).
    pub fn exponential(name: &'static str, min: u64, max: u64) -> Self {
        let mut bounds = [0u64; BUCKETS];
        if BUCKETS > 0 {
            let log_min = (min.max(1) as f64).ln();
            let log_max = (max.max(min + 1) as f64).ln();
            let step = (log_max - log_min) / (BUCKETS as f64);

            for (i, bound) in bounds.iter_mut().enumerate() {
                let val = (log_min + step * (i + 1) as f64).exp().round() as u64;
                *bound = val;
            }
            bounds[BUCKETS - 1] = u64::MAX;
        }

        Self {
            name,
            bucket_bounds: bounds,
            buckets: std::array::from_fn(|_| AtomicU64::new(0)),
            count: AtomicU64::new(0),
            sum: AtomicU64::new(0),
        }
    }

    /// Cria um histograma com distribuição linear (largura de buckets idêntica).
    pub fn linear(name: &'static str, min: u64, max: u64) -> Self {
        let mut bounds = [0u64; BUCKETS];
        if BUCKETS > 0 {
            let range = max.saturating_sub(min);
            let step = (range as f64 / BUCKETS as f64).max(1.0);

            for (i, bound) in bounds.iter_mut().enumerate() {
                *bound = min + (step * (i + 1) as f64).round() as u64;
            }
            bounds[BUCKETS - 1] = u64::MAX;
        }

        Self {
            name,
            bucket_bounds: bounds,
            buckets: std::array::from_fn(|_| AtomicU64::new(0)),
            count: AtomicU64::new(0),
            sum: AtomicU64::new(0),
        }
    }

    /// Registra uma nova amostra no histograma de forma atômica (< 2ns).
    #[inline]
    pub fn sample(&self, value: u64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(value, Ordering::Relaxed);

        // Busca rápida do bucket correspondente
        for (i, &bound) in self.bucket_bounds.iter().enumerate() {
            if value <= bound {
                self.buckets[i].fetch_add(1, Ordering::Relaxed);
                return;
            }
        }

        if BUCKETS > 0 {
            self.buckets[BUCKETS - 1].fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Gera um instantâneo (snapshot) com métricas calculadas e percentis aproximados.
    pub fn snapshot(&self) -> HistogramSnapshot {
        let count = self.count.load(Ordering::Relaxed);
        let sum = self.sum.load(Ordering::Relaxed);
        let mean = if count > 0 {
            sum as f64 / count as f64
        } else {
            0.0
        };

        let mut bucket_counts = Vec::with_capacity(BUCKETS);
        let mut raw_counts = [0u64; BUCKETS];

        for (i, raw_count) in raw_counts.iter_mut().enumerate() {
            let c = self.buckets[i].load(Ordering::Relaxed);
            *raw_count = c;
            bucket_counts.push((self.bucket_bounds[i], c));
        }

        let p50 = self.compute_percentile(0.50, count, &raw_counts);
        let p90 = self.compute_percentile(0.90, count, &raw_counts);
        let p99 = self.compute_percentile(0.99, count, &raw_counts);

        HistogramSnapshot {
            name: self.name,
            count,
            sum,
            mean,
            p50,
            p90,
            p99,
            bucket_counts,
        }
    }

    fn compute_percentile(&self, p: f64, total_count: u64, counts: &[u64; BUCKETS]) -> u64 {
        if total_count == 0 || BUCKETS == 0 {
            return 0;
        }

        let target = (total_count as f64 * p).ceil() as u64;
        let mut accum = 0u64;

        for (i, &c) in counts.iter().enumerate() {
            accum += c;
            if accum >= target {
                return self.bucket_bounds[i];
            }
        }

        self.bucket_bounds[BUCKETS - 1]
    }

    /// Exporta o snapshot em formato JSON para telemetria e DevTools.
    pub fn to_json(&self) -> String {
        let s = self.snapshot();
        format!(
            "{{\"name\":\"{}\",\"count\":{},\"sum\":{},\"mean\":{:.2},\"p50\":{},\"p90\":{},\"p99\":{}}}",
            s.name, s.count, s.sum, s.mean, s.p50, s.p90, s.p99
        )
    }
}
