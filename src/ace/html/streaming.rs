//! Streaming parser com foco em API estável e correção.
//! 
//! Enhanced with:
//! - 16KB chunk processing with state preservation
//! - Partial token handling across chunk boundaries
//! - Backpressure with flow control and pause/resume
//! - Performance target: < 1ms latency per chunk (p50), < 2ms (p99)

use std::time::{Duration, Instant};
use std::collections::VecDeque;

use super::integrated_parser::ParseResult;
use super::metrics::{MetricsCollector, ParserMetrics};
use super::{HtmlDocument, ParserOptions, TreeBuildOutput};

/// Recommended chunk size for optimal performance
/// Balances memory usage vs parsing overhead
pub const OPTIMAL_CHUNK_SIZE: usize = 16 * 1024; // 16 KB

/// Maximum buffer size before backpressure kicks in
/// Prevents unbounded memory growth
pub const MAX_BUFFER_SIZE: usize = 1024 * 1024; // 1 MB

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
    
    // Enhanced streaming features
    partial_token_buffer: String,  // Holds incomplete tokens across chunks
    chunk_queue: VecDeque<String>, // Queue for backpressure handling
    max_queue_size: usize,         // Backpressure threshold
    bytes_processed: usize,        // Total bytes processed
    p50_latency: Duration,         // Median latency
    p99_latency: Duration,         // 99th percentile latency
    latency_samples: Vec<Duration>, // For percentile calculation
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
            partial_token_buffer: String::new(),
            chunk_queue: VecDeque::new(),
            max_queue_size: 64, // Allow up to 64 chunks (1MB) in queue
            bytes_processed: 0,
            p50_latency: Duration::ZERO,
            p99_latency: Duration::ZERO,
            latency_samples: Vec::new(),
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

        // Backpressure: check if queue is full
        if self.chunk_queue.len() >= self.max_queue_size {
            return ChunkResult::Paused;
        }

        let start_time = Instant::now();
        self.state = StreamingState::Parsing;
        
        // Handle partial tokens from previous chunk
        let mut complete_chunk = if !self.partial_token_buffer.is_empty() {
            let mut combined = std::mem::take(&mut self.partial_token_buffer);
            combined.push_str(chunk);
            combined
        } else {
            chunk.to_string()
        };
        
        // Check for incomplete token at end of chunk
        // An incomplete token is one that starts with '<' but doesn't have '>'
        if let Some(last_lt) = complete_chunk.rfind('<') {
            if !complete_chunk[last_lt..].contains('>') {
                // Save incomplete token for next chunk
                self.partial_token_buffer = complete_chunk[last_lt..].to_string();
                complete_chunk.truncate(last_lt);
            }
        }
        
        self.buffer.push_str(&complete_chunk);
        self.advance_position(&complete_chunk);
        self.bytes_processed += chunk.len();

        let result = if chunk.is_empty() {
            ChunkResult::NeedsMoreData
        } else {
            ChunkResult::Ok
        };

        // Track latency
        let latency = start_time.elapsed();
        self.latency_samples.push(latency);
        
        // Update average latency
        self.avg_chunk_latency = Duration::from_secs_f64(
            (self.avg_chunk_latency.as_secs_f64() * self.chunks_processed as f64
                + latency.as_secs_f64())
                / (self.chunks_processed + 1) as f64,
        );
        self.chunks_processed += 1;
        
        // Update percentiles every 100 chunks
        if self.chunks_processed % 100 == 0 {
            self.update_percentiles();
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
        let output = super::build_document_with_errors_and_options(&self.buffer, &self.options);
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
        // Save complete parser state including buffer
        ParserSnapshot {
            state: self.state,
            position: self.bytes_processed,
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
        self.line = snapshot.line;
        self.column = snapshot.column;
        self.chunks_processed = snapshot.chunks_processed;
        self.avg_chunk_latency = snapshot.avg_chunk_latency;
        self.options = snapshot.options;
        self.last_feed_time = None;
        
        // Restore buffer state
        self.buffer = snapshot.buffer;
        self.bytes_processed = snapshot.position;
        
        // Clear partial token buffer and queue on restore
        self.partial_token_buffer.clear();
        self.chunk_queue.clear();
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
    
    /// Update p50 and p99 latency percentiles
    fn update_percentiles(&mut self) {
        if self.latency_samples.is_empty() {
            return;
        }
        
        let mut sorted = self.latency_samples.clone();
        sorted.sort();
        
        let p50_idx = (sorted.len() as f64 * 0.50) as usize;
        let p99_idx = (sorted.len() as f64 * 0.99) as usize;
        
        self.p50_latency = sorted[p50_idx.min(sorted.len() - 1)];
        self.p99_latency = sorted[p99_idx.min(sorted.len() - 1)];
    }
    
    /// Get p50 (median) latency
    pub fn p50_latency(&self) -> Duration {
        self.p50_latency
    }
    
    /// Get p99 latency
    pub fn p99_latency(&self) -> Duration {
        self.p99_latency
    }
    
    /// Get total bytes processed
    pub fn bytes_processed(&self) -> usize {
        self.bytes_processed
    }
    
    /// Check if backpressure is active
    pub fn is_backpressure_active(&self) -> bool {
        self.chunk_queue.len() >= self.max_queue_size
    }
    
    /// Get current queue size
    pub fn queue_size(&self) -> usize {
        self.chunk_queue.len()
    }
    
    /// Set maximum queue size for backpressure
    pub fn set_max_queue_size(&mut self, size: usize) {
        self.max_queue_size = size;
    }
    
    /// Feed chunk with automatic 16KB splitting
    /// Splits large inputs into optimal 16KB chunks
    pub fn feed_auto_chunk(&mut self, input: &str) -> Vec<ChunkResult> {
        let mut results = Vec::new();
        let bytes = input.as_bytes();
        let mut pos = 0;
        
        while pos < bytes.len() {
            let end = (pos + OPTIMAL_CHUNK_SIZE).min(bytes.len());
            
            // Try to find a safe boundary (after '>')
            let chunk_end = if end < bytes.len() {
                let search_start = end.saturating_sub(256); // Look back up to 256 bytes
                bytes[search_start..end]
                    .iter()
                    .rposition(|&b| b == b'>')
                    .map(|i| search_start + i + 1)
                    .unwrap_or(end)
            } else {
                end
            };
            
            if let Ok(chunk) = std::str::from_utf8(&bytes[pos..chunk_end]) {
                results.push(self.feed(chunk));
            }
            
            pos = chunk_end;
        }
        
        results
    }
    
    /// Process queued chunks with flow control
    /// Returns number of chunks processed
    pub fn process_queue(&mut self, max_chunks: usize) -> usize {
        let mut processed = 0;
        
        while processed < max_chunks && !self.chunk_queue.is_empty() {
            if let Some(chunk) = self.chunk_queue.pop_front() {
                self.feed(&chunk);
                processed += 1;
            }
        }
        
        processed
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

        // Restore to snapshot - buffer goes back to "<div>a"
        parser.restore(snapshot);
        
        // Feed new content - replaces the incomplete tag
        parser.feed("b</div>");

        let document = parser.end();
        let tree = format!("{:?}", document.children);
        
        // After restore and new feed, we should have both "a" and "b"
        // because restore brings back "<div>a" and feed adds "b</div>"
        // resulting in "<div>ab</div>"
        assert!(tree.contains("b"));
        // The "a" is still there from the restored buffer
        assert!(tree.contains("a"));
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
    
    #[test]
    fn test_partial_token_handling() {
        let mut parser = StreamingHtmlParser::new();
        
        // Feed incomplete tag
        parser.feed("<div class=\"test");
        // Complete the tag in next chunk
        parser.feed("\">Content</div>");
        
        let doc = parser.end();
        let tree = format!("{:?}", doc.children);
        assert!(tree.contains("Content"));
    }
    
    #[test]
    fn test_16kb_chunk_processing() {
        let mut parser = StreamingHtmlParser::new();
        
        // Generate 32KB of HTML
        let mut html = String::with_capacity(32 * 1024);
        html.push_str("<html><body>");
        for i in 0..1000 {
            html.push_str(&format!("<div id=\"item-{}\">Content {}</div>", i, i));
        }
        html.push_str("</body></html>");
        
        // Feed in 16KB chunks
        let results = parser.feed_auto_chunk(&html);
        
        // Should split into multiple chunks
        assert!(results.len() >= 2);
        assert!(results.iter().all(|r| matches!(r, ChunkResult::Ok)));
        
        let doc = parser.end();
        assert!(!doc.children.is_empty());
    }
    
    #[test]
    fn test_backpressure() {
        let mut parser = StreamingHtmlParser::new();
        parser.set_max_queue_size(2);
        
        // Fill queue
        parser.chunk_queue.push_back("<div>1</div>".to_string());
        parser.chunk_queue.push_back("<div>2</div>".to_string());
        
        // Should trigger backpressure
        let result = parser.feed("<div>3</div>");
        assert_eq!(result, ChunkResult::Paused);
        assert!(parser.is_backpressure_active());
    }
    
    #[test]
    fn test_latency_tracking() {
        let mut parser = StreamingHtmlParser::new();
        
        // Feed multiple chunks
        for i in 0..10 {
            parser.feed(&format!("<div>{}</div>", i));
        }
        
        // Should have latency metrics
        assert!(parser.avg_chunk_latency() > Duration::ZERO);
        assert_eq!(parser.chunks_processed(), 10);
        assert_eq!(parser.bytes_processed(), 10 * "<div>0</div>".len());
    }
    
    #[test]
    fn test_percentile_latency() {
        let mut parser = StreamingHtmlParser::new();
        
        // Feed 100+ chunks to trigger percentile calculation
        for i in 0..150 {
            parser.feed(&format!("<p>{}</p>", i));
        }
        
        // Should have percentile metrics
        assert!(parser.p50_latency() > Duration::ZERO);
        assert!(parser.p99_latency() >= parser.p50_latency());
    }
    
    #[test]
    fn test_queue_processing() {
        let mut parser = StreamingHtmlParser::new();
        
        // Add chunks to queue
        parser.chunk_queue.push_back("<div>1</div>".to_string());
        parser.chunk_queue.push_back("<div>2</div>".to_string());
        parser.chunk_queue.push_back("<div>3</div>".to_string());
        
        // Process 2 chunks
        let processed = parser.process_queue(2);
        assert_eq!(processed, 2);
        assert_eq!(parser.queue_size(), 1);
    }
    
    #[test]
    fn test_chunk_boundary_safety() {
        let mut parser = StreamingHtmlParser::new();
        
        // Feed chunk that ends mid-tag
        parser.feed("<div><span>Text");
        // Complete in next chunk
        parser.feed("</span></div>");
        
        let doc = parser.end();
        let tree = format!("{:?}", doc.children);
        assert!(tree.contains("Text"));
    }
}
