use ace_core::telemetry::Histogram;

#[test]
fn test_histogram_linear_and_exponential_sampling() {
    let hist: Histogram<10> = Histogram::linear("LayoutLatencyMs", 0, 100);

    for v in 1..=50 {
        hist.sample(v);
    }

    let snap = hist.snapshot();
    assert_eq!(snap.count, 50);
    assert_eq!(snap.sum, 1275);
    assert_eq!(snap.mean, 25.5);
    assert!(snap.p50 > 0);
    assert!(snap.p90 >= snap.p50);

    let json = hist.to_json();
    assert!(json.contains("\"name\":\"LayoutLatencyMs\""));
    assert!(json.contains("\"count\":50"));
}

#[test]
fn test_histogram_exponential_distribution() {
    let hist: Histogram<8> = Histogram::exponential("NetworkFetchDurationMs", 1, 10000);
    hist.sample(10);
    hist.sample(100);
    hist.sample(1000);

    let snap = hist.snapshot();
    assert_eq!(snap.count, 3);
    assert_eq!(snap.sum, 1110);
}
