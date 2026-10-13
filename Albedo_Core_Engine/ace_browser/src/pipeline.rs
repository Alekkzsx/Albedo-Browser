//! # Pipeline Integrado Mestre do Navegador (`BrowserPipeline`)
//!
//! Orquestra o ciclo completo de vida de navegação web unificando:
//! `ace_ipc` -> `ace_net` -> `ace_dom` -> `ace_style` -> `RenderTree`.

use ace_core::id::NodeId;
use ace_dom::parser::StreamingHtmlParser;
use ace_dom::tree::Document;
use ace_ipc::protocol::{DomMutationPayload, DomMutationType, RenderFramePayload};
use ace_ipc::{IpcMessage, IpcRouter, Topic};
use ace_net::engine::css_loader::CssLoaderService;
use ace_net::engine::fetcher::ResourceFetcher;
use ace_net::engine::priority::PriorityLevel;
use ace_style::cascade::origin::StyleSheetOrigin;
use ace_style::engine::invalidation::DOMMutation;
use ace_style::engine::restyle_driver::RestyleDriver;
use ace_style::engine::style_engine::{RenderTree, StyleEngine};
use ace_style::model::stylesheet::StyleSheet;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::LogicalSize;
use ace_layout::engine::LayoutEngine;
use ace_layout::fragment::FragmentTree;
use std::sync::Arc;
use thiserror::Error;
use url::Url;

/// Erros decorrentes da execução do pipeline do navegador.
#[derive(Debug, Error)]
pub enum PipelineError {
    /// Falha nas operações de requisição de rede ou transporte.
    #[error("Erro de rede no pipeline: {0}")]
    NetworkError(String),

    /// Falha na transmissão ou roteamento de mensagens IPC.
    #[error("Erro de IPC no pipeline: {0}")]
    IpcError(String),

    /// Tentativa de aplicar mutação sem documento ativo carregado.
    #[error("Documento não carregado ou inativo")]
    NoActiveDocument,

    /// URL malformada ou inválida.
    #[error("URL inválida: {0}")]
    InvalidUrl(String),
}

/// O orquestrador central do navegador que conecta todos os subsistemas em fluxo contínuo.
pub struct BrowserPipeline {
    /// Roteador pub/sub de IPC in-process.
    pub router: IpcRouter,
    /// Orquestrador de transporte de rede e cache.
    pub fetcher: Arc<ResourceFetcher>,
    /// Carregador assíncrono de folhas de estilo CSS.
    pub css_loader: CssLoaderService,
    /// Documento DOM ativo atualmente no pipeline.
    pub document: Option<Document>,
    /// Driver de reestilização reativa incremental.
    pub driver: Option<RestyleDriver>,
    /// Motor de layout geométrico (LayoutEngine).
    pub layout_engine: LayoutEngine,
    /// Árvore de fragmentos física e imutável ativa.
    pub fragment_tree: Option<Arc<FragmentTree>>,
}

impl BrowserPipeline {
    /// Inicializa uma nova instância do pipeline do navegador com todos os motores integrados.
    pub fn new() -> Result<Self, PipelineError> {
        let router = IpcRouter::new();
        let fetcher = Arc::new(
            ResourceFetcher::new().map_err(|e| PipelineError::NetworkError(e.to_string()))?,
        );
        let css_loader = CssLoaderService::new(fetcher.clone());
        let layout_engine = LayoutEngine::new();

        Ok(Self {
            router,
            fetcher,
            css_loader,
            document: None,
            driver: None,
            layout_engine,
            fragment_tree: None,
        })
    }

    /// Executa o ciclo de navegação completo:
    /// Requisição de rede -> Streaming de HTML -> Construção da Árvore DOM -> Carregamento de CSS -> RenderTree.
    pub async fn navigate(&mut self, url_str: &str) -> Result<RenderTree, PipelineError> {
        // 1. Notifica início de navegação no barramento IPC
        let _ = self.router.publish(
            Topic::Navigation,
            IpcMessage::Navigate {
                url: url_str.to_string(),
                referrer: None,
            },
        );

        // 2. Busca o recurso principal com prioridade de caminho crítico (VeryHigh)
        let response = self
            .fetcher
            .fetch_url(url_str, PriorityLevel::VeryHigh)
            .await
            .map_err(|e| PipelineError::NetworkError(e.to_string()))?;

        // 3. Alimenta o parser streaming em pedaços contínuos
        let body_bytes = response
            .collect_bytes()
            .await
            .map_err(|e| PipelineError::NetworkError(e.to_string()))?;

        let mut streaming_parser = StreamingHtmlParser::with_capacity(body_bytes.len());
        streaming_parser.feed_chunk(&body_bytes);
        let doc = streaming_parser.finish();

        // 4. Prepara o motor de estilo e descobre folhas de estilo inline e linkadas
        let mut style_engine = StyleEngine::new();
        let base_url = Url::parse(url_str).ok();

        // Extrai folhas de estilo de tags <style> e <link rel="stylesheet">
        if let Some(doc_element_id) = doc.document_element {
            for (desc_id, desc_node) in doc.descendants(doc_element_id) {
                if let Some(el) = desc_node.as_element() {
                    if el.tag_name == "style" {
                        // Coleta o texto de nós filhos de texto
                        let mut css_text = String::new();
                        for (child_id, _) in doc.children(desc_id) {
                            if let Some(child_node) = doc.get_node(child_id) {
                                if let Some(text) = child_node.text_content() {
                                    css_text.push_str(text);
                                }
                            }
                        }
                        if !css_text.is_empty() {
                            let sheet = StyleSheet::parse(&css_text, StyleSheetOrigin::Author);
                            style_engine.add_stylesheet(sheet);
                        }
                    } else if el.tag_name == "link" {
                        if let Some(rel) = el.get_attribute("rel") {
                            if rel.eq_ignore_ascii_case("stylesheet") {
                                if let Some(href) = el.get_attribute("href") {
                                    let resolved_url = if let Some(ref base) = base_url {
                                        base.join(href)
                                            .map(|u| u.to_string())
                                            .unwrap_or(href.to_string())
                                    } else {
                                        href.to_string()
                                    };

                                    if let Ok(css_content) =
                                        self.css_loader.load_stylesheet(&resolved_url).await
                                    {
                                        let sheet = StyleSheet::parse(
                                            &css_content,
                                            StyleSheetOrigin::Author,
                                        );
                                        style_engine.add_stylesheet(sheet);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 5. Inicializa o RestyleDriver e compila a RenderTree inicial
        let restyle_driver = RestyleDriver::new(style_engine, &doc);
        let render_tree = restyle_driver.render_tree().clone();

        // 6. Executa layout produzindo a FragmentTree
        let viewport = LogicalSize::new(LayoutUnit::from_px(1024), LayoutUnit::from_px(768));
        let fragment_tree = self
            .layout_engine
            .layout_tree(&render_tree, &doc, viewport)
            .ok();
        self.fragment_tree = fragment_tree.clone();

        // 7. Publica evento de RenderFrame no barramento IPC
        let nodes_count = fragment_tree
            .as_ref()
            .map(|t| t.total_fragments())
            .unwrap_or_else(|| render_tree.node_styles.len());
        let _ = self.router.publish(
            Topic::Render,
            IpcMessage::RenderFrame(RenderFramePayload {
                doc_id: 1,
                root_node_id: doc.root().raw(),
                nodes_count,
            }),
        );

        self.document = Some(doc);
        self.driver = Some(restyle_driver);

        Ok(render_tree)
    }

    /// Aplica uma mutação dinâmica no DOM e aciona a reestilização reativa instantânea.
    pub fn apply_mutation(
        &mut self,
        node_id: NodeId,
        mutation: &DOMMutation,
    ) -> Result<usize, PipelineError> {
        let doc = self
            .document
            .as_ref()
            .ok_or(PipelineError::NoActiveDocument)?;
        let driver = self
            .driver
            .as_mut()
            .ok_or(PipelineError::NoActiveDocument)?;

        let invalidated = driver.on_mutation(doc, node_id, mutation);

        // Notifica mutação no IPC
        let ipc_mutation_type = match mutation {
            DOMMutation::ClassChanged {
                old_class,
                new_class,
            } => DomMutationType::ClassChanged {
                old_class: old_class.as_ref().map(|a| a.as_str().to_string()),
                new_class: new_class.as_ref().map(|a| a.as_str().to_string()),
            },
            DOMMutation::AttributeChanged { name } => DomMutationType::AttributeChanged {
                name: name.as_str().to_string(),
                value: None,
            },
            DOMMutation::IdChanged { old_id, new_id } => DomMutationType::IdChanged {
                old_id: old_id.as_ref().map(|a| a.as_str().to_string()),
                new_id: new_id.as_ref().map(|a| a.as_str().to_string()),
            },
        };

        let _ = self.router.publish(
            Topic::Dom,
            IpcMessage::DomMutation(DomMutationPayload {
                doc_id: 1,
                target_node_id: node_id.raw(),
                mutation_type: ipc_mutation_type,
            }),
        );

        // Notifica nova RenderFrame atualizada
        let nodes_count = driver.render_tree().node_styles.len();
        let _ = self.router.publish(
            Topic::Render,
            IpcMessage::RenderFrame(RenderFramePayload {
                doc_id: 1,
                root_node_id: doc.root().raw(),
                nodes_count,
            }),
        );

        Ok(invalidated)
    }

    /// Retorna o documento DOM atualmente ativo no pipeline.
    pub fn document(&self) -> Option<&Document> {
        self.document.as_ref()
    }

    /// Retorna a RenderTree atualmente ativa no pipeline.
    pub fn render_tree(&self) -> Option<&RenderTree> {
        self.driver.as_ref().map(|d| d.render_tree())
    }

    /// Retorna a FragmentTree atualmente ativa no pipeline, se disponível.
    pub fn fragment_tree(&self) -> Option<Arc<FragmentTree>> {
        self.fragment_tree.clone()
    }
}
