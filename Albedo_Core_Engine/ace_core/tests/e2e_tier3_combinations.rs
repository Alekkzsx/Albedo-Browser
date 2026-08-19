//! # Tier 3 E2E Test Suite: Cross-Feature Multi-Module Pairwise Interactions
//!
//! Testes de ponta a ponta que cobrem integrações e combinações entre múltiplos
//! subsistemas do `ace_core`:
//! 1. `Arena<T>` + `InlineVec`
//! 2. `TripleBuffer` + `Color`
//! 3. `Origin` + `compute_referrer`
//! 4. `Origin` + `matches_domain_pattern`
//! 5. `UnguessableToken` + `Origin`
//! 6. `percent_decode` + `Origin`
//! 7. `sniff_mime_type` + `percent_decode`
//! 8. `EventLoop` + `TaskSource` + `Microtask`
//! 9. `EventLoop` + Timers + Starvation Prevention
//! 10. `EventLoop` + Dynamic Timer Nesting + Microtasks
//! 11. `LayoutUnit` + `Color`
//! 12. `LayoutUnit` + `style_hint_to_node_flags`
//! 13. `BreadcrumbBuffer` + `EventLoop`
//! 14. `BreadcrumbBuffer` + `Arena`
//! 15. `InlineVec` + `BreadcrumbBuffer`
//! 16. `Color` (Bradford Adaptation) + `Color` (Polar Interpolation)
//! 17. `UnguessableToken` + `compute_referrer`
//! 18. `TripleBuffer` + `LayoutUnit`
//! 19. `Arena` + `InlineVec` + `style_hint_to_node_flags`

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ace_core::arena::{Arena, ArenaId};
use ace_core::collections::{triple_buffer, InlineVec};
use ace_core::diagnostics::{BreadcrumbBuffer, BreadcrumbEntry};
use ace_core::event_loop::{EventLoop, TaskSource};
use ace_core::flags::{is_node_dirty, style_hint_to_node_flags, NodeFlags, StyleChangeHint};
use ace_core::math::color::{display_p3_to_srgb, mix_colors, srgb_to_display_p3, ColorSpace};
use ace_core::math::{Color, LayoutUnit, Oklch};
use ace_core::net::{parse_data_uri, percent_decode, sniff_mime_type};
use ace_core::security::{
    compute_referrer, is_potentially_trustworthy_origin, matches_domain_pattern, Origin,
    ReferrerPolicy, UnguessableToken,
};
use ace_core::time::{Clock, MockClock};

// =========================================================================
// 1. Arena<T> + InlineVec
// =========================================================================
#[test]
fn tier3_combo_01_arena_and_inline_vec() {
    #[derive(Debug, PartialEq)]
    struct DomNode {
        tag: &'static str,
        parent: Option<ArenaId<DomNode>>,
        children: InlineVec<ArenaId<DomNode>, 4>,
    }

    let mut arena: Arena<DomNode> = Arena::new();

    // Cria nó raiz
    let root_id = arena.alloc(DomNode {
        tag: "html",
        parent: None,
        children: InlineVec::new(),
    });

    // Adiciona 3 filhos (permanecem inline no InlineVec do root)
    let body_id = arena.alloc(DomNode {
        tag: "body",
        parent: Some(root_id),
        children: InlineVec::new(),
    });
    let head_id = arena.alloc(DomNode {
        tag: "head",
        parent: Some(root_id),
        children: InlineVec::new(),
    });
    let meta_id = arena.alloc(DomNode {
        tag: "meta",
        parent: Some(root_id),
        children: InlineVec::new(),
    });

    let root = arena.get_mut(root_id).expect("root deve existir");
    root.children.push(head_id);
    root.children.push(meta_id);
    root.children.push(body_id);
    assert!(root.children.is_inline(), "3 filhos cabem no buffer inline de N=4");
    assert_eq!(root.children.len(), 3);

    // Adiciona mais filhos para forçar spill para heap no InlineVec
    let script_id = arena.alloc(DomNode {
        tag: "script",
        parent: Some(root_id),
        children: InlineVec::new(),
    });
    let style_id = arena.alloc(DomNode {
        tag: "style",
        parent: Some(root_id),
        children: InlineVec::new(),
    });

    let root = arena.get_mut(root_id).expect("root deve existir");
    root.children.push(script_id);
    root.children.push(style_id);
    assert!(!root.children.is_inline(), "5 filhos forçam transição para heap");
    assert_eq!(root.children.len(), 5);

    // Valida integridade do grafo pai-filho
    assert_eq!(arena.len(), 6);
    let child_tags: Vec<&str> = arena
        .get(root_id)
        .unwrap()
        .children
        .iter()
        .map(|&cid| arena.get(cid).unwrap().tag)
        .collect();
    assert_eq!(child_tags, vec!["head", "meta", "body", "script", "style"]);

    // Remove um nó e limpa a arena
    let removed = arena.remove(meta_id);
    assert!(removed.is_some());
    assert_eq!(arena.len(), 5);
    assert!(!arena.contains(meta_id));

    // Clear geral da arena e verificação de integridade
    arena.clear();
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());
    assert!(arena.get(root_id).is_none());

    // Aloca novo nó pós-clear: valida reutilização de slot
    let new_root = arena.alloc(DomNode {
        tag: "svg",
        parent: None,
        children: InlineVec::new(),
    });
    assert_eq!(arena.len(), 1);
    assert_eq!(arena.get(new_root).unwrap().tag, "svg");
    assert_ne!(new_root, root_id, "Versão geracional deve mudar após clear");
}

// =========================================================================
// 2. TripleBuffer + Color
// =========================================================================
#[test]
fn tier3_combo_02_triple_buffer_and_color() {
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct LayerPalette {
        background: Color,
        foreground: Color,
        border: Color,
    }

    let initial = LayerPalette {
        background: Color::BLACK,
        foreground: Color::WHITE,
        border: Color::TRANSPARENT,
    };

    let (mut producer, mut consumer) = triple_buffer(initial);

    // Frame 0: Nenhum update publicado ainda
    assert!(consumer.consume().is_none());
    assert_eq!(consumer.read().background, Color::BLACK);

    // Frame 1: Producer renderiza novas cores CSS
    let bg_frame1 = Color::parse_css("hsl(210, 100%, 50%)").unwrap();
    let fg_frame1 = Color::RED.blend_source_over(Color::BLUE);
    producer.write(LayerPalette {
        background: bg_frame1,
        foreground: fg_frame1,
        border: Color::from_rgba(255, 255, 0, 128),
    });
    producer.publish();

    // Consumer consome frame 1 de forma zero-copy
    let frame1_ref = consumer.consume().expect("Frame 1 deve estar disponível");
    assert_eq!(frame1_ref.background, bg_frame1);
    assert_eq!(frame1_ref.foreground, fg_frame1);
    assert_eq!(frame1_ref.border.a, 128);

    // Frame 2 & 3: Producer produz múltiplos frames a 120 FPS
    for i in 1..=5 {
        let t = i as f32 / 5.0;
        let interpolated_bg = Color::RED.lerp(Color::LIME, t);
        producer.write_with(|palette| {
            palette.background = interpolated_bg;
            palette.foreground = Color::WHITE;
        });
        producer.publish();
    }

    // Consumer consome o frame mais recente publicado
    let latest = consumer.consume().expect("Último frame deve estar disponível");
    assert_eq!(latest.background, Color::LIME);
    assert_eq!(latest.foreground, Color::WHITE);

    // Próxima leitura não traz novos frames
    assert!(consumer.consume().is_none());
    assert_eq!(consumer.read().background, Color::LIME);
}

// =========================================================================
// 3. Origin + compute_referrer
// =========================================================================
#[test]
fn tier3_combo_03_origin_and_compute_referrer() {
    let secure_origin = Origin::parse("https://albedo.org:443").unwrap();
    let insecure_origin = Origin::parse("http://insecure.net:80").unwrap();
    let file_origin = Origin::parse("file:///C:/browser/index.html").unwrap();

    // 1. Same-Origin HTTPS request com política StrictOriginWhenCrossOrigin
    let ref1 = compute_referrer(
        &secure_origin,
        "https://albedo.org/articles/rust#section-1",
        "https://albedo.org/api/comments",
        ReferrerPolicy::StrictOriginWhenCrossOrigin,
    );
    assert_eq!(
        ref1,
        Some("https://albedo.org/articles/rust".to_string()),
        "Same-origin deve manter o caminho sem fragmento"
    );

    // 2. Cross-Origin HTTPS request com política StrictOriginWhenCrossOrigin
    let ref2 = compute_referrer(
        &secure_origin,
        "https://albedo.org/articles/rust?id=42#fragment",
        "https://cdn.albedo.org/assets/style.css",
        ReferrerPolicy::StrictOriginWhenCrossOrigin,
    );
    assert_eq!(
        ref2,
        Some("https://albedo.org".to_string()),
        "Cross-origin seguro deve truncar para a origem serializada"
    );

    // 3. Downgrade HTTPS -> HTTP com política StrictOrigin
    let ref3 = compute_referrer(
        &secure_origin,
        "https://albedo.org/login",
        "http://insecure.net/tracker",
        ReferrerPolicy::StrictOriginWhenCrossOrigin,
    );
    assert_eq!(ref3, None, "Downgrade de HTTPS para HTTP deve suprimir o Referer");

    // 4. SameOrigin policy em requisição cross-origin
    let ref4 = compute_referrer(
        &secure_origin,
        "https://albedo.org/home",
        "https://other.com/page",
        ReferrerPolicy::SameOrigin,
    );
    assert_eq!(ref4, None, "SameOrigin policy deve retornar None para cross-origin");

    // 5. File URL não deve emitir cabeçalho Referer
    let ref5 = compute_referrer(
        &file_origin,
        "file:///C:/browser/index.html",
        "https://albedo.org/home",
        ReferrerPolicy::UnsafeUrl,
    );
    assert_eq!(ref5, None, "Esquemas não-HTTP/HTTPS não devem gerar Referer");

    // 6. NoReferrer policy
    let ref6 = compute_referrer(
        &insecure_origin,
        "http://insecure.net/page",
        "http://insecure.net/other",
        ReferrerPolicy::NoReferrer,
    );
    assert_eq!(ref6, None);
}

// =========================================================================
// 4. Origin + matches_domain_pattern
// =========================================================================
#[test]
fn tier3_combo_04_origin_and_matches_domain_pattern() {
    let origins = [
        Origin::parse("https://api.albedo.dev:443").unwrap(),
        Origin::parse("https://cdn.static.albedo.dev:8443").unwrap(),
        Origin::parse("https://albedo.dev").unwrap(),
        Origin::parse("https://evil-albedo.dev").unwrap(),
        Origin::parse("http://localhost:3000").unwrap(),
    ];

    let pattern_wildcard = "*.albedo.dev";
    let pattern_exact = "api.albedo.dev";
    let pattern_all = "*";

    for origin in &origins {
        if let Origin::Tuple { host, .. } = origin {
            let host_str = host.as_str();
            if host_str == "api.albedo.dev" {
                assert!(matches_domain_pattern(pattern_wildcard, &host_str));
                assert!(matches_domain_pattern(pattern_exact, &host_str));
                assert!(matches_domain_pattern(pattern_all, &host_str));
            } else if host_str == "cdn.static.albedo.dev" {
                assert!(matches_domain_pattern(pattern_wildcard, &host_str));
                assert!(!matches_domain_pattern(pattern_exact, &host_str));
            } else if host_str == "evil-albedo.dev" {
                assert!(!matches_domain_pattern(pattern_wildcard, &host_str));
                assert!(!matches_domain_pattern(pattern_exact, &host_str));
            } else if host_str == "localhost" {
                assert!(origin.is_secure());
                assert!(is_potentially_trustworthy_origin(origin));
            }
        }
    }
}

// =========================================================================
// 5. UnguessableToken + Origin
// =========================================================================
#[test]
fn tier3_combo_05_unguessable_token_and_origin() {
    struct SecurityContext {
        origin: Origin,
        token: UnguessableToken,
        is_isolated: bool,
    }

    let origin1 = Origin::parse("https://bank.example.com").unwrap();
    let origin2 = Origin::parse("https://bank.example.com").unwrap();
    let cross_origin = Origin::parse("https://ad.tracker.com").unwrap();

    let ctx1 = SecurityContext {
        origin: origin1.clone(),
        token: UnguessableToken::new(),
        is_isolated: false,
    };
    let ctx2 = SecurityContext {
        origin: origin2.clone(),
        token: UnguessableToken::new(),
        is_isolated: false,
    };
    let ctx_cross = SecurityContext {
        origin: cross_origin,
        token: UnguessableToken::new(),
        is_isolated: true,
    };

    // Validar não-nulidade e unicidade de 128 bits
    assert!(!ctx1.token.is_empty());
    assert!(!ctx2.token.is_empty());
    assert!(!ctx_cross.token.is_empty());
    assert!(!ctx1.is_isolated);
    assert!(ctx_cross.is_isolated);
    assert_ne!(ctx1.token, ctx2.token, "Tokens devem ser únicos");
    assert_ne!(ctx1.token, ctx_cross.token);

    // Validação de Same-Origin associado a tokens distintos
    assert!(ctx1.origin.same_origin(&ctx2.origin));
    assert!(!ctx1.origin.same_origin(&ctx_cross.origin));

    // Formatação de 32 hex chars e deserialização por raw
    let hex = ctx1.token.to_hex();
    assert_eq!(hex.len(), 32);
    let reconstructed = UnguessableToken::from_raw(ctx1.token.high(), ctx1.token.low());
    assert_eq!(ctx1.token, reconstructed);
}

// =========================================================================
// 6. percent_decode + Origin
// =========================================================================
#[test]
fn tier3_combo_06_percent_decode_and_origin() {
    let raw_url = "https://example.com:8443/search?q=Albedo%20Browser%20100%25%20Fast#res";
    let decoded_path = percent_decode(raw_url);
    assert!(decoded_path.contains("Albedo Browser 100% Fast"));

    let origin = Origin::parse(raw_url).expect("URL com percent-encoding deve gerar origem válida");
    assert_eq!(origin.ascii_serialization(), "https://example.com:8443");
    assert!(origin.is_secure());

    // Teste com sequências malformadas de % (ex: 100%_concluido)
    let malformed_url = "http://localhost:8080/files/100%_concluido.pdf";
    let decoded_malformed = percent_decode(malformed_url);
    assert_eq!(decoded_malformed, "http://localhost:8080/files/100%_concluido.pdf");

    let origin_malformed = Origin::parse(&decoded_malformed).unwrap();
    assert_eq!(origin_malformed.ascii_serialization(), "http://localhost:8080");
    assert!(origin_malformed.is_secure(), "Localhost deve ser contexto seguro");
}

// =========================================================================
// 7. sniff_mime_type + percent_decode
// =========================================================================
#[test]
fn tier3_combo_07_sniff_mime_type_and_percent_decode() {
    // 1. Data URI com SVG percent-encoded
    let svg_data_uri = "data:image/svg+xml;utf8,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20width%3D%22100%22%3E%3Crect%2F%3E%3C%2Fsvg%3E";
    let (mime, bytes) = parse_data_uri(svg_data_uri).expect("Data URI SVG válida");
    assert_eq!(mime.essence(), "image/svg+xml");
    let sniffed = sniff_mime_type(&bytes);
    assert_eq!(sniffed, "image/svg+xml");

    // 2. Data URI com HTML percent-encoded
    let html_data_uri = "data:text/html;charset=utf-8,%3C!DOCTYPE%20html%3E%3Chtml%3E%3Cbody%3E%3Ch1%3EOk%3C%2Fh1%3E%3C%2Fbody%3E%3C%2Fhtml%3E";
    let (mime_html, bytes_html) = parse_data_uri(html_data_uri).unwrap();
    assert_eq!(mime_html.essence(), "text/html");
    assert_eq!(mime_html.charset(), Some("utf-8"));
    let sniffed_html = sniff_mime_type(&bytes_html);
    assert_eq!(sniffed_html, "text/html");

    // 3. Percent-decoded text PDF header
    let raw_encoded_pdf = "%25PDF-1.7%0D%0A1%200%20obj";
    let decoded_pdf = percent_decode(raw_encoded_pdf);
    let sniffed_pdf = sniff_mime_type(decoded_pdf.as_bytes());
    assert_eq!(sniffed_pdf, "application/pdf");

    // 4. Data URI Base64 PNG
    let png_data_uri = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
    let (mime_png, png_bytes) = parse_data_uri(png_data_uri).unwrap();
    assert_eq!(mime_png.essence(), "image/png");
    let sniffed_png = sniff_mime_type(&png_bytes);
    assert_eq!(sniffed_png, "image/png");
}

// =========================================================================
// 8. EventLoop + TaskSource + Microtask
// =========================================================================
#[test]
fn tier3_combo_08_event_loop_task_source_and_microtask() {
    let event_loop = EventLoop::new();
    let handle = event_loop.handle();
    let execution_log = Arc::new(parking_lot::Mutex::new(Vec::new()));

    // Enfileira tarefas de diferentes TaskSources WHATWG
    let log1 = Arc::clone(&execution_log);
    let handle1 = handle.clone();
    handle.queue_task(TaskSource::UserInteraction, move || {
        log1.lock().push("Task:UserInteraction");
        // Microtask enfileirada dentro da macrotask
        let log_micro = Arc::clone(&log1);
        handle1.queue_microtask(move || {
            log_micro.lock().push("Microtask:UserInteraction_1");
        });
    });

    let log2 = Arc::clone(&execution_log);
    let handle2 = handle.clone();
    handle.queue_task(TaskSource::DomManipulation, move || {
        log2.lock().push("Task:DomManipulation");
        let log_micro = Arc::clone(&log2);
        handle2.queue_microtask(move || {
            log_micro.lock().push("Microtask:DomManipulation_1");
        });
    });

    let log3 = Arc::clone(&execution_log);
    handle.queue_task(TaskSource::Networking, move || {
        log3.lock().push("Task:Networking");
    });

    // Executa 3 steps no Event Loop
    event_loop.step();
    event_loop.step();
    event_loop.step();

    let logs = execution_log.lock().clone();
    assert_eq!(
        logs,
        vec![
            "Task:UserInteraction",
            "Microtask:UserInteraction_1",
            "Task:DomManipulation",
            "Microtask:DomManipulation_1",
            "Task:Networking",
        ],
        "Microtasks devem drenar completamente imediatamente após cada macrotask"
    );
}

// =========================================================================
// 9. EventLoop + Timers + Starvation Prevention
// =========================================================================
#[test]
fn tier3_combo_09_event_loop_timers_and_starvation_prevention() {
    let mock_clock = Arc::new(MockClock::new(1000));
    // Configura limite de starvation para 4 passos determinísticos
    let event_loop = EventLoop::with_clock_and_starvation_limit(
        Arc::clone(&mock_clock) as Arc<dyn Clock>,
        4,
    );
    let handle = event_loop.handle();

    let timer_executed = Arc::new(AtomicBool::new(false));
    let ui_tasks_count = Arc::new(AtomicUsize::new(0));

    // Agenda um timer para t = 1050ms (+50ms)
    let te = Arc::clone(&timer_executed);
    handle.schedule_timer(Duration::from_millis(50), move || {
        te.store(true, Ordering::SeqCst);
    });

    // Enfileira tarefas contínuas de UI
    for _ in 0..10 {
        let ui_counter = Arc::clone(&ui_tasks_count);
        handle.queue_user_interaction(move || {
            ui_counter.fetch_add(1, Ordering::SeqCst);
        });
    }

    // Executa 2 passos antes do timer expirar
    event_loop.step();
    event_loop.step();
    assert!(!timer_executed.load(Ordering::SeqCst), "Timer não deve ter disparado antes do prazo");

    // Avança o relógio para depois do prazo do timer (+60ms)
    mock_clock.advance_millis(60);

    // Executa passos do loop até a execução garantida pela prevenção de starvation
    for _ in 0..6 {
        if timer_executed.load(Ordering::SeqCst) {
            break;
        }
        event_loop.step();
    }

    assert!(
        timer_executed.load(Ordering::SeqCst),
        "Timer expirado deve executar sem starvation mesmo sob carga contínua de UI"
    );
}

// =========================================================================
// 10. EventLoop + Dynamic Timer Nesting + Microtasks
// =========================================================================
#[test]
fn tier3_combo_10_event_loop_dynamic_timer_nesting_and_microtasks() {
    let mock_clock = Arc::new(MockClock::new(0));
    let event_loop = EventLoop::with_clock(Arc::clone(&mock_clock) as Arc<dyn Clock>);
    let handle = event_loop.handle();

    let timer_log = Arc::new(parking_lot::Mutex::new(Vec::new()));

    // Timers com níveis de aninhamento 1 a 5 solicitando delay de 1ms
    for nesting in 1..=5 {
        let log = Arc::clone(&timer_log);
        let h = handle.clone();
        handle.schedule_timer_with_nesting(Duration::from_millis(1), nesting, move || {
            log.lock().push(format!("Timer_Nesting_{}", nesting));
            let log_micro = Arc::clone(&log);
            h.queue_microtask(move || {
                log_micro.lock().push(format!("Micro_Nesting_{}", nesting));
            });
        });
    }

    // Em t = 1ms: timers 1..4 expiram (delay = 1ms)
    mock_clock.advance_millis(1);
    for _ in 0..4 {
        event_loop.step();
    }

    {
        let logs = timer_log.lock().clone();
        assert!(logs.contains(&"Timer_Nesting_1".to_string()));
        assert!(logs.contains(&"Micro_Nesting_1".to_string()));
        assert!(logs.contains(&"Timer_Nesting_2".to_string()));
        assert!(logs.contains(&"Timer_Nesting_3".to_string()));
        assert!(logs.contains(&"Timer_Nesting_4".to_string()));
        assert!(!logs.contains(&"Timer_Nesting_5".to_string()), "Nível 5 sofre clamp para 4ms!");
    }

    // Avança para t = 4ms: nível 5 dispara
    mock_clock.advance_millis(3);
    event_loop.step();

    {
        let logs = timer_log.lock().clone();
        assert!(logs.contains(&"Timer_Nesting_5".to_string()));
        assert!(logs.contains(&"Micro_Nesting_5".to_string()));
    }
}

// =========================================================================
// 11. LayoutUnit + Color
// =========================================================================
#[test]
fn tier3_combo_11_layout_unit_and_color() {
    #[derive(Debug, Clone, Copy)]
    struct StyledBox {
        x: LayoutUnit,
        y: LayoutUnit,
        width: LayoutUnit,
        height: LayoutUnit,
        color: Color,
    }

    let box1 = StyledBox {
        x: LayoutUnit::from_f32_px(10.3333),
        y: LayoutUnit::from_f32_px(20.0),
        width: LayoutUnit::from_f32_px(100.3333),
        height: LayoutUnit::from_px(50),
        color: Color::from_rgba(255, 0, 0, 200),
    };

    let box2 = StyledBox {
        x: box1.x + box1.width,
        y: box1.y,
        width: LayoutUnit::from_f32_px(50.6666),
        height: LayoutUnit::from_px(50),
        color: Color::from_rgba(0, 0, 255, 200),
    };

    // Verificação de continuidade e ausência de pixel-cracking
    let right1 = box1.x + box1.width;
    let left2 = box2.x;
    assert_eq!(right1, left2, "Bordas adjacentes devem ser estritamente contíguas em LayoutUnit");
    assert_eq!(box1.height, LayoutUnit::from_px(50));
    assert_eq!(box2.height, LayoutUnit::from_px(50));

    // Interpolação de cores para degradê no layout
    let blended_color = box1.color.lerp(box2.color, 0.5);
    assert_eq!(blended_color.r, 128);
    assert_eq!(blended_color.b, 128);
    assert_eq!(blended_color.a, 200);

    // Pré-multiplicação alfa para rasterização GPU
    let (pr, pg, pb, pa) = blended_color.to_premultiplied_f32();
    let expected_alpha = 200.0 / 255.0;
    assert!((pa - expected_alpha).abs() < 1e-4);
    assert!((pr - (128.0 / 255.0) * expected_alpha).abs() < 1e-4);
    assert_eq!(pg, 0.0);
    assert!((pb - (128.0 / 255.0) * expected_alpha).abs() < 1e-4);
}

// =========================================================================
// 12. LayoutUnit + style_hint_to_node_flags
// =========================================================================
#[test]
fn tier3_combo_12_layout_unit_and_style_hint_to_node_flags() {
    struct LayoutBox {
        bounds: (LayoutUnit, LayoutUnit, LayoutUnit, LayoutUnit),
        flags: NodeFlags,
    }

    let mut lbox = LayoutBox {
        bounds: (
            LayoutUnit::from_px(0),
            LayoutUnit::from_px(0),
            LayoutUnit::from_px(200),
            LayoutUnit::from_px(100),
        ),
        flags: NodeFlags::empty(),
    };

    // 1. Mudança de estilo que requer apenas Repaint (ex: color)
    let hint_repaint = StyleChangeHint::REPAINT;
    lbox.flags |= style_hint_to_node_flags(hint_repaint);
    assert!(is_node_dirty(lbox.flags));
    assert!(lbox.flags.contains(NodeFlags::DIRTY_PAINT));
    assert!(!lbox.flags.contains(NodeFlags::DIRTY_LAYOUT));

    // 2. Mudança de estilo que requer Reflow Layout (ex: width)
    let hint_reflow = StyleChangeHint::REFLOW_LAYOUT;
    lbox.flags |= style_hint_to_node_flags(hint_reflow);
    assert!(lbox.flags.contains(NodeFlags::DIRTY_LAYOUT));

    // Se estiver dirty de layout, recalculamos as dimensões com LayoutUnit
    if lbox.flags.contains(NodeFlags::DIRTY_LAYOUT) {
        lbox.bounds.2 = LayoutUnit::from_f32_px(350.5); // Novo width
        lbox.flags.remove(NodeFlags::DIRTY_LAYOUT | NodeFlags::DIRTY_PAINT);
    }

    assert_eq!(lbox.bounds.2.to_f32_px(), 350.5);
    assert_eq!(lbox.bounds.2.raw(), 21030); // 350.5 * 60 = 21030
    assert!(!is_node_dirty(lbox.flags));
}

// =========================================================================
// 13. BreadcrumbBuffer + EventLoop
// =========================================================================
#[test]
fn tier3_combo_13_breadcrumb_buffer_and_event_loop() {
    let mock_clock = Arc::new(MockClock::new(100));
    let event_loop = EventLoop::with_clock(Arc::clone(&mock_clock) as Arc<dyn Clock>);
    let handle = event_loop.handle();
    let breadcrumbs = Arc::new(BreadcrumbBuffer::<16>::new());

    let bc1 = Arc::clone(&breadcrumbs);
    let clk1 = Arc::clone(&mock_clock);
    handle.queue_user_interaction(move || {
        bc1.record("EventLoop", "Dispatch UserInteraction", clk1.now_ms());
    });

    let bc2 = Arc::clone(&breadcrumbs);
    let clk2 = Arc::clone(&mock_clock);
    handle.queue_dom(move || {
        bc2.record("EventLoop", "Dispatch DomManipulation", clk2.now_ms());
    });

    mock_clock.advance_millis(15);
    event_loop.step();

    mock_clock.advance_millis(20);
    event_loop.step();

    let snap = breadcrumbs.snapshot();
    assert_eq!(snap.len(), 2);
    assert_eq!(snap[0].category, "EventLoop");
    assert_eq!(snap[0].message, "Dispatch UserInteraction");
    assert_eq!(snap[0].timestamp_ms, 115);
    assert_eq!(snap[1].message, "Dispatch DomManipulation");
    assert_eq!(snap[1].timestamp_ms, 135);
}

// =========================================================================
// 14. BreadcrumbBuffer + Arena
// =========================================================================
#[test]
fn tier3_combo_14_breadcrumb_buffer_and_arena() {
    let breadcrumbs = BreadcrumbBuffer::<8>::new();
    let mut arena: Arena<String> = Arena::new();

    let id1 = arena.alloc("Node 1".to_string());
    breadcrumbs.record("Arena", format!("Allocated slot {:?}", id1), 10);

    let id2 = arena.alloc("Node 2".to_string());
    breadcrumbs.record("Arena", format!("Allocated slot {:?}", id2), 20);

    arena.remove(id1);
    breadcrumbs.record("Arena", format!("Removed slot {:?}", id1), 30);

    let id3 = arena.alloc("Node 3".to_string());
    breadcrumbs.record("Arena", format!("Reused slot {:?}", id3), 40);

    let stats = arena.stats();
    assert_eq!(stats.live, 2);
    assert_eq!(stats.slot_reuses, 1);

    let snap = breadcrumbs.snapshot();
    assert_eq!(snap.len(), 4);
    assert_eq!(snap[0].message, format!("Allocated slot {:?}", id1));
    assert_eq!(snap[2].message, format!("Removed slot {:?}", id1));
    assert_eq!(snap[3].message, format!("Reused slot {:?}", id3));
}

// =========================================================================
// 15. InlineVec + BreadcrumbBuffer
// =========================================================================
#[test]
fn tier3_combo_15_inline_vec_and_breadcrumb_buffer() {
    let breadcrumbs = BreadcrumbBuffer::<32>::new();

    // Acumula eventos diagnósticos em lote sem alocação heap inicial
    let mut local_batch: InlineVec<BreadcrumbEntry, 4> = InlineVec::new();
    local_batch.push(BreadcrumbEntry {
        category: "Parser",
        message: "Start tag <html>".to_string(),
        timestamp_ms: 100,
    });
    local_batch.push(BreadcrumbEntry {
        category: "Parser",
        message: "Start tag <body>".to_string(),
        timestamp_ms: 102,
    });
    local_batch.push(BreadcrumbEntry {
        category: "Parser",
        message: "Start tag <div>".to_string(),
        timestamp_ms: 105,
    });
    assert!(local_batch.is_inline());

    // Flush em lote para o BreadcrumbBuffer
    for entry in local_batch.drain(..) {
        breadcrumbs.record(entry.category, entry.message, entry.timestamp_ms);
    }

    let snap = breadcrumbs.snapshot();
    assert_eq!(snap.len(), 3);
    assert_eq!(snap[0].message, "Start tag <html>");
    assert_eq!(snap[2].message, "Start tag <div>");
}

// =========================================================================
// 16. Color (Bradford Adaptation) + Color (Polar Interpolation)
// =========================================================================
#[test]
fn tier3_combo_16_color_bradford_adaptation_and_polar_interpolation() {
    // 1. Oklab e Oklch roundtrips
    let srgb_pure_red = Color::RED;
    let oklab_red = srgb_pure_red.to_oklab();
    assert!((oklab_red.l - 0.627).abs() < 0.05);

    let oklch_red = srgb_pure_red.to_oklch();
    assert!(oklch_red.c > 0.2);

    // 2. Interpolação polar no espaço Oklch (menor arco angular)
    let c_start = Color::from_oklch(Oklch::new(0.6, 0.2, 350.0, 1.0));
    let c_end = Color::from_oklch(Oklch::new(0.6, 0.2, 10.0, 1.0));
    let c_mid = c_start.lerp_oklch(c_end, 0.5);
    let mid_oklch = c_mid.to_oklch();
    // O meio angular entre 350° e 10° deve passar por 0° / 360° (menor arco de 20°)
    assert!(
        mid_oklch.h < 5.0 || mid_oklch.h > 355.0,
        "Interpolação de menor arco deve passar por 0°, obteve: {}",
        mid_oklch.h
    );

    // 3. Conversões de Wide-Gamut Display-P3 <-> sRGB
    let (p3_r, p3_g, p3_b, p3_a) = srgb_to_display_p3(1.0, 0.0, 0.0, 1.0);
    let (back_r, back_g, back_b, back_a) = display_p3_to_srgb(p3_r, p3_g, p3_b, p3_a);
    assert!((back_r - 1.0).abs() < 1e-3);
    assert!(back_g.abs() < 1e-3);
    assert!(back_b.abs() < 1e-3);
    assert_eq!(back_a, 1.0);

    // 4. color-mix parsing no espaço oklab
    let mixed = mix_colors(
        ColorSpace::Oklab,
        Color::RED,
        Some(0.5),
        Color::BLUE,
        Some(0.5),
    )
    .unwrap();
    assert!(mixed.r > 50);
    assert!(mixed.b > 50);
}

// =========================================================================
// 17. UnguessableToken + compute_referrer
// =========================================================================
#[test]
fn tier3_combo_17_unguessable_token_and_compute_referrer() {
    let token = UnguessableToken::new();
    let auth_url = format!("https://auth.bank.com/session?token={}#credentials", token.to_hex());
    let bank_origin = Origin::parse(&auth_url).unwrap();

    // 1. Cross-origin request para terceiro com StrictOriginWhenCrossOrigin
    let referrer = compute_referrer(
        &bank_origin,
        &auth_url,
        "https://analytics.thirdparty.com/track",
        ReferrerPolicy::StrictOriginWhenCrossOrigin,
    );
    assert_eq!(
        referrer,
        Some("https://auth.bank.com".to_string()),
        "Token e parâmetros confidenciais não devem vazar para cross-origin"
    );
    assert!(
        !referrer.unwrap().contains(&token.to_hex()),
        "Token nunca deve constar no cabeçalho cross-origin"
    );

    // 2. Same-origin request: fragmento (#credentials) é suprimido
    let same_origin_ref = compute_referrer(
        &bank_origin,
        &auth_url,
        "https://auth.bank.com/dashboard",
        ReferrerPolicy::SameOrigin,
    );
    assert!(same_origin_ref.is_some());
    let sanitized = same_origin_ref.unwrap();
    assert!(!sanitized.contains("#credentials"), "Fragmentos devem ser removidos");
}

// =========================================================================
// 18. TripleBuffer + LayoutUnit
// =========================================================================
#[test]
fn tier3_combo_18_triple_buffer_and_layout_unit() {
    #[derive(Debug, Clone, PartialEq)]
    struct LayoutPassOutput {
        viewport_width: LayoutUnit,
        viewport_height: LayoutUnit,
        rects: InlineVec<(LayoutUnit, LayoutUnit, LayoutUnit, LayoutUnit), 4>,
    }

    let initial = LayoutPassOutput {
        viewport_width: LayoutUnit::from_px(800),
        viewport_height: LayoutUnit::from_px(600),
        rects: InlineVec::new(),
    };

    let (mut producer, mut consumer) = triple_buffer(initial);

    // Thread de Layout produz um frame com caixas subpixel
    let mut rects = InlineVec::new();
    rects.push((
        LayoutUnit::from_f32_px(0.0),
        LayoutUnit::from_f32_px(0.0),
        LayoutUnit::from_f32_px(399.5),
        LayoutUnit::from_px(100),
    ));
    rects.push((
        LayoutUnit::from_f32_px(399.5),
        LayoutUnit::from_f32_px(0.0),
        LayoutUnit::from_f32_px(400.5),
        LayoutUnit::from_px(100),
    ));

    producer.write(LayoutPassOutput {
        viewport_width: LayoutUnit::from_px(800),
        viewport_height: LayoutUnit::from_px(600),
        rects,
    });
    producer.publish();

    // Thread de Renderização consome o frame zero-copy e realiza box snapping
    let frame = consumer.consume().expect("Frame de layout deve estar pronto");
    assert_eq!(frame.rects.len(), 2);

    let box1 = frame.rects[0];
    let box2 = frame.rects[1];

    // Continuidade de subpixels
    assert_eq!(box1.0 + box1.2, box2.0);
    assert_eq!(box1.0.floor_px(), 0);
    assert_eq!(box2.0.round_px(), 400);
}

// =========================================================================
// 19. Arena + InlineVec + style_hint_to_node_flags
// =========================================================================
#[test]
fn tier3_combo_19_arena_inline_vec_and_style_hint_to_node_flags() {
    #[derive(Debug)]
    struct DomElement {
        tag: &'static str,
        flags: NodeFlags,
        width: LayoutUnit,
        height: LayoutUnit,
        children: InlineVec<ArenaId<DomElement>, 4>,
    }

    let mut arena: Arena<DomElement> = Arena::new();

    // Monta hierarquia: Root -> Child1, Child2
    let c1_id = arena.alloc(DomElement {
        tag: "span",
        flags: NodeFlags::IS_ELEMENT,
        width: LayoutUnit::from_px(50),
        height: LayoutUnit::from_px(20),
        children: InlineVec::new(),
    });
    let c2_id = arena.alloc(DomElement {
        tag: "p",
        flags: NodeFlags::IS_ELEMENT,
        width: LayoutUnit::from_px(100),
        height: LayoutUnit::from_px(40),
        children: InlineVec::new(),
    });

    let mut root_children = InlineVec::new();
    root_children.push(c1_id);
    root_children.push(c2_id);

    let root_id = arena.alloc(DomElement {
        tag: "div",
        flags: NodeFlags::IS_ELEMENT,
        width: LayoutUnit::from_px(200),
        height: LayoutUnit::from_px(100),
        children: root_children,
    });

    // 1. Aplica dica de estilo SUBTREE_RECALC na raiz
    let hint = StyleChangeHint::SUBTREE_RECALC;
    let dirty_flags = style_hint_to_node_flags(hint);

    let root = arena.get_mut(root_id).unwrap();
    root.flags |= dirty_flags;
    assert!(root.flags.contains(NodeFlags::SUBTREE_DIRTY));
    assert!(root.flags.contains(NodeFlags::DIRTY_STYLE));

    // 2. Propaga sujeira de estilo pelos nós filhos usando InlineVec na Arena
    let child_ids: Vec<ArenaId<DomElement>> = root.children.iter().copied().collect();
    for cid in child_ids {
        let child = arena.get_mut(cid).unwrap();
        child.flags |= NodeFlags::DIRTY_STYLE;
        assert!(is_node_dirty(child.flags));
    }

    // 3. Passe de recalculo de estilo e layout
    for (_id, elem) in arena.iter_mut() {
        if elem.flags.contains(NodeFlags::DIRTY_STYLE) {
            elem.width += LayoutUnit::from_px(10); // Ajuste de padding
            elem.flags.remove(NodeFlags::DIRTY_STYLE | NodeFlags::SUBTREE_DIRTY);
        }
    }

    // Validação de estado limpo e dimensões atualizadas
    let updated_root = arena.get(root_id).unwrap();
    assert_eq!(updated_root.tag, "div");
    assert_eq!(updated_root.height, LayoutUnit::from_px(100));
    assert_eq!(updated_root.width, LayoutUnit::from_px(210));
    assert!(!is_node_dirty(updated_root.flags));

    let updated_c1 = arena.get(c1_id).unwrap();
    assert_eq!(updated_c1.tag, "span");
    assert_eq!(updated_c1.height, LayoutUnit::from_px(20));
    assert_eq!(updated_c1.width, LayoutUnit::from_px(60));
    assert!(!is_node_dirty(updated_c1.flags));
}
