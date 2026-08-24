//! # Buffer de Chunks em Cadeia para Streams Web (WHATWG Streams Standard)
//!
//! Estrutura de armazenamento de fluxos de bytes baseada em blocos contíguos fixos (4KB),
//! evitando alocações e cópias em massa durante downloads de rede e decodificação de mídia.

use std::collections::VecDeque;

/// Buffer de chunks em blocos contíguos parametrizado pelo tamanho do bloco.
#[derive(Debug, Clone)]
pub struct ChunkBuffer<const CHUNK_SIZE: usize = 4096> {
    chunks: VecDeque<Vec<u8>>,
    read_pos: usize,
    total_len: usize,
}

impl<const CHUNK_SIZE: usize> ChunkBuffer<CHUNK_SIZE> {
    /// Cria um novo buffer de chunks vazio.
    pub fn new() -> Self {
        Self {
            chunks: VecDeque::new(),
            read_pos: 0,
            total_len: 0,
        }
    }

    /// Escreve uma fatia de bytes no buffer, alocando novos chunks de tamanho fixo conforme necessário.
    pub fn write_bytes(&mut self, data: &[u8]) {
        let mut offset = 0;
        let len = data.len();
        self.total_len += len;

        while offset < len {
            // Se o último chunk tiver espaço livre, escreve nele
            if let Some(last) = self.chunks.back_mut() {
                if last.len() < CHUNK_SIZE {
                    let available = CHUNK_SIZE - last.len();
                    let to_write = (len - offset).min(available);
                    last.extend_from_slice(&data[offset..offset + to_write]);
                    offset += to_write;
                    continue;
                }
            }

            // Caso contrário, aloca um novo chunk com capacidade pré-definida
            let to_write = (len - offset).min(CHUNK_SIZE);
            let mut new_chunk = Vec::with_capacity(CHUNK_SIZE);
            new_chunk.extend_from_slice(&data[offset..offset + to_write]);
            self.chunks.push_back(new_chunk);
            offset += to_write;
        }
    }

    /// Lê até `dest.len()` bytes do buffer e os descarta da memória.
    /// Retorna a quantidade de bytes efetivamente lidos.
    pub fn read_bytes(&mut self, dest: &mut [u8]) -> usize {
        let mut bytes_read = 0;
        let requested = dest.len();

        while bytes_read < requested && !self.chunks.is_empty() {
            let chunk = self.chunks.front_mut().unwrap();
            let available = chunk.len() - self.read_pos;
            let to_read = (requested - bytes_read).min(available);

            dest[bytes_read..bytes_read + to_read]
                .copy_from_slice(&chunk[self.read_pos..self.read_pos + to_read]);

            self.read_pos += to_read;
            bytes_read += to_read;
            self.total_len -= to_read;

            if self.read_pos >= chunk.len() {
                self.chunks.pop_front();
                self.read_pos = 0;
            }
        }

        bytes_read
    }

    /// Pré-visualiza até `n` bytes do início do buffer sem avançar o cursor de leitura.
    pub fn peek(&self, n: usize) -> Vec<u8> {
        let limit = n.min(self.total_len);
        let mut out = Vec::with_capacity(limit);
        let mut read = 0;
        let mut first_offset = self.read_pos;

        for chunk in &self.chunks {
            if read >= limit {
                break;
            }
            let available = chunk.len() - first_offset;
            let to_copy = (limit - read).min(available);
            out.extend_from_slice(&chunk[first_offset..first_offset + to_copy]);
            read += to_copy;
            first_offset = 0;
        }

        out
    }

    /// Retorna o total de bytes não lidos disponíveis no buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.total_len
    }

    /// Retorna `true` se o buffer estiver completamente vazio.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.total_len == 0
    }

    /// Esvazia e descarta todos os chunks alocados.
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.read_pos = 0;
        self.total_len = 0;
    }
}

impl<const CHUNK_SIZE: usize> Default for ChunkBuffer<CHUNK_SIZE> {
    fn default() -> Self {
        Self::new()
    }
}
