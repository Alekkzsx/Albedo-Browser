//! # Background HTML Parser (Blink HTMLParserThread Pattern)
//!
//! Desacoplamento assíncrono do Tokenizer em background thread para eliminação de jank
//! no Event Loop e aceleração de Time-to-Interactive (TTI).

use crate::parser::chunk::{ParsedChunk, DEFAULT_PARSER_CHUNK_SIZE};
use crate::preload_scanner::{PreloadKind, PreloadRequest};
use crate::tokenizer::state::TokenizerState;
use crate::tokenizer::token::{CompactHTMLToken, Token};
use crate::tokenizer::{HTMLTokenizer, TokenSink, TokenizerAction};
use crate::tree::Document;
use crate::tree_builder::HTMLTreeBuilder;
use ace_core::text::SegmentedString;
use crossbeam::channel::{bounded, Receiver, Sender};
use std::thread::JoinHandle;

/// Receptor de tokens interno que agrupa tokens em `ParsedChunk`s e detecta sub-recursos.
struct BackgroundChunkSink {
    sender: Sender<ParsedChunk>,
    current_chunk: ParsedChunk,
    chunk_size: usize,
}

impl BackgroundChunkSink {
    fn new(sender: Sender<ParsedChunk>, chunk_size: usize) -> Self {
        Self {
            sender,
            current_chunk: ParsedChunk::new(TokenizerState::Data),
            chunk_size,
        }
    }

    /// Inspeciona a tag de abertura para extração especulativa de preloads em $O(1)$.
    fn inspect_preload(
        tag_name: &ace_core::intern::Atom,
        attributes: &ace_core::collections::InlineVec<crate::node::element::Attribute, 4>,
    ) -> Option<PreloadRequest> {
        let tag = tag_name.as_str();
        if tag.eq_ignore_ascii_case("link") {
            let mut href = None;
            let mut rel = None;
            let mut as_kind = None;
            for attr in attributes.as_slice() {
                if attr.name.eq_ignore_ascii_case("href") {
                    href = Some(attr.value.to_string());
                } else if attr.name.eq_ignore_ascii_case("rel") {
                    rel = Some(attr.value.to_string());
                } else if attr.name.eq_ignore_ascii_case("as") {
                    as_kind = Some(attr.value.to_string());
                }
            }
            if let Some(h) = href {
                let is_stylesheet = rel.as_deref().is_some_and(|r| r.eq_ignore_ascii_case("stylesheet"));
                let is_preload = rel.as_deref().is_some_and(|r| r.eq_ignore_ascii_case("preload"));
                if is_stylesheet || (is_preload && as_kind.as_deref() == Some("style")) {
                    return Some(PreloadRequest {
                        url: SmolStr::new(h),
                        kind: PreloadKind::Stylesheet,
                        as_type: as_kind.map(SmolStr::new),
                        media: None,
                    });
                } else if is_preload && as_kind.as_deref() == Some("script") {
                    return Some(PreloadRequest {
                        url: SmolStr::new(h),
                        kind: PreloadKind::Script,
                        as_type: Some(SmolStr::new("script")),
                        media: None,
                    });
                }
            }
        } else if tag.eq_ignore_ascii_case("script") {
            for attr in attributes.as_slice() {
                if attr.name.eq_ignore_ascii_case("src") {
                    return Some(PreloadRequest {
                        url: attr.value.clone(),
                        kind: PreloadKind::Script,
                        as_type: None,
                        media: None,
                    });
                }
            }
        } else if tag.eq_ignore_ascii_case("img") {
            for attr in attributes.as_slice() {
                if attr.name.eq_ignore_ascii_case("src") {
                    return Some(PreloadRequest {
                        url: attr.value.clone(),
                        kind: PreloadKind::Image,
                        as_type: None,
                        media: None,
                    });
                }
            }
        }
        None
    }

    fn flush_chunk(&mut self, is_last: bool, ending_state: TokenizerState) {
        if !self.current_chunk.tokens.is_empty() || is_last {
            self.current_chunk.is_last = is_last;
            self.current_chunk.ending_state = ending_state;
            let chunk_to_send = std::mem::replace(
                &mut self.current_chunk,
                ParsedChunk::new(ending_state),
            );
            let _ = self.sender.send(chunk_to_send);
        }
    }
}

impl TokenSink for BackgroundChunkSink {
    fn process_token(&mut self, token: Token) -> TokenizerAction {
        if let Some(compact) = CompactHTMLToken::from_token(token) {
            if let CompactHTMLToken::StartTag {
                ref name,
                ref attributes,
                ..
            } = compact
            {
                if let Some(preload) = Self::inspect_preload(name, attributes) {
                    self.current_chunk.preloads.push(preload);
                }
            }

            let is_eof = compact.is_eof();
            self.current_chunk.tokens.push(compact);

            if self.current_chunk.tokens.len() >= self.chunk_size || is_eof {
                self.flush_chunk(is_eof, TokenizerState::Data);
            }
        }
        TokenizerAction::Continue
    }
}

/// Handle de controle da tokenização assíncrona na thread principal.
pub struct BackgroundParserHandle {
    receiver: Receiver<ParsedChunk>,
    join_handle: Option<JoinHandle<()>>,
    discovered_preloads: Vec<PreloadRequest>,
    completed: bool,
}

impl BackgroundParserHandle {
    /// Drena todos os chunks de tokens atualmente disponíveis no canal sem bloquear a thread principal.
    /// Retorna o número de tokens processados.
    pub fn pump(&mut self, builder: &mut HTMLTreeBuilder) -> usize {
        let mut tokens_processed = 0;
        while let Ok(chunk) = self.receiver.try_recv() {
            if !chunk.preloads.is_empty() {
                self.discovered_preloads.extend(chunk.preloads);
            }
            for compact in chunk.tokens {
                tokens_processed += 1;
                let is_eof = compact.is_eof();
                builder.process_token(compact.to_token());
                if is_eof {
                    self.completed = true;
                }
            }
            if chunk.is_last {
                self.completed = true;
                break;
            }
        }
        tokens_processed
    }

    /// Aguarda a finalização completa da thread de background, drena os chunks restantes e constrói o `Document`.
    pub fn finish(mut self, mut builder: HTMLTreeBuilder) -> Document {
        while !self.completed {
            match self.receiver.recv() {
                Ok(chunk) => {
                    if !chunk.preloads.is_empty() {
                        self.discovered_preloads.extend(chunk.preloads);
                    }
                    for compact in chunk.tokens {
                        let is_eof = compact.is_eof();
                        builder.process_token(compact.to_token());
                        if is_eof {
                            self.completed = true;
                        }
                    }
                    if chunk.is_last {
                        self.completed = true;
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }

        builder.finish()
    }

    /// Retorna uma referência aos sub-recursos críticos descobertos em background.
    #[inline]
    pub fn discovered_preloads(&self) -> &[PreloadRequest] {
        &self.discovered_preloads
    }

    /// Retorna `true` se o parsing em background já tiver completado.
    #[inline]
    pub fn is_completed(&self) -> bool {
        self.completed
    }
}

/// O coordenador do Background HTML Parser.
pub struct BackgroundHTMLParser;

impl BackgroundHTMLParser {
    /// Inicia o parsing de uma string HTML em uma thread de background dedicada.
    pub fn spawn_parse(html: String) -> (BackgroundParserHandle, HTMLTreeBuilder) {
        Self::spawn_parse_with_chunk_size(html, DEFAULT_PARSER_CHUNK_SIZE)
    }

    /// Inicia o parsing em background com tamanho de chunk customizado.
    pub fn spawn_parse_with_chunk_size(
        html: String,
        chunk_size: usize,
    ) -> (BackgroundParserHandle, HTMLTreeBuilder) {
        let (sender, receiver) = bounded::<ParsedChunk>(32);

        let join_handle = std::thread::Builder::new()
            .name("ace_html_background_parser".to_string())
            .spawn(move || {
                let mut tokenizer = HTMLTokenizer::new();
                let mut input = SegmentedString::from(html.as_str());
                let mut sink = BackgroundChunkSink::new(sender, chunk_size);
                tokenizer.tokenize(&mut input, &mut sink);
                sink.flush_chunk(true, tokenizer.state);
            })
            .expect("Falha ao criar thread de Background HTML Parser");

        let handle = BackgroundParserHandle {
            receiver,
            join_handle: Some(join_handle),
            discovered_preloads: Vec::new(),
            completed: false,
        };

        let builder = HTMLTreeBuilder::new(None);
        (handle, builder)
    }

    /// Executa o parsing multithread completo de forma transparente de ponta a ponta.
    pub fn parse_threaded(html: impl Into<String>) -> Document {
        let (handle, builder) = Self::spawn_parse(html.into());
        handle.finish(builder)
    }
}
