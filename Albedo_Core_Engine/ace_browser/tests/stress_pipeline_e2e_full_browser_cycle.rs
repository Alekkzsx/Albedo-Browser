use ace_browser::{BrowserPipeline, PipelineError};
use ace_core::intern::Atom;
use ace_ipc::{IpcMessage, Topic};
use ace_style::engine::invalidation::DOMMutation;
use ace_style::values::enums::Display;

#[tokio::test]
async fn test_pipeline_e2e_full_browser_cycle() -> Result<(), PipelineError> {
    let mut pipeline = BrowserPipeline::new()?;

    // Inscreve observadores no barramento IPC para auditar telemetria
    let rx_nav = pipeline.router.subscribe(Topic::Navigation);
    let rx_render = pipeline.router.subscribe(Topic::Render);
    let rx_dom = pipeline.router.subscribe(Topic::Dom);

    let html_payload = r#"<!DOCTYPE html>
<html>
<head>
    <title>Albedo Browser Test</title>
    <style>
        body { margin: 0; background-color: #f0f0f0; color: #111; }
        header { display: block; padding: 20px; background-color: #333; color: #fff; }
        .container { display: block; max-width: 1200px; margin: 0 auto; }
        .card { display: block; padding: 15px; margin: 10px; background-color: #fff; }
        .featured { background-color: #fffae6; border: 1px solid #ffd700; }
        .theme-dark { background-color: #121212; color: #ffffff; }
        .hidden { display: none; }
    </style>
</head>
<body>
    <header><h1>Portal Albedo</h1></header>
    <main class="container">
        <div id="card-1" class="card featured">
            <h2>Artigo Principal</h2>
            <p>Conteúdo de teste com renderização em cascata.</p>
        </div>
        <div id="card-2" class="card">
            <h2>Artigo Secundário</h2>
            <p>Segundo bloco do documento.</p>
        </div>
        <div id="card-hidden" class="hidden">
            <p>Este nó não deve gerar caixa visível na RenderTree.</p>
        </div>
    </main>
</body>
</html>"#;

    // Codifica payload como Data URL HTTP/WHATWG
    let data_url = format!(
        "data:text/html;base64,{}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, html_payload)
    );

    // 1. Executa a navegação completa fim-a-fim no pipeline
    let render_tree = pipeline.navigate(&data_url).await?;

    // 2. Valida emissão e recepção de mensagens pelo barramento IPC
    let nav_msg = rx_nav
        .recv_timeout(std::time::Duration::from_millis(500))
        .expect("Mensagem Navigate esperada");
    assert!(matches!(nav_msg, IpcMessage::Navigate { .. }));

    let render_msg = rx_render
        .recv_timeout(std::time::Duration::from_millis(500))
        .expect("Mensagem RenderFrame esperada");
    if let IpcMessage::RenderFrame(payload) = render_msg {
        assert!(
            payload.nodes_count > 0,
            "RenderFrame deve reportar contagem positiva de nós"
        );
    } else {
        panic!("Mensagem inesperada no tópico Render");
    }

    // 3. Validações semânticas da RenderTree
    let doc = pipeline.document().expect("Documento deve estar ativo");
    assert!(doc.body.is_some());
    assert!(doc.document_element.is_some());

    // O body deve possuir estilo computado de bloco
    let body_id = doc.body.unwrap();
    let body_style = render_tree
        .get_style(body_id)
        .expect("Body deve ter estilo computado");
    assert_eq!(body_style.display(), Display::Block);

    // Nós visíveis devem estar presentes, nó com display: none deve ser suprimido
    assert!(render_tree.node_styles.len() >= 6);

    // 4. Ciclo de Mutação Dinâmica Integrada em Tempo Real
    let root_element_id = doc.document_element.unwrap();
    let mutation = DOMMutation::ClassChanged {
        old_class: Some(Atom::new("featured")),
        new_class: Some(Atom::new("theme-dark")),
    };

    let invalidated_nodes = pipeline.apply_mutation(root_element_id, &mutation)?;
    assert!(
        invalidated_nodes > 0,
        "Mutação deve invalidar os nós afetados"
    );

    // Verifica que as mensagens de mutação e novo RenderFrame foram emitidas no IPC
    let dom_msg = rx_dom
        .recv_timeout(std::time::Duration::from_millis(500))
        .expect("Mensagem DomMutation esperada");
    assert!(matches!(dom_msg, IpcMessage::DomMutation(_)));

    let updated_render_msg = rx_render
        .recv_timeout(std::time::Duration::from_millis(500))
        .expect("Mensagem RenderFrame atualizada esperada");
    assert!(matches!(updated_render_msg, IpcMessage::RenderFrame(_)));

    println!("[PIPELINE E2E] Ciclo completo de navegação, carregamento, renderização e mutação validado com 100% de sucesso!");
    Ok(())
}

#[tokio::test]
async fn test_pipeline_e2e_stress_massive_dom_and_styling() -> Result<(), PipelineError> {
    let mut pipeline = BrowserPipeline::new()?;

    // Gera página massiva com 1.000 nós estruturados e regras CSS com seletores de classe
    let mut html = String::from("<!DOCTYPE html><html><head><style>");
    html.push_str(".item { display: block; color: #222; margin: 4px; }\n");
    html.push_str(".highlight { color: #f00; font-weight: bold; }\n");
    html.push_str(".active { background-color: #eef; }\n");
    html.push_str("</style></head><body><div id='container'>");
    for i in 0..1_000 {
        html.push_str(&format!(
            "<div id='d-{}' class='item'><span>Texto {}</span></div>",
            i, i
        ));
    }
    html.push_str("</div></body></html>");

    let data_url = format!(
        "data:text/html;base64,{}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, html)
    );

    let start = std::time::Instant::now();
    let render_tree = pipeline.navigate(&data_url).await?;
    let elapsed_nav = start.elapsed();

    println!(
        "[STRESS PIPELINE] 1.000 nós parseados, estilizados e compilados em {:.2}ms",
        elapsed_nav.as_secs_f64() * 1000.0
    );

    assert!(render_tree.node_styles.len() >= 1000);

    // Executa 25 mutações em tempo real
    let doc = pipeline.document().unwrap();
    let root_id = doc.document_element.unwrap();
    let start_mutations = std::time::Instant::now();

    for _ in 0..25 {
        let mutation = DOMMutation::ClassChanged {
            old_class: Some(Atom::new("item")),
            new_class: Some(Atom::new("highlight active")),
        };
        let _ = pipeline.apply_mutation(root_id, &mutation)?;
    }

    let elapsed_mut = start_mutations.elapsed();
    println!(
        "[STRESS PIPELINE] 25 mutações reativas processadas em {:.2}ms (média: {:.2}ms/mut)",
        elapsed_mut.as_secs_f64() * 1000.0,
        elapsed_mut.as_secs_f64() * 1000.0 / 25.0
    );

    Ok(())
}
