use ace_core::arena::Arena;
use ace_core::event_loop::{EventLoop, TaskSource};
use ace_core::math::geometry::{point2, rect, Transform3D, Vec4};
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::quad::Quad2D;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_layout_unit_checked_and_saturating_arithmetic() {
    let zero = LayoutUnit::from_raw(0);
    let ten = LayoutUnit::from_raw(10);
    let max = LayoutUnit::from_raw(i32::MAX);
    let min = LayoutUnit::from_raw(i32::MIN);

    // 0 / 0 deve retornar NaN
    assert!(zero.div_layout_unit(zero).is_nan());
    // 10 / 0 deve retornar Infinity
    assert_eq!(ten.div_layout_unit(zero), f32::INFINITY);

    // Divisão por zero em inteiros não deve entrar em pânico, mas sim saturar
    assert_eq!(ten / 0, LayoutUnit::MAX);
    assert_eq!(min / 0, LayoutUnit::MIN);
    assert_eq!(min / -1, LayoutUnit::MAX); // Overflow protection

    // Checked methods
    assert_eq!(ten.checked_add(ten), Some(LayoutUnit::from_raw(20)));
    assert_eq!(max.checked_add(ten), None);
    assert_eq!(min.checked_sub(ten), None);
    assert_eq!(max.checked_mul(2), None);
    assert_eq!(ten.checked_div(0), None);
    assert_eq!(min.checked_div(-1), None);
}

#[test]
fn test_arena_gc_sweep_and_retain() {
    let mut arena: Arena<String> = Arena::new();
    let id1 = arena.alloc("Node 1".to_string());
    let id2 = arena.alloc("Node 2".to_string());
    let id3 = arena.alloc("Node 3".to_string());

    assert_eq!(arena.len(), 3);

    // Retém apenas os nós ímpares
    let removed = arena.retain(|val| val.contains('1') || val.contains('3'));
    assert_eq!(removed, 1);
    assert_eq!(arena.len(), 2);
    assert!(arena.get(id1).is_some());
    assert!(arena.get(id2).is_none());
    assert!(arena.get(id3).is_some());

    // Executa uma varredura de GC (Sweep) simulando um Root Set onde apenas id1 está vivo
    let swept = arena.gc_sweep(|id, _| id == id1);
    assert_eq!(swept, 1);
    assert_eq!(arena.len(), 1);
    assert!(arena.get(id1).is_some());
    assert!(arena.get(id3).is_none());

    // Shrink to fit
    arena.shrink_to_fit();
}

#[test]
fn test_quad_3d_transformed_rect_and_perspective() {
    let r = rect(0.0, 0.0, 100.0, 100.0);
    let identity: Transform3D<f32, (), ()> = Transform3D::identity();

    let quad = Quad2D::from_transformed_rect_3d(&r, &identity).expect("3D identity quad");
    assert_eq!(quad.points[0], point2(0.0, 0.0));
    assert_eq!(quad.points[1], point2(100.0, 0.0));

    // Teste de coordenadas homogêneas Vec4
    let v_valid = Vec4::new(10.0, 20.0, 30.0, 2.0);
    let pt2d = v_valid.to_cartesian_2d::<()>().unwrap();
    assert_eq!(pt2d, point2(5.0, 10.0));

    let v_behind = Vec4::new(10.0, 20.0, 30.0, -1.0); // w <= 0 (atrás da câmera)
    assert!(v_behind.to_cartesian_2d::<()>().is_none());
    assert!(v_behind.to_cartesian_3d::<()>().is_none());
}

#[test]
fn test_event_loop_waker_and_non_busy_waiting() {
    let event_loop = EventLoop::new();
    let handle = event_loop.handle();

    let executed = Arc::new(AtomicBool::new(false));
    let exec_clone = Arc::clone(&executed);

    // Thread auxiliar que envia uma tarefa após 30ms
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        handle.queue_task(TaskSource::UserInteraction, move || {
            exec_clone.store(true, Ordering::SeqCst);
        });
    });

    let loop_exec = Arc::clone(&executed);

    // Roda um loop com timeout
    let timer = std::time::Instant::now();
    while !loop_exec.load(Ordering::SeqCst) && timer.elapsed() < Duration::from_millis(500) {
        event_loop.step();
        std::thread::sleep(Duration::from_millis(5));
    }

    assert!(executed.load(Ordering::SeqCst));
}
