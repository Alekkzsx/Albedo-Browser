//! Streaming parser com foco em API estável e correção.

use std::time::{Duration, Instant};

use super::integrated_parser::ParseResult;
use super::metrics::{MetricsCollector, ParserMetrics};
use super::tree_builder::build_document_with_errors_and_options;
use super::{HtmlDocument, ParserOptions, TreeBuildOutput};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingState {
    Ready,
    Parsing,
    Paused,
    Ended,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkResult {
    Ok,
    NeedsMoreData,
    EofReached,
    Paused,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ParserSnapshot {
    pub state: StreamingState,
    pub position: usize,
    pub line: usize,
    pub column: usize,
    pub pending_tokens: usize,
    pub open_elements_depth: usize,
    pub buffer: String,
    pub chunks_processed: usize,
    pub avg_chunk_latency: Duration,
    pub options: ParserOptions,
}

pub struct StreamingHtmlParser {
    state: StreamingState,
    buffer: String,
    options: ParserOptions,
    metrics: MetricsCollector,
    last_feed_time: Option<Instant>,
    avg_chunk_latency: Duration,
    chunks_processed: usize,
    line: usize,
    column: usize,
}

impl StreamingHtmlParser {
    pub fn new() -> Self {
        Self::with_options(ParserOptions::default())
    }

    pub fn with_options(options: ParserOptions) -> Self {
        Self {
            state: StreamingState::Ready,
            buffer: String::new(),
            options,
            metrics: MetricsCollector::new(),
            last_feed_time: None,
            avg_chunk_latency: Duration::ZERO,
            chunks_processed: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn with_metrics() -> Self {
        Self::new()
    }

    pub fn state(&self) -> StreamingState {
        self.state
    }

    pub fn feed(&mut self, chunk: &str) -> ChunkResult {
        if self.state == StreamingState::Ended {
            return ChunkResult::Error("Parser already ended".to_string());
        }

        if self.state == StreamingState::Paused {
            self.resume();
        }

        self.last_feed_time = Some(Instant::now());
        self.state = StreamingState::Parsing;
        self.buffer.push_str(chunk);
        self.advance_position(chunk);

        let result = if chunk.is_empty() {
            ChunkResult::NeedsMoreData
        } else {
            ChunkResult::Ok
        };

        if let Some(feed_time) = self.last_feed_time.take() {
            let latency = feed_time.elapsed();
            self.avg_chunk_latency = Duration::from_secs_f64(
                (self.avg_chunk_latency.as_secs_f64() * self.chunks_processed as f64
                    + latency.as_secs_f64())
                    / (self.chunks_processed + 1) as f64,
            );
            self.chunks_processed += 1;
        }

        self.state = StreamingState::Ready;
        result
    }

    pub fn end(&mut self) -> HtmlDocument {
        self.end_with_output().document
    }

    pub fn end_with_parse_result(&mut self) -> ParseResult {
        ParseResult::from_tree_build_output(self.end_with_output())
    }

    pub fn end_with_output(&mut self) -> TreeBuildOutput {
        self.state = StreamingState::Ended;

        self.metrics.start_tree_building();
        let output = build_document_with_errors_and_options(&self.buffer, &self.options);
        let node_count = count_nodes(&output.document);
        self.metrics.end_tree_building(node_count);

        output
    }

    pub fn pause(&mut self) {
        self.state = StreamingState::Paused;
    }

    pub fn resume(&mut self) {
        if self.state == StreamingState::Paused {
            self.state = StreamingState::Ready;
        }
    }

    pub fn snapshot(&self) -> ParserSnapshot {
        ParserSnapshot {
            state: self.state,
            position: self.buffer.len(),
            line: self.line,
            column: self.column,
            pending_tokens: 0,
            open_elements_depth: 0,
            buffer: self.buffer.clone(),
            chunks_processed: self.chunks_processed,
            avg_chunk_latency: self.avg_chunk_latency,
            options: self.options.clone(),
        }
    }

    pub fn restore(&mut self, snapshot: ParserSnapshot) {
        self.state = snapshot.state;
        self.buffer = snapshot.buffer;
        self.line = snapshot.line;
        self.column = snapshot.column;
        self.chunks_processed = snapshot.chunks_processed;
        self.avg_chunk_latency = snapshot.avg_chunk_latency;
        self.options = snapshot.options;
        self.last_feed_time = None;
    }

    pub fn avg_chunk_latency(&self) -> Duration {
        self.avg_chunk_latency
    }

    pub fn chunks_processed(&self) -> usize {
        self.chunks_processed
    }

    pub fn finish_with_metrics(self, input_bytes: usize) -> ParserMetrics {
        self.metrics.finish(input_bytes)
    }

    fn advance_position(&mut self, chunk: &str) {
        for ch in chunk.chars() {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
    }
}

impl Default for StreamingHtmlParser {
    fn default() -> Self {
        Self::new()
    }
}

fn count_nodes(document: &HtmlDocument) -> usize {
    fn walk(node: &super::HtmlNode) -> usize {
        match node {
            super::HtmlNode::Element(el) => 1 + el.children.iter().map(walk).sum::<usize>(),
            super::HtmlNode::Text(_) | super::HtmlNode::Comment(_) => 1,
        }
    }

    document.children.iter().map(walk).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_parser_new() {
        let parser = StreamingHtmlParser::new();
        assert_eq!(parser.state(), StreamingState::Ready);
    }

    #[test]
    fn test_streaming_parser_feed_simple() {
        let mut parser = StreamingHtmlParser::new();
        let result = parser.feed("<html><body>Hello</body></html>");
        assert_eq!(result, ChunkResult::Ok);
    }

    #[test]
    fn test_streaming_parser_multiple_chunks() {
        let mut parser = StreamingHtmlParser::new();
        parser.feed("<html>");
        parser.feed("<head><title>Test</title></head>");
        parser.feed("<body>Hello World</body>");
        parser.feed("</html>");

        assert_eq!(parser.chunks_processed(), 4);
    }

    #[test]
    fn test_streaming_parser_pause_resume() {
        let mut parser = StreamingHtmlParser::new();
        parser.feed("<html>");
        parser.pause();
        assert_eq!(parser.state(), StreamingState::Paused);
        parser.feed("<body>");
        assert_eq!(parser.state(), StreamingState::Ready);
    }

    #[test]
    fn test_streaming_parser_snapshot() {
        let parser = StreamingHtmlParser::new();
        let snapshot = parser.snapshot();
        assert_eq!(snapshot.state, StreamingState::Ready);
        assert_eq!(snapshot.line, 1);
        assert_eq!(snapshot.column, 1);
    }

    #[test]
    fn test_streaming_end() {
        let mut parser = StreamingHtmlParser::new();
        parser.feed("<html><body>Test</body></html>");
        let doc = parser.end();

        assert_eq!(parser.state(), StreamingState::Ended);
        assert!(!doc.children.is_empty());
    }

    #[test]
    fn test_streaming_parser_restore_rewinds_buffer() {
        let mut parser = StreamingHtmlParser::new();
        parser.feed("<div>a");
        let snapshot = parser.snapshot();
        parser.feed("</div>");

        parser.restore(snapshot);
        parser.feed("b</div>");

        let document = parser.end();
        let tree = format!("{:?}", document.children);
        assert!(tree.contains("b"));
        assert!(!tree.contains("a"));
    }

    #[test]
    fn test_streaming_end_with_parse_result_matches_batch_shape() {
        let mut parser = StreamingHtmlParser::new();
        parser.feed("<link rel=\"stylesheet\" href=\"app.css\"><div>ok</div>");

        let result = parser.end_with_parse_result();

        assert!(result.errors.is_empty());
        assert_eq!(result.stats.total_preloads, 1);
        assert!(!result.document.children.is_empty());
    }
}
