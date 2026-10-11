//! # Protocolo de Mensagens Fundamentais do Navegador (`IpcMessage`)
//!
//! Estruturas normativas de comunicação entre os subsistemas de UI, Rede, DOM,
//! Estilo e Renderização do Albedo.

/// Tipo de mutação no DOM comunicada via barramento IPC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomMutationType {
    /// Modificação ou adição de atributo.
    AttributeChanged {
        /// Nome do atributo modificado.
        name: String,
        /// Novo valor textual (ou None se removido).
        value: Option<String>,
    },
    /// Alteração específica de classes CSS.
    ClassChanged {
        /// Classe anterior.
        old_class: Option<String>,
        /// Nova classe atribuída.
        new_class: Option<String>,
    },
    /// Alteração de identificador (`id`).
    IdChanged {
        /// ID anterior.
        old_id: Option<String>,
        /// Novo ID atribuído.
        new_id: Option<String>,
    },
    /// Inserção de nó filho.
    NodeInserted {
        /// ID do nó pai.
        parent_id: u64,
        /// ID do nó filho inserido.
        child_id: u64,
    },
    /// Remoção de nó filho.
    NodeRemoved {
        /// ID do nó pai.
        parent_id: u64,
        /// ID do nó filho removido.
        child_id: u64,
    },
}

/// Payload completo de uma mutação no DOM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomMutationPayload {
    /// Identificador do documento.
    pub doc_id: u64,
    /// Identificador do nó alvo da mutação.
    pub target_node_id: u64,
    /// Detalhamento da mutação.
    pub mutation_type: DomMutationType,
}

/// Payload de requisição de recurso de rede.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRequestPayload {
    /// ID unívoco da requisição.
    pub request_id: u64,
    /// URL solicitada.
    pub url: String,
    /// Nível de prioridade numérica (RFC 9218).
    pub priority: u8,
    /// Iniciador da requisição (ex: "parser", "css", "script").
    pub initiator: String,
}

/// Payload de dados de recurso recebidos em streaming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDataPayload {
    /// ID da requisição correspondente.
    pub request_id: u64,
    /// Pedaço de bytes binários da resposta.
    pub chunk: Vec<u8>,
    /// Indicador de fim de fluxo de dados.
    pub is_eof: bool,
}

/// Payload de eventos de entrada do usuário (mouse, teclado, touch).
#[derive(Debug, Clone, PartialEq)]
pub struct InputEventPayload {
    /// Tipo de evento (0: MouseMove, 1: MouseDown, 2: MouseUp, 3: KeyDown, 4: KeyUp).
    pub event_kind: u8,
    /// Coordenada horizontal X na janela.
    pub x: f32,
    /// Coordenada vertical Y na janela.
    pub y: f32,
    /// Código da tecla física ou virtual.
    pub key_code: u32,
}

/// Payload de notificação de atualização da árvore de renderização.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderFramePayload {
    /// Identificador do documento.
    pub doc_id: u64,
    /// Identificador do nó raiz da árvore de renderização.
    pub root_node_id: u64,
    /// Contagem total de caixas renderizáveis.
    pub nodes_count: usize,
}

/// Mensagem mestra de IPC trafegada entre subsistemas.
#[derive(Debug, Clone, PartialEq)]
pub enum IpcMessage {
    /// Solicitação de navegação para uma URL.
    Navigate {
        /// URL alvo.
        url: String,
        /// Referrer opcional da navegação.
        referrer: Option<String>,
    },
    /// Solicitação de carregamento de recurso de rede.
    LoadResource(ResourceRequestPayload),
    /// Pedaço de dados recebidos da rede em streaming.
    ResourceData(ResourceDataPayload),
    /// Mutação ocorrida no DOM que requer reação do motor de estilo.
    DomMutation(DomMutationPayload),
    /// Árvore de renderização recalculada e pronta para o layout / compositor.
    RenderFrame(RenderFramePayload),
    /// Evento de entrada de usuário despachado da janela principal.
    InputEvent(InputEventPayload),
    /// Sinal de desligamento ordenado de subsistema.
    Shutdown,
}
