use ace_core::telemetry::frame::{FrameBudgetTracker, FrameStage};
use std::thread;
use std::time::Duration;

#[test]
fn test_frame_budget_tracker_stages() {
    let tracker = FrameBudgetTracker::for_60hz();

    let mut recorder = tracker.begin_frame();
    recorder.record_stage(FrameStage::DomEvents, 1.2);
    recorder.record_stage(FrameStage::Layout, 2.5);
    recorder.record_stage(FrameStage::Paint, 3.1);

    let metrics = tracker.finish_frame(recorder);

    assert_eq!(metrics.frame_number, 1);
    assert_eq!(metrics.stages_ms[FrameStage::Layout as usize], 2.5);
    assert_eq!(tracker.total_frames(), 1);
}

#[test]
fn test_frame_budget_jank_detection() {
    // Orçamento minúsculo de 1ms para forçar detecção de jank no teste
    let tracker = FrameBudgetTracker::new(1.0);

    let recorder = tracker.begin_frame();
    thread::sleep(Duration::from_millis(5));
    let metrics = tracker.finish_frame(recorder);

    assert!(metrics.is_jank);
    assert_eq!(tracker.total_janks(), 1);
}
