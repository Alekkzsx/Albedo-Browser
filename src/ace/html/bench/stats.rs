//! Statistical analysis for benchmark results

use std::time::Duration;

/// Statistical analysis results
#[derive(Debug, Clone)]
pub struct BenchStats {
    pub samples: Vec<Duration>,
    pub mean: Duration,
    pub median: Duration,
    pub std_dev: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub min: Duration,
    pub max: Duration,
    pub outliers: Vec<usize>, // Indices of outlier samples
}

impl BenchStats {
    /// Create statistics from raw samples
    pub fn from_samples(mut samples: Vec<Duration>) -> Self {
        if samples.is_empty() {
            return Self::empty();
        }
        
        samples.sort();
        
        let mean = Self::calculate_mean(&samples);
        let median = Self::calculate_median(&samples);
        let std_dev = Self::calculate_std_dev(&samples, mean);
        let p95 = Self::calculate_percentile(&samples, 0.95);
        let p99 = Self::calculate_percentile(&samples, 0.99);
        let min = samples[0];
        let max = samples[samples.len() - 1];
        let outliers = Self::detect_outliers(&samples);
        
        Self {
            samples,
            mean,
            median,
            std_dev,
            p95,
            p99,
            min,
            max,
            outliers,
        }
    }
    
    fn empty() -> Self {
        Self {
            samples: Vec::new(),
            mean: Duration::ZERO,
            median: Duration::ZERO,
            std_dev: Duration::ZERO,
            p95: Duration::ZERO,
            p99: Duration::ZERO,
            min: Duration::ZERO,
            max: Duration::ZERO,
            outliers: Vec::new(),
        }
    }
    
    fn calculate_mean(samples: &[Duration]) -> Duration {
        let sum: Duration = samples.iter().sum();
        sum / samples.len() as u32
    }
    
    fn calculate_median(samples: &[Duration]) -> Duration {
        let len = samples.len();
        if len % 2 == 0 {
            let mid = len / 2;
            (samples[mid - 1] + samples[mid]) / 2
        } else {
            samples[len / 2]
        }
    }
    
    fn calculate_std_dev(samples: &[Duration], mean: Duration) -> Duration {
        if samples.len() < 2 {
            return Duration::ZERO;
        }
        
        let mean_secs = mean.as_secs_f64();
        let variance: f64 = samples
            .iter()
            .map(|d| {
                let diff = d.as_secs_f64() - mean_secs;
                diff * diff
            })
            .sum::<f64>()
            / (samples.len() - 1) as f64;
        
        Duration::from_secs_f64(variance.sqrt())
    }
    
    fn calculate_percentile(samples: &[Duration], percentile: f64) -> Duration {
        let idx = ((samples.len() as f64 - 1.0) * percentile) as usize;
        samples[idx.min(samples.len() - 1)]
    }
    
    /// Detect outliers using IQR (Interquartile Range) method
    fn detect_outliers(samples: &[Duration]) -> Vec<usize> {
        if samples.len() < 4 {
            return Vec::new();
        }
        
        let q1 = Self::calculate_percentile(samples, 0.25);
        let q3 = Self::calculate_percentile(samples, 0.75);
        let iqr = q3 - q1;
        
        let lower_bound = q1.saturating_sub(iqr + iqr / 2); // Q1 - 1.5 * IQR
        let upper_bound = q3 + iqr + iqr / 2; // Q3 + 1.5 * IQR
        
        samples
            .iter()
            .enumerate()
            .filter_map(|(i, &sample)| {
                if sample < lower_bound || sample > upper_bound {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Get coefficient of variation (CV) as percentage
    pub fn coefficient_of_variation(&self) -> f64 {
        if self.mean.as_secs_f64() == 0.0 {
            return 0.0;
        }
        (self.std_dev.as_secs_f64() / self.mean.as_secs_f64()) * 100.0
    }
    
    /// Check if results are stable (CV < 5%)
    pub fn is_stable(&self) -> bool {
        self.coefficient_of_variation() < 5.0
    }
    
    /// Get outlier percentage
    pub fn outlier_percentage(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        (self.outliers.len() as f64 / self.samples.len() as f64) * 100.0
    }
}

/// Trait for statistical analysis
pub trait StatisticalAnalysis {
    fn analyze(&self) -> BenchStats;
}

impl StatisticalAnalysis for Vec<Duration> {
    fn analyze(&self) -> BenchStats {
        BenchStats::from_samples(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bench_stats_basic() {
        let samples = vec![
            Duration::from_millis(10),
            Duration::from_millis(12),
            Duration::from_millis(11),
            Duration::from_millis(13),
            Duration::from_millis(10),
        ];
        
        let stats = BenchStats::from_samples(samples);
        
        assert!(stats.mean > Duration::ZERO);
        assert!(stats.median > Duration::ZERO);
        assert!(stats.std_dev > Duration::ZERO);
        assert_eq!(stats.min, Duration::from_millis(10));
        assert_eq!(stats.max, Duration::from_millis(13));
    }
    
    #[test]
    fn test_outlier_detection() {
        let samples = vec![
            Duration::from_millis(10),
            Duration::from_millis(11),
            Duration::from_millis(10),
            Duration::from_millis(12),
            Duration::from_millis(11),
            Duration::from_millis(100), // Outlier
        ];
        
        let stats = BenchStats::from_samples(samples);
        
        assert!(!stats.outliers.is_empty());
        assert!(stats.outlier_percentage() > 0.0);
    }
    
    #[test]
    fn test_coefficient_of_variation() {
        let samples = vec![
            Duration::from_millis(10),
            Duration::from_millis(10),
            Duration::from_millis(10),
        ];
        
        let stats = BenchStats::from_samples(samples);
        
        // Very stable results should have low CV
        assert!(stats.coefficient_of_variation() < 1.0);
        assert!(stats.is_stable());
    }
}
