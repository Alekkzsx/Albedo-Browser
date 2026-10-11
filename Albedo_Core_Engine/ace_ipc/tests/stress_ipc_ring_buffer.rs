use ace_ipc::{bounded_channel, DomMutationPayload, DomMutationType, IpcMessage, IpcRouter, Topic};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

#[test]
fn test_stress_ipc_bounded_ring_buffer() {
    const TOTAL_MESSAGES: usize = 1_000_000;
    const CHANNEL_CAPACITY: usize = 10_000;
    const PRODUCER_THREADS: usize = 4;
    const MSGS_PER_PRODUCER: usize = TOTAL_MESSAGES / PRODUCER_THREADS;

    let (tx, rx) = bounded_channel::<IpcMessage>(CHANNEL_CAPACITY);
    let start = Instant::now();

    // Spawn consumer
    let consumer = thread::spawn(move || {
        let mut count = 0;
        while count < TOTAL_MESSAGES {
            if let Ok(msg) = rx.recv() {
                match msg {
                    IpcMessage::DomMutation(_) => {
                        count += 1;
                    }
                    _ => panic!("Mensagem inesperada no teste"),
                }
            } else {
                break;
            }
        }
        count
    });

    // Spawn producers
    let mut producers = Vec::new();
    for producer_id in 0..PRODUCER_THREADS {
        let tx_clone = tx.clone();
        let handle = thread::spawn(move || {
            for i in 0..MSGS_PER_PRODUCER {
                let msg = IpcMessage::DomMutation(DomMutationPayload {
                    doc_id: 1,
                    target_node_id: (producer_id * MSGS_PER_PRODUCER + i) as u64,
                    mutation_type: DomMutationType::ClassChanged {
                        old_class: None,
                        new_class: Some("theme-dark".to_string()),
                    },
                });
                tx_clone.send(msg).expect("Falha ao enviar mensagem IPC");
            }
        });
        producers.push(handle);
    }
    drop(tx); // drop original sender

    for handle in producers {
        handle.join().expect("Producer panic");
    }

    let consumed = consumer.join().expect("Consumer panic");
    let elapsed = start.elapsed();

    assert_eq!(
        consumed, TOTAL_MESSAGES,
        "Todas as 1.000.000 mensagens devem ser consumidas sem perdas"
    );

    let msgs_per_sec = (TOTAL_MESSAGES as f64) / elapsed.as_secs_f64();
    println!(
        "[STRESS IPC] 1.000.000 mensagens transmitidas em {:.3}s ({:.1} msg/s)",
        elapsed.as_secs_f64(),
        msgs_per_sec
    );
    assert!(
        msgs_per_sec > 100_000.0,
        "Vazão do IPC deve superar com folga 100k msg/s"
    );
}

#[test]
fn test_stress_ipc_router_multitopic_broadcast() {
    const MESSAGES_PER_TOPIC: usize = 50_000;
    let router = IpcRouter::new();

    let rx_dom1 = router.subscribe(Topic::Dom);
    let rx_dom2 = router.subscribe(Topic::Dom);
    let rx_nav = router.subscribe(Topic::Navigation);

    let counter_dom1 = Arc::new(AtomicUsize::new(0));
    let counter_dom2 = Arc::new(AtomicUsize::new(0));
    let counter_nav = Arc::new(AtomicUsize::new(0));

    let c_dom1 = counter_dom1.clone();
    let h_dom1 = thread::spawn(move || {
        while let Ok(msg) = rx_dom1.recv() {
            if matches!(msg, IpcMessage::DomMutation(_)) {
                c_dom1.fetch_add(1, Ordering::Relaxed);
            }
            if matches!(msg, IpcMessage::Shutdown) {
                break;
            }
        }
    });

    let c_dom2 = counter_dom2.clone();
    let h_dom2 = thread::spawn(move || {
        while let Ok(msg) = rx_dom2.recv() {
            if matches!(msg, IpcMessage::DomMutation(_)) {
                c_dom2.fetch_add(1, Ordering::Relaxed);
            }
            if matches!(msg, IpcMessage::Shutdown) {
                break;
            }
        }
    });

    let c_nav = counter_nav.clone();
    let h_nav = thread::spawn(move || {
        while let Ok(msg) = rx_nav.recv() {
            if matches!(msg, IpcMessage::Navigate { .. }) {
                c_nav.fetch_add(1, Ordering::Relaxed);
            }
            if matches!(msg, IpcMessage::Shutdown) {
                break;
            }
        }
    });

    // Publicar mensagens
    for i in 0..MESSAGES_PER_TOPIC {
        router
            .publish(
                Topic::Dom,
                IpcMessage::DomMutation(DomMutationPayload {
                    doc_id: 1,
                    target_node_id: i as u64,
                    mutation_type: DomMutationType::AttributeChanged {
                        name: "data-index".to_string(),
                        value: Some(i.to_string()),
                    },
                }),
            )
            .unwrap();

        router
            .publish(
                Topic::Navigation,
                IpcMessage::Navigate {
                    url: format!("https://albedo.local/page/{}", i),
                    referrer: None,
                },
            )
            .unwrap();
    }

    // Sinalizar shutdown
    router.publish(Topic::Dom, IpcMessage::Shutdown).unwrap();
    router
        .publish(Topic::Navigation, IpcMessage::Shutdown)
        .unwrap();

    h_dom1.join().unwrap();
    h_dom2.join().unwrap();
    h_nav.join().unwrap();

    assert_eq!(counter_dom1.load(Ordering::SeqCst), MESSAGES_PER_TOPIC);
    assert_eq!(counter_dom2.load(Ordering::SeqCst), MESSAGES_PER_TOPIC);
    assert_eq!(counter_nav.load(Ordering::SeqCst), MESSAGES_PER_TOPIC);
}
