use ace_net::engine::critical_scheduler::{CriticalResourceScheduler, ResourcePriority};
use std::sync::Arc;

#[test]
fn test_stress_critical_resource_scheduler_priority_inversion() {
    let scheduler = Arc::new(CriticalResourceScheduler::new(10));

    // Agendar 100 tarefas com prioridades intercaladas
    for i in 0..20 {
        scheduler.schedule(
            format!("https://example.com/telemetry/{}", i),
            ResourcePriority::Lowest,
            "beacon",
        );
        scheduler.schedule(
            format!("https://example.com/image/{}", i),
            ResourcePriority::Low,
            "lazy-image",
        );
        scheduler.schedule(
            format!("https://example.com/script/{}", i),
            ResourcePriority::Medium,
            "async-script",
        );
        scheduler.schedule(
            format!("https://example.com/font/{}", i),
            ResourcePriority::High,
            "font",
        );
        scheduler.schedule(
            format!("https://example.com/critical.css?v={}", i),
            ResourcePriority::Urgent,
            "render-blocking-css",
        );
    }

    assert_eq!(scheduler.pending_count(), 100);

    // O despacho deve entregar primeiro TODAS as 20 tarefas Urgent
    let mut dispatched_priorities = Vec::new();
    while let Some(task) = scheduler.poll_next() {
        dispatched_priorities.push(task.priority);
        scheduler.mark_completed(); // Libera slot imediatamente para drenar fila
    }

    assert_eq!(dispatched_priorities.len(), 100);

    // Primeiras 20 devem ser Urgent
    for p in &dispatched_priorities[0..20] {
        assert_eq!(
            *p,
            ResourcePriority::Urgent,
            "Tarefas 0..20 devem ser Urgent"
        );
    }

    // Próximas 20 devem ser High
    for p in &dispatched_priorities[20..40] {
        assert_eq!(*p, ResourcePriority::High, "Tarefas 20..40 devem ser High");
    }

    // Próximas 20 devem ser Medium
    for p in &dispatched_priorities[40..60] {
        assert_eq!(
            *p,
            ResourcePriority::Medium,
            "Tarefas 40..60 devem ser Medium"
        );
    }

    // Próximas 20 devem ser Low
    for p in &dispatched_priorities[60..80] {
        assert_eq!(*p, ResourcePriority::Low, "Tarefas 60..80 devem ser Low");
    }

    // Últimas 20 devem ser Lowest
    for p in &dispatched_priorities[80..100] {
        assert_eq!(
            *p,
            ResourcePriority::Lowest,
            "Tarefas 80..100 devem ser Lowest"
        );
    }

    println!("[STRESS SCHEDULER] 100 tarefas reordenadas e despachadas com sucesso estrito de prioridade!");
}

#[test]
fn test_stress_scheduler_concurrency_throttle() {
    const MAX_CONCURRENT: usize = 4;
    let scheduler = Arc::new(CriticalResourceScheduler::new(MAX_CONCURRENT));

    for i in 0..20 {
        scheduler.schedule(
            format!("https://example.com/res/{}", i),
            ResourcePriority::Medium,
            "parser",
        );
    }

    // Puxa até o limite máximo de concorrência
    let mut active = Vec::new();
    for _ in 0..MAX_CONCURRENT {
        active.push(
            scheduler
                .poll_next()
                .expect("Deve despachar até MAX_CONCURRENT"),
        );
    }

    assert_eq!(scheduler.in_flight_count(), MAX_CONCURRENT as u64);

    // Próxima tentativa deve retornar None devido à saturação
    assert!(
        scheduler.poll_next().is_none(),
        "Não pode exceder limite de concorrência"
    );

    // Libera 2 tarefas
    scheduler.mark_completed();
    scheduler.mark_completed();
    assert_eq!(scheduler.in_flight_count(), 2);

    // Agora pode despachar mais 2
    assert!(scheduler.poll_next().is_some());
    assert!(scheduler.poll_next().is_some());
    assert_eq!(scheduler.in_flight_count(), 4);
    assert!(scheduler.poll_next().is_none());
}
