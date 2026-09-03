//! # Testes de Escalonamento e Prioridade de Recursos

use ace_net::priority::{PrioritizedItem, PriorityLevel};
use ace_net::request::RequestDestination;
use std::collections::BinaryHeap;

#[test]
fn test_priority_hierarchy() {
    assert!(PriorityLevel::VeryHigh > PriorityLevel::High);
    assert!(PriorityLevel::High > PriorityLevel::Medium);
    assert!(PriorityLevel::Medium > PriorityLevel::Low);
    assert!(PriorityLevel::Low > PriorityLevel::Lowest);
}

#[test]
fn test_render_blocking_classification() {
    assert!(PriorityLevel::VeryHigh.is_render_blocking());
    assert!(PriorityLevel::High.is_render_blocking());
    assert!(!PriorityLevel::Medium.is_render_blocking());
    assert!(!PriorityLevel::Low.is_render_blocking());
    assert!(!PriorityLevel::Lowest.is_render_blocking());
}

#[test]
fn test_request_destination_priorities() {
    assert_eq!(RequestDestination::Document.default_priority(), PriorityLevel::VeryHigh);
    assert_eq!(RequestDestination::Style.default_priority(), PriorityLevel::High);
    assert_eq!(RequestDestination::Script.default_priority(), PriorityLevel::High);
    assert_eq!(RequestDestination::Font.default_priority(), PriorityLevel::High);
    assert_eq!(RequestDestination::Image.default_priority(), PriorityLevel::Medium);
    assert_eq!(RequestDestination::Media.default_priority(), PriorityLevel::Low);
    assert_eq!(RequestDestination::Fetch.default_priority(), PriorityLevel::Low);
}

#[test]
fn test_prioritized_scheduler_queue() {
    let mut queue = BinaryHeap::new();

    queue.push(PrioritizedItem {
        priority: PriorityLevel::Low,
        sequence_id: 1,
        item: "background-image.jpg",
    });

    queue.push(PrioritizedItem {
        priority: PriorityLevel::VeryHigh,
        sequence_id: 2,
        item: "document.html",
    });

    queue.push(PrioritizedItem {
        priority: PriorityLevel::High,
        sequence_id: 3,
        item: "main.css",
    });

    queue.push(PrioritizedItem {
        priority: PriorityLevel::High,
        sequence_id: 4,
        item: "bundle.js",
    });

    queue.push(PrioritizedItem {
        priority: PriorityLevel::Lowest,
        sequence_id: 5,
        item: "analytics.js",
    });

    assert_eq!(queue.pop().unwrap().item, "document.html");
    assert_eq!(queue.pop().unwrap().item, "main.css");
    assert_eq!(queue.pop().unwrap().item, "bundle.js");
    assert_eq!(queue.pop().unwrap().item, "background-image.jpg");
    assert_eq!(queue.pop().unwrap().item, "analytics.js");
}
