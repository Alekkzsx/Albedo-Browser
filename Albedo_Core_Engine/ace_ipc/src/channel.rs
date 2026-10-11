//! # Canais Tipados In-Process (`IpcSender`, `IpcReceiver`)
//!
//! Fornece comunicação entre threads e subsistemas do Albedo Browser com latência
//! ultrabaixa e zero alocações desnecessárias.

use crate::error::IpcError;
use crossbeam_channel::{bounded, unbounded, Receiver, Sender, TryRecvError, TrySendError};
use std::time::Duration;

/// Emissor de mensagens fortemente tipadas através do canal IPC.
#[derive(Debug, Clone)]
pub struct IpcSender<T> {
    inner: Sender<T>,
}

/// Receptor de mensagens fortemente tipadas através do canal IPC.
#[derive(Debug, Clone)]
pub struct IpcReceiver<T> {
    inner: Receiver<T>,
}

/// Cria um par de canais IPC delimitado (bounded) com capacidade fixa (ring-buffer semantics).
pub fn bounded_channel<T>(capacity: usize) -> (IpcSender<T>, IpcReceiver<T>) {
    let (tx, rx) = bounded(capacity);
    (IpcSender { inner: tx }, IpcReceiver { inner: rx })
}

/// Cria um par de canais IPC não-delimitado (unbounded).
pub fn unbounded_channel<T>() -> (IpcSender<T>, IpcReceiver<T>) {
    let (tx, rx) = unbounded();
    (IpcSender { inner: tx }, IpcReceiver { inner: rx })
}

impl<T> IpcSender<T> {
    /// Envia uma mensagem bloqueando até que haja espaço no canal.
    pub fn send(&self, msg: T) -> Result<(), IpcError> {
        self.inner.send(msg).map_err(|_| IpcError::Disconnected)
    }

    /// Envia uma mensagem de forma não-bloqueante.
    pub fn try_send(&self, msg: T) -> Result<(), IpcError> {
        match self.inner.try_send(msg) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(IpcError::Full),
            Err(TrySendError::Disconnected(_)) => Err(IpcError::Disconnected),
        }
    }

    /// Envia uma mensagem aguardando até o limite do timeout.
    pub fn send_timeout(&self, msg: T, timeout: Duration) -> Result<(), IpcError> {
        self.inner.send_timeout(msg, timeout).map_err(|e| match e {
            crossbeam_channel::SendTimeoutError::Timeout(_) => IpcError::Timeout,
            crossbeam_channel::SendTimeoutError::Disconnected(_) => IpcError::Disconnected,
        })
    }

    /// Verifica se o canal está cheio.
    pub fn is_full(&self) -> bool {
        self.inner.is_full()
    }

    /// Retorna a quantidade de itens atualmente enfileirados no canal.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Verifica se o canal está vazio.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<T> IpcReceiver<T> {
    /// Recebe uma mensagem bloqueando a thread atual até sua chegada.
    pub fn recv(&self) -> Result<T, IpcError> {
        self.inner.recv().map_err(|_| IpcError::Disconnected)
    }

    /// Tenta receber uma mensagem sem bloquear.
    pub fn try_recv(&self) -> Result<T, IpcError> {
        match self.inner.try_recv() {
            Ok(msg) => Ok(msg),
            Err(TryRecvError::Empty) => Err(IpcError::Empty),
            Err(TryRecvError::Disconnected) => Err(IpcError::Disconnected),
        }
    }

    /// Recebe uma mensagem aguardando até a expiração do timeout especificado.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, IpcError> {
        self.inner.recv_timeout(timeout).map_err(|e| match e {
            crossbeam_channel::RecvTimeoutError::Timeout => IpcError::Timeout,
            crossbeam_channel::RecvTimeoutError::Disconnected => IpcError::Disconnected,
        })
    }

    /// Retorna a quantidade de mensagens pendentes.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Verifica se não há mensagens no canal.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
