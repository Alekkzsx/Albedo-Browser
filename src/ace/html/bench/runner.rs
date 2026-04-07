//! Benchmark runner with warmup, measurement, and baseline comparison

use std::time::{Duration, Instant};
use super::stats::BenchStats;

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchConfig {
    pub warmup_iterations: usize,
    pub measurement_iterations: usize,
    pub name: String,
    pub baseline: Option<BenchStats>, // For regression detection
}

impl BenchConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            warmup_iterations: 10,
            measurement_iterations: 100,
            name: name.into(),
            baseline: None,
        }
    }
    
    pub fn with_warmup(mut self, iterations: usize) -> Self {
        self.warmup_iterations = iterations;
        self
    }
    
    pub fn with_measurements(mut self, iterations: usize) -> Self {
        self.measurement_iterations = iterations;
        self
    }
    
    pub fn with_baseline(mut self, baseline: BenchStats) -> Self {
        self.baseline = Some(baseline);
        self
    }
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self::new("benchmark")
    }
}

/// Benchmark result with comparison to baseline
#[derive(Debug, Clone)]
pub struct BenchResult {
    pub name: String,
    pub stats: BenchStats,
    pub comparison: Option<Comparison>,
}

/// Comparison with baseline
#[derive(Debug, Clone)]
pub struct Comparison {
    pub baseline_mean: Duration,
    pub current_mean: Duration,
    pub speedup: f64, // > 1.0 means faster, < 1.0 means slower
    pub is_regression: bool,
    pub regression_threshold: f64, // Default 5% slower
}

impl Comparison {
    pub fn new(baseline: &BenchStats, current: &BenchStats) -> Self {
        let baseline_secs = baseline.mean.as_secs_f64();
        let current_secs = current.mean.as_secs_f64();
        
        let speedup = if current_secs > 0.0 {
            baseline_secs / current_secs
        } else {
            1.0
        };
        
        let regression_threshold = 0.05; // 5% slower is a regression
        let is_regression = speedup < (1.0 - regression_threshold);
        
        Self {
            baseline_mean: baseline.mean,
            current_mean: current.mean,
            speedup,
            is_regression,
            regression_threshold,
        }
    }
    
    pub fn percentage_change(&self) -> f64 {
        (self.speedup - 1.0) * 100.0
    }
}

/// Benchmark runner
pub struct BenchRunner {
    config: BenchConfig,
}

impl BenchRunner {
    pub fn new(config: BenchConfig) -> Self {
        Self { config }
    }
    
    /// Run benchmark with given function
    pub fn run<F>(&self, mut f: F) -> BenchResult
    where
        F: FnMut(),
    {
        // Warmup phase
        for _ in 0..self.config.warmup_iterations {
            f();
        }
        
        // Measurement phase
        let mut samples = Vec::with_capacity(self.config.measurement_iterations);
        for _ in 0..self.config.measurement_iterations {
            let start = Instant::now();
            f();
            let elapsed = start.elapsed();
            samples.push(elapsed);
        }
        
        let stats = BenchStats::from_samples(samples);
        
        // Compare with baseline if available
        let comparison = self.config.baseline.as_ref().map(|baseline| {
            Comparison::new(baseline, &stats)
        });
        
        BenchResult {
            name: self.config.name.clone(),
            stats,
            comparison,
        }
    }
    
    /// Print benchmark result
    pub fn print_result(&self, result: &BenchResult) {
        println!("\n{}", "=".repeat(60));
        println!("Benchmark: {}", result.name);
        println!("{}", "=".repeat(60));
        println!("  Mean:     {:?}", result.stats.mean);
        println!("  Median:   {:?}", result.stats.median);
        println!("  Std Dev:  {:?}", result.stats.std_dev);
        println!("  P95:      {:?}", result.stats.p95);
        println!("  P99:      {:?}", result.stats.p99);
        println!("  Min:      {:?}", result.stats.min);
        println!("  Max:      {:?}", result.stats.max);
        println!("  CV:       {:.2}%", result.stats.coefficient_of_variation());
        println!("  Outliers: {} ({:.1}%)", 
            result.stats.outliers.len(),
            result.stats.outlier_percentage()
        );
        
        if result.stats.is_stable() {
            println!("  Status:   ✓ Stable");
        } else {
            println!("  Status:   ⚠ Unstable (CV > 5%)");
        }
        
        if let Some(comp) = &result.comparison {
            println!("\n  Baseline Comparison:");
            println!("    Baseline: {:?}", comp.baseline_mean);
            println!("    Current:  {:?}", comp.current_mean);
            println!("    Speedup:  {:.2}x", comp.speedup);
            println!("    Change:   {:+.2}%", comp.percentage_change());
            
            if comp.is_regression {
                println!("    Status:   ❌ REGRESSION DETECTED");
            } else if comp.speedup > 1.05 {
                println!("    Status:   ✓ Performance improved");
            } else {
                println!("    Status:   ✓ No significant change");
            }
        }
        
        println!("{}", "=".repeat(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bench_runner_basic() {
        let config = BenchConfig::new("test")
            .with_warmup(5)
            .with_measurements(10);
        
        let runner = BenchRunner::new(config);
        
        let result = runner.run(|| {
            // Simulate some work
            std::thread::sleep(Duration::from_micros(100));
        });
        
        assert_eq!(result.name, "test");
        assert_eq!(result.stats.samples.len(), 10);
        assert!(result.stats.mean > Duration::ZERO);
    }
    
    #[test]
    fn test_baseline_comparison() {
        let baseline_samples = vec![Duration::from_millis(10); 10];
        let baseline = BenchStats::from_samples(baseline_samples);
        
        let config = BenchConfig::new("test")
            .with_warmup(1)
            .with_measurements(10)
            .with_baseline(baseline);
        
        let runner = BenchRunner::new(config);
        
        let result = runner.run(|| {
            std::thread::sleep(Duration::from_millis(5)); // Faster than baseline
        });
        
        assert!(result.comparison.is_some());
        let comp = result.comparison.unwrap();
        assert!(comp.speedup > 1.0); // Should be faster
        assert!(!comp.is_regression);
    }
}
