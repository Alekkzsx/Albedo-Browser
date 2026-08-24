use ace_core::time::{Clock, MockClock};
use std::time::Duration;

#[test]
fn test_mock_clock_determinism() {
    let clock = MockClock::new(100);
    assert_eq!(clock.now_ms(), 100);

    clock.advance(Duration::from_millis(50));
    assert_eq!(clock.now_ms(), 150);
}

#[test]
fn test_time_quantization_and_spectre_mitigation() {
    use ace_core::time::{quantize_highres, quantize_micros, QuantizedClock};

    // 1234 microssegundos com granularidade de 20 microssegundos = 1220 microssegundos
    assert_eq!(quantize_micros(1234, 20), 1220);
    assert_eq!(quantize_micros(1239, 20), 1220);
    assert_eq!(quantize_micros(1240, 20), 1240);

    // Quantização de alta resolução (DOMHighResTimeStamp)
    let quantized_hr = quantize_highres(1.23456, 20); // 1234.56µs -> 1220µs = 1.220ms
    assert!((quantized_hr - 1.220).abs() < 1e-5);

    // QuantizedClock
    let mock = MockClock::from_micros(1005);
    let qclock = QuantizedClock::new(mock, 20);
    assert_eq!(qclock.now_us(), 1000);
}

