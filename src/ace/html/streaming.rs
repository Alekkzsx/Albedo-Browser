use std::time::{Duration, Instant};
use super::types::{ParserOptions, ParseResult, Encoding, DecodedHtml, HtmlDocument, StreamingSnapshot};
use super::encoding::{decode_html_bytes, detect_bom, sniff_meta_charset};
use super::{parse_document_with_errors_and_options};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChunkResult {
    Ok,
    Error(String),
}

pub struct StreamingHtmlParser {
    raw_bytes: Vec<u8>,
    buffer: String,
    decided_encoding: Option<Encoding>,
    options: ParserOptions,
    chunk_latencies: Vec<Duration>,
    last_parse_result: Option<ParseResult>,
}

impl StreamingHtmlParser {
    pub fn new() -> Self {
        Self::with_options(ParserOptions::default())
    }

    pub fn with_options(options: ParserOptions) -> Self {
        Self {
            raw_bytes: Vec::new(),
            buffer: String::new(),
            decided_encoding: options.encoding_hint,
            options,
            chunk_latencies: Vec::new(),
            last_parse_result: None,
        }
    }

    pub fn feed(&mut self, chunk: &str) -> ChunkResult {
        self.feed_bytes(chunk.as_bytes())
    }

    pub fn feed_bytes(&mut self, chunk: &[u8]) -> ChunkResult {
        let start = Instant::now();
        self.raw_bytes.extend_from_slice(chunk);

        if self.decided_encoding.is_none() {
            if let Some((encoding, _)) = detect_bom(&self.raw_bytes) {
                self.decided_encoding = Some(encoding);
            } else if let Some(meta_encoding) = sniff_meta_charset(&self.raw_bytes) {
                self.decided_encoding = Some(meta_encoding);
            } else if let Some(hint) = self.options.encoding_hint {
                self.decided_encoding = Some(hint);
            }
        }

        let decode_hint = self.decided_encoding.or(Some(Encoding::Utf8));
        match decode_html_bytes(&self.raw_bytes, None, decode_hint) {
            Ok(decoded) => {
                self.buffer = decoded.content;
                self.last_parse_result =
                    Some(parse_document_with_errors_and_options(&self.buffer, &self.options));
            }
            Err(err) => {
                return ChunkResult::Error(format!(
                    "failed to decode stream chunk: {}",
                    err.message
                ));
            }
        }

        self.chunk_latencies.push(start.elapsed());
        ChunkResult::Ok
    }

    pub fn end(&mut self) -> HtmlDocument {
        self.end_with_parse_result().document
    }

    pub fn end_with_parse_result(&mut self) -> ParseResult {
        if !self.raw_bytes.is_empty() {
            if let Ok(decoded) = decode_html_bytes(
                &self.raw_bytes,
                None,
                self.decided_encoding.or(self.options.encoding_hint),
            ) {
                self.decided_encoding = Some(decoded.encoding);
                self.buffer = decoded.content;
                self.last_parse_result =
                    Some(parse_document_with_errors_and_options(&self.buffer, &self.options));
            }
        }

        self.last_parse_result
            .take()
            .unwrap_or_else(|| parse_document_with_errors_and_options(&self.buffer, &self.options))
    }

    pub fn snapshot(&self) -> StreamingSnapshot {
        StreamingSnapshot {
            raw_bytes: self.raw_bytes.clone(),
            buffer: self.buffer.clone(),
            decided_encoding: self.decided_encoding,
        }
    }

    pub fn restore(&mut self, snapshot: StreamingSnapshot) {
        self.raw_bytes = snapshot.raw_bytes;
        self.buffer = snapshot.buffer;
        self.decided_encoding = snapshot.decided_encoding;
        self.last_parse_result = Some(parse_document_with_errors_and_options(
            &self.buffer,
            &self.options,
        ));
    }

    pub fn p50_latency(&self) -> Duration {
        percentile_duration(&self.chunk_latencies, 50)
    }

    pub fn p99_latency(&self) -> Duration {
        percentile_duration(&self.chunk_latencies, 99)
    }

    pub fn decided_encoding(&self) -> Option<Encoding> {
        self.decided_encoding
    }

    pub fn bytes_seen(&self) -> usize {
        self.raw_bytes.len()
    }
}

pub fn percentile_duration(samples: &[Duration], percentile: usize) -> Duration {
    if samples.is_empty() {
        return Duration::ZERO;
    }

    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() - 1) * percentile) / 100;
    sorted[index]
}
