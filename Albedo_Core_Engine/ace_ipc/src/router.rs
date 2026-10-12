//! # Roteador de Mensagens e Tópicos IPC (`IpcRouter`)
//!
//! Permite publicação e assinatura concorrente (Pub/Sub) baseada em tópicos
//! para coordenação desacoplada entre os subsistemas do navegador.

use crate::channel::{unbounded_channel, IpcReceiver, IpcSender};
use crate::error::IpcError;
use crate::protocol::IpcMessage;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use std::sync::Arc;

/// Tópicos de comunicação do Albedo Core Engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Topic {
    /// Eventos de navegação e ciclo de vida de documento
    Navigation,
    /// Requisições e streaming de rede
    Network,
    /// Mutações na árvore de nós do DOM
    Dom,
    /// Atualizações da árvore de renderização e estilo
    Render,
    /// Entrada de usuário (mouse, teclado)
    Input,
}

/// Roteador thread-safe de mensagens IPC.
#[derive(Debug, Default, Clone)]
pub struct IpcRouter {
    subscribers: Arc<RwLock<FxHashMap<Topic, Vec<IpcSender<IpcMessage>>>>>,
}

impl IpcRouter {
    /// Cria uma nova instância vazia do roteador IPC.
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }

    /// Inscreve um receptor em um tópico específico.
    pub fn subscribe(&self, topic: Topic) -> IpcReceiver<IpcMessage> {
        let (tx, rx) = unbounded_channel();
        let mut subs = self.subscribers.write();
        subs.entry(topic).or_default().push(tx);
        rx
    }

    /// Publica uma mensagem em um tópico específico, entregando a todos os inscritos ativos.
    pub fn publish(&self, topic: Topic, msg: IpcMessage) -> Result<usize, IpcError> {
        let mut subs = self.subscribers.write();
        let list = match subs.get_mut(&topic) {
            Some(l) => l,
            None => return Ok(0),
        };

        let mut delivered = 0;
        // Retém apenas os emissores que ainda têm receptores ativos
        list.retain(|sender| {
            if sender.send(msg.clone()).is_ok() {
                delivered += 1;
                true
            } else {
                false // Canal desconectado
            }
        });

        Ok(delivered)
    }

    /// Limpa todos os canais e assinaturas do roteador.
    pub fn clear(&self) {
        self.subscribers.write().clear();
    }
}
