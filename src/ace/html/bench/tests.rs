//! Tests for benchmark framework

use super::*;
use std::time::Duration;

#[test]
fn test_statistical_analysis() {
    let samples = vec![
        Duration::from_millis(10),
        Duration::from_millis(12),
        Duration::from_millis(11),
        Duration::from_millis(13),
        Duration::from_millis(10),
        Duration::from_millis(11),
        Duration::from_millis(12),
    ];
    
    let stats = BenchStats::from_samples(samples);
    
    assert!(stats.mean > Duration::ZERO);
    assert!(stats.median > Duration::ZERO);
    assert!(stats.std_dev > Duration::ZERO);
    assert!(stats.p95 > Duration::ZERO);
    assert!(stats.p99 > Duration::ZERO);
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
        Duration::from_millis(10),
        Duration::from_millis(100), // Clear outlier
    ];
    
    let stats = BenchStats::from_samples(samples);
    
    assert!(!stats.outliers.is_empty());
    assert!(stats.outlier_percentage() > 0.0);
}

#[test]
fn test_benchmark_runner() {
    let config = BenchConfig::new("test_runner")
        .with_warmup(3)
        .with_measurements(10);
    
    let runner = BenchRunner::new(config);
    
    let result = runner.run(|| {
        // Simulate work
        let mut sum = 0;
        for i in 0..1000 {
            sum += i;
        }
        std::hint::black_box(sum);
    });
    
    assert_eq!(result.name, "test_runner");
    assert_eq!(result.stats.samples.len(), 10);
    assert!(result.stats.mean > Duration::ZERO);
}

#[test]
fn test_baseline_comparison_faster() {
    // Create baseline (slower)
    let baseline_samples = vec![Duration::from_millis(10); 10];
    let baseline = BenchStats::from_samples(baseline_samples);
    
    let config = BenchConfig::new("comparison_test")
        .with_warmup(1)
        .with_measurements(10)
        .with_baseline(baseline);
    
    let runner = BenchRunner::new(config);
    
    // Run faster version
    let result = runner.run(|| {
        std::thread::sleep(Duration::from_millis(5));
    });
    
    assert!(result.comparison.is_some());
    let comp = result.comparison.unwrap();
    assert!(comp.speedup > 1.0);
    assert!(!comp.is_regression);
}

#[test]
fn test_baseline_comparison_regression() {
    // Create baseline (faster)
    let baseline_samples = vec![Duration::from_millis(5); 10];
    let baseline = BenchStats::from_samples(baseline_samples);
    
    let config = BenchConfig::new("regression_test")
        .with_warmup(1)
        .with_measurements(10)
        .with_baseline(baseline);
    
    let runner = BenchRunner::new(config);
    
    // Run slower version (regression)
    let result = runner.run(|| {
        std::thread::sleep(Duration::from_millis(10));
    });
    
    assert!(result.comparison.is_some());
    let comp = result.comparison.unwrap();
    assert!(comp.speedup < 1.0);
    assert!(comp.is_regression);
}

#[test]
fn test_report_generation_json() {
    let samples = vec![Duration::from_millis(10); 10];
    let stats = BenchStats::from_samples(samples);
    
    let result = BenchResult {
        name: "json_test".to_string(),
        stats,
        comparison: None,
    };
    
    let mut generator = ReportGenerator::new();
    generator.add_result(result);
    
    let path = "target/bench_test.json";
    generator.generate(path, ReportFormat::Json).unwrap();
    
    assert!(std::path::Path::new(path).exists());
}

#[test]
fn test_report_generation_html() {
    let samples = vec![Duration::from_millis(10); 10];
    let stats = BenchStats::from_samples(samples);
    
    let result = BenchResult {
        name: "html_test".to_string(),
        stats,
        comparison: None,
    };
    
    let mut generator = ReportGenerator::new();
    generator.add_result(result);
    
    let path = "target/bench_test.html";
    generator.generate(path, ReportFormat::Html).unwrap();
    
    assert!(std::path::Path::new(path).exists());
}

#[test]
fn test_coefficient_of_variation() {
    // Very stable results
    let stable_samples = vec![Duration::from_millis(10); 10];
    let stable_stats = BenchStats::from_samples(stable_samples);
    
    assert!(stable_stats.is_stable());
    assert!(stable_stats.coefficient_of_variation() < 1.0);
    
    // Unstable results
    let unstable_samples = vec![
        Duration::from_millis(5),
        Duration::from_millis(15),
        Duration::from_millis(8),
        Duration::from_millis(20),
        Duration::from_millis(10),
    ];
    let unstable_stats = BenchStats::from_samples(unstable_samples);
    
    assert!(unstable_stats.coefficient_of_variation() > 5.0);
}
