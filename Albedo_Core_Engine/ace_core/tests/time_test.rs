use ace_core::time::{Clock, MockClock};
use std::time::Duration;

#[test]
fn test_mock_clock_determinism() {
    let clock = MockClock::new(100);
    assert_eq!(clock.now_ms(), 100);

    clock.advance(Duration::from_millis(50));
    assert_eq!(clock.now_ms(), 150);
}
