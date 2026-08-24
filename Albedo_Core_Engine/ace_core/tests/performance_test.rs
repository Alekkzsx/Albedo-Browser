use ace_core::performance::PerformanceTimeline;
use ace_core::time::MockClock;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_performance_timeline_marks_and_measures() {
    let mock_clock = Arc::new(MockClock::new(1000));
    let timeline =
        PerformanceTimeline::with_clock(Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>);

    // Registra mark 1 em t = 1000ms
    let m1 = timeline.mark("parse_start");
    assert_eq!(m1.start_time_ms, 1000.0);

    // Avança tempo virtual em 250ms (t = 1250ms)
    mock_clock.advance(Duration::from_millis(250));
    let m2 = timeline.mark("parse_end");
    assert_eq!(m2.start_time_ms, 1250.0);

    // Mede intervalo
    let measure = timeline
        .measure("HTML Parse", "parse_start", "parse_end")
        .unwrap();
    assert_eq!(measure.start_time_ms, 1000.0);
    assert_eq!(measure.duration_ms, 250.0);
}

#[test]
fn test_scoped_raii_measure() {
    let mock_clock = Arc::new(MockClock::new(2000));
    let timeline =
        PerformanceTimeline::with_clock(Arc::clone(&mock_clock) as Arc<dyn ace_core::time::Clock>);

    {
        let _guard = timeline.scoped_measure("Style Recalc");
        mock_clock.advance(Duration::from_millis(15));
    } // Ao sair de escopo, registra a medida

    let entries = timeline.get_entries_by_name("Style Recalc");
    assert_eq!(entries.len(), 1);
}
