//! # Cursor de Bytes Binários para Redes e IPC
//!
//! Leitura sequencial de fatias de bytes (`&[u8]`) com suporte a inteiros Big-Endian.

/// Cursor de leitura sequencial sobre buffers de bytes binários.
#[derive(Debug, Clone)]
pub struct ByteCursor<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> ByteCursor<'a> {
    /// Cria um novo cursor de bytes no início da fatia.
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    /// Retorna o byte atual sem avançar a posição (*peek*).
    #[inline]
    pub fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    /// Avança e consome um byte.
    #[inline]
    pub fn advance(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    /// Lê e consome uma fatia de `count` bytes. Retorna `None` se houver menos bytes disponíveis.
    pub fn read_bytes(&mut self, count: usize) -> Option<&'a [u8]> {
        if self.pos + count <= self.input.len() {
            let slice = &self.input[self.pos..self.pos + count];
            self.pos += count;
            Some(slice)
        } else {
            None
        }
    }

    /// Lê um inteiro `u16` em formato Big-Endian.
    pub fn read_u16_be(&mut self) -> Option<u16> {
        let bytes = self.read_bytes(2)?;
        Some(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    /// Lê um inteiro `u32` em formato Big-Endian.
    pub fn read_u32_be(&mut self) -> Option<u32> {
        let bytes = self.read_bytes(4)?;
        Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Retorna `true` se o cursor atingiu o fim do buffer.
    #[inline]
    pub fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// Retorna a fatia restante de bytes não consumidos.
    #[inline]
    pub fn remaining(&self) -> &'a [u8] {
        &self.input[self.pos..]
    }

    /// Retorna a quantidade de bytes restantes.
    #[inline]
    pub fn remaining_len(&self) -> usize {
        self.input.len().saturating_sub(self.pos)
    }

    /// Retorna a posição atual do cursor em bytes.
    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
    }
}
