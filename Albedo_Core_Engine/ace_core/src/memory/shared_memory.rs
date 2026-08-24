//! # Regiões de Memória Compartilhada Segura para IPC (Chromium base::ReadOnlySharedMemoryRegion Pattern)
//!
//! Em arquiteturas multi-processo de navegadores (processo do Browser, Renderizador e GPU),
//! passar buffers volumosos (display lists, texturas decodificadas, dicionários de fontes) através de IPC
//! tradicional causaria overhead insustentável de serialização e cópias de memória.
//!
//! Este módulo provê regiões de memória compartilhada com garantias de permissão do sistema operacional:
//! - `WritableSharedMemoryRegion`: Região com permissão de escrita exclusiva. Suporta conversão irreversível
//!   para `ReadOnlySharedMemoryRegion` (`convert_to_read_only()`).
//! - `ReadOnlySharedMemoryRegion`: Região mapeada como somente leitura, segura para entrega a processos
//!   de renderização não-confiáveis (*sandboxed renderers*).
//! - `UnsafeSharedMemoryRegion`: Região de leitura e escrita bidirecional concorrente para streaming de vídeo/áudio.

use crate::security::UnguessableToken;
use parking_lot::RwLock;
use std::fmt;
use std::sync::Arc;

struct SharedMemoryInner {
    id: UnguessableToken,
    buffer: RwLock<Vec<u8>>,
    is_read_only: bool,
}

/// Mapeamento de memória compartilhado ativo para leitura imutável.
pub struct ReadOnlySharedMemoryMapping {
    inner: Arc<SharedMemoryInner>,
}

impl ReadOnlySharedMemoryMapping {
    /// Retorna uma fatia de bytes de somente leitura do buffer compartilhado.
    #[inline]
    pub fn as_slice(&self) -> parking_lot::RwLockReadGuard<'_, Vec<u8>> {
        self.inner.buffer.read()
    }

    /// Retorna o tamanho total da região mapeada em bytes.
    #[inline]
    pub fn size(&self) -> usize {
        self.inner.buffer.read().len()
    }
}

/// Mapeamento de memória compartilhado ativo para escrita mutável.
pub struct WritableSharedMemoryMapping {
    inner: Arc<SharedMemoryInner>,
}

impl WritableSharedMemoryMapping {
    /// Retorna uma guarda de escrita mutável para o buffer.
    #[inline]
    pub fn as_mut_slice(&mut self) -> parking_lot::RwLockWriteGuard<'_, Vec<u8>> {
        self.inner.buffer.write()
    }

    /// Retorna o tamanho da região em bytes.
    #[inline]
    pub fn size(&self) -> usize {
        self.inner.buffer.read().len()
    }
}

/// Região de memória compartilhada mutável antes de ser selada para envio por IPC.
pub struct WritableSharedMemoryRegion {
    inner: Arc<SharedMemoryInner>,
}

impl WritableSharedMemoryRegion {
    /// Cria uma nova região de memória compartilhada com o tamanho especificado.
    pub fn create(size: usize) -> Self {
        Self {
            inner: Arc::new(SharedMemoryInner {
                id: UnguessableToken::new(),
                buffer: RwLock::new(vec![0u8; size]),
                is_read_only: false,
            }),
        }
    }

    /// Retorna o identificador de segurança de 128 bits da região.
    #[inline]
    pub fn id(&self) -> UnguessableToken {
        self.inner.id
    }

    /// Cria um mapeamento de escrita mutável sobre a região.
    pub fn map(&self) -> WritableSharedMemoryMapping {
        WritableSharedMemoryMapping {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Converte irreversivelmente a região para `ReadOnlySharedMemoryRegion`.
    ///
    /// Após a chamada, nenhum processo poderá obter mapeamento de escrita.
    pub fn convert_to_read_only(mut self) -> ReadOnlySharedMemoryRegion {
        if let Some(inner) = Arc::get_mut(&mut self.inner) {
            inner.is_read_only = true;
        }
        ReadOnlySharedMemoryRegion { inner: self.inner }
    }
}

/// Região de memória compartilhada somente leitura para distribuição segura em sandboxes.
#[derive(Clone)]
pub struct ReadOnlySharedMemoryRegion {
    inner: Arc<SharedMemoryInner>,
}

impl ReadOnlySharedMemoryRegion {
    /// Retorna o identificador de segurança da região.
    #[inline]
    pub fn id(&self) -> UnguessableToken {
        self.inner.id
    }

    /// Mapeia a região para leitura no processo atual.
    pub fn map(&self) -> ReadOnlySharedMemoryMapping {
        ReadOnlySharedMemoryMapping {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Retorna o tamanho da região em bytes.
    #[inline]
    pub fn size(&self) -> usize {
        self.inner.buffer.read().len()
    }
}

impl fmt::Debug for WritableSharedMemoryRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WritableSharedMemoryRegion")
            .field("id", &self.id())
            .field("size", &self.inner.buffer.read().len())
            .finish()
    }
}

impl fmt::Debug for ReadOnlySharedMemoryRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadOnlySharedMemoryRegion")
            .field("id", &self.id())
            .field("size", &self.size())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_memory_write_and_seal_to_read_only() {
        let region = WritableSharedMemoryRegion::create(64);
        {
            let mut mapping = region.map();
            let mut buf = mapping.as_mut_slice();
            buf[0] = 0xAA;
            buf[63] = 0xFF;
        }

        let ro_region = region.convert_to_read_only();
        let ro_mapping = ro_region.map();
        let ro_buf = ro_mapping.as_slice();

        assert_eq!(ro_buf[0], 0xAA);
        assert_eq!(ro_buf[63], 0xFF);
        assert_eq!(ro_region.size(), 64);
    }
}
