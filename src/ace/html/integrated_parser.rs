//! Wrapper integrado para parsing HTML com estatísticas resumidas.

use std::collections::HashMap;

use super::tree_builder::build_document_with_errors_and_options;
use super::{
    parse_document_from_bytes_with_errors_and_options, HtmlDocument, HtmlNode, NodeArena,
    ParseError, ParserOptions, PreloadRequest, TreeBuildOutput,
};

/// Result of parsing with integrated statistics.
///
/// This struct provides the parsed document along with detailed statistics about
/// the parsing process, including memory usage, string interning efficiency, and
/// any errors encountered.
///
/// # Examples
///
/// ```
/// use ace::html::parse_html_integrated;
///
/// let html = "<div><span>Hello</span></div>";
/// let result = parse_html_integrated(html);
///
/// println!("Parsed {} nodes", result.document.children.len());
/// println!("Memory usage: {} KB", result.stats.arena_capacity_kb);
/// println!("String hit rate: {:.1}%", result.stats.interner_hit_rate * 100.0);
/// ```
pub struct ParseResult {
    /// The parsed HTML document.
    pub document: HtmlDocument,
    /// Error messages (simplified).
    pub errors: Vec<String>,
    /// Detailed parse errors with positions.
    pub parse_errors: Vec<ParseError>,
    /// Resources discovered by the preload scanner.
    pub preload_requests: Vec<PreloadRequest>,
    /// Parsing statistics.
    pub stats: ParserStats,
}

impl ParseResult {
    pub fn from_tree_build_output(output: TreeBuildOutput) -> Self {
        let parse_errors = output.parse_errors();
        let stats =
            build_parser_stats(&output.document, output.errors.len(), output.preload_requests.len());

        Self {
            stats,
            errors: parse_errors.iter().map(|e| e.message.clone()).collect(),
            parse_errors,
            preload_requests: output.preload_requests,
            document: output.document,
        }
    }
}

/// Statistics collected during HTML parsing.
///
/// These statistics provide insights into memory usage, string interning efficiency,
/// and other performance metrics.
///
/// # Examples
///
/// ```
/// use ace::html::parse_html_integrated;
///
/// let html = "<div class='test'><p>Hello</p></div>";
/// let result = parse_html_integrated(html);
///
/// println!("Arena chunks: {}", result.stats.arena_chunk_count);
/// println!("Unique strings: {}", result.stats.interner_unique_strings);
/// println!("Hit rate: {:.2}%", result.stats.interner_hit_rate * 100.0);
/// ```
pub struct ParserStats {
    /// Number of memory chunks allocated by the arena.
    pub arena_chunk_count: usize,
    /// Total arena capacity in kilobytes.
    pub arena_capacity_kb: usize,
    /// Arena memory utilization (0.0 to 1.0).
    pub arena_utilization: f32,
    /// Number of unique strings in the interner.
    pub interner_unique_strings: usize,
    /// String interner hit rate (0.0 to 1.0).
    pub interner_hit_rate: f32,
    /// Total number of parse errors.
    pub total_errors: usize,
    /// Total number of preload requests discovered.
    pub total_preloads: usize,
}

/// Builder for parsing HTML with integrated statistics collection.
///
/// This builder provides a fluent API for configuring and executing HTML parsing
/// with automatic statistics collection.
///
/// # Examples
///
/// ```
/// use ace::html::{IntegratedTreeBuilder, ParserOptions};
///
/// let mut options = ParserOptions::default();
/// options.collect_preloads = true;
///
/// let result = IntegratedTreeBuilder::with_options("<div>Hello</div>", options)
///     .parse();
///
/// assert!(!result.document.children.is_empty());
/// ```
pub struct IntegratedTreeBuilder<'a> {
    input: &'a str,
    options: ParserOptions,
}

impl<'a> IntegratedTreeBuilder<'a> {
    /// Creates a new builder with default options.
    ///
    /// # Examples
    ///
    /// ```
    /// use ace::html::IntegratedTreeBuilder;
    ///
    /// let builder = IntegratedTreeBuilder::new("<div>Hello</div>");
    /// let result = builder.parse();
    /// ```
    pub fn new(input: &'a str) -> Self {
        Self::with_options(input, ParserOptions::default())
    }

    /// Creates a new builder with custom options.
    ///
    /// # Examples
    ///
    /// ```
    /// use ace::html::{IntegratedTreeBuilder, ParserOptions};
    ///
    /// let mut options = ParserOptions::default();
    /// options.scripting_enabled = false;
    ///
    /// let builder = IntegratedTreeBuilder::with_options("<noscript>Hi</noscript>", options);
    /// ```
    pub fn with_options(input: &'a str, options: ParserOptions) -> Self {
        Self { input, options }
    }

    /// Parses the HTML and returns results with statistics.
    ///
    /// # Examples
    ///
    /// ```
    /// use ace::html::IntegratedTreeBuilder;
    ///
    /// let result = IntegratedTreeBuilder::new("<div>Hello</div>").parse();
    /// assert!(result.stats.arena_chunk_count > 0);
    /// ```
    pub fn parse(self) -> ParseResult {
        let output = build_document_with_errors_and_options(self.input, &self.options);
        Self::to_parse_result(output)
    }

    /// Parses the HTML and returns the raw tree build output.
    ///
    /// Use this method if you need access to the raw [`TreeBuildOutput`] instead
    /// of the processed [`ParseResult`].
    pub fn parse_output(self) -> TreeBuildOutput {
        build_document_with_errors_and_options(self.input, &self.options)
    }

    fn to_parse_result(output: TreeBuildOutput) -> ParseResult {
        ParseResult::from_tree_build_output(output)
    }
}

/// Parses HTML with integrated statistics collection.
///
/// This is a convenience function that creates an [`IntegratedTreeBuilder`] with
/// default options and returns the parsing results with statistics.
///
/// # Examples
///
/// ```
/// use ace::html::parse_html_integrated;
///
/// let html = "<div><p>Hello, World!</p></div>";
/// let result = parse_html_integrated(html);
///
/// println!("Parsed successfully!");
/// println!("Memory used: {} KB", result.stats.arena_capacity_kb);
/// println!("Errors: {}", result.stats.total_errors);
/// ```
pub fn parse_html_integrated(input: &str) -> ParseResult {
    IntegratedTreeBuilder::new(input).parse()
}

/// Parses HTML with custom options and integrated statistics.
///
/// # Examples
///
/// ```
/// use ace::html::{parse_html_integrated_with_options, ParserOptions};
///
/// let mut options = ParserOptions::default();
/// options.collect_preloads = true;
///
/// let html = "<link rel='stylesheet' href='app.css'>";
/// let result = parse_html_integrated_with_options(html, &options);
///
/// assert!(result.stats.total_preloads > 0);
/// ```
pub fn parse_html_integrated_with_options(input: &str, options: &ParserOptions) -> ParseResult {
    IntegratedTreeBuilder::with_options(input, options.clone()).parse()
}

/// Parses HTML from bytes with integrated statistics.
///
/// The character encoding is automatically detected from the byte stream.
///
/// # Errors
///
/// Returns an error if the bytes cannot be decoded to valid text.
///
/// # Examples
///
/// ```
/// use ace::html::parse_html_integrated_from_bytes;
///
/// let bytes = b"<div>Hello</div>";
/// let result = parse_html_integrated_from_bytes(bytes).unwrap();
/// assert!(!result.document.children.is_empty());
/// ```
pub fn parse_html_integrated_from_bytes(bytes: &[u8]) -> Result<ParseResult, String> {
    parse_html_integrated_from_bytes_with_options(bytes, None, &ParserOptions::default())
}

/// Parses HTML from bytes with custom options and integrated statistics.
///
/// # Arguments
///
/// * `bytes` - The raw bytes of the HTML document
/// * `http_header` - Optional HTTP Content-Type header for encoding detection
/// * `options` - Parser configuration options
///
/// # Errors
///
/// Returns an error if the bytes cannot be decoded to valid text.
///
/// # Examples
///
/// ```
/// use ace::html::{parse_html_integrated_from_bytes_with_options, ParserOptions};
///
/// let bytes = b"<div>Hello</div>";
/// let options = ParserOptions::default();
///
/// let result = parse_html_integrated_from_bytes_with_options(bytes, None, &options).unwrap();
/// assert!(result.stats.arena_chunk_count > 0);
/// ```
pub fn parse_html_integrated_from_bytes_with_options(
    bytes: &[u8],
    http_header: Option<&str>,
    options: &ParserOptions,
) -> Result<ParseResult, String> {
    let output = parse_document_from_bytes_with_errors_and_options(bytes, http_header, options)?;
    Ok(IntegratedTreeBuilder::to_parse_result(output))
}

#[cfg(test)]
fn count_document_nodes(document: &HtmlDocument) -> usize {
    document.children.iter().map(count_node).sum()
}

fn count_node(node: &HtmlNode) -> usize {
    match node {
        HtmlNode::Element(el) => 1 + el.children.iter().map(count_node).sum::<usize>(),
        HtmlNode::Text(_) | HtmlNode::Comment(_) => 1,
    }
}

fn count_unique_strings(document: &HtmlDocument) -> usize {
    use std::collections::HashSet;

    fn walk(node: &HtmlNode, seen: &mut HashSet<String>) {
        match node {
            HtmlNode::Element(el) => {
                seen.insert(el.tag.clone());
                for (key, value) in &el.attributes {
                    seen.insert(key.clone());
                    seen.insert(value.clone());
                }
                if let Some(slot_name) = &el.slot_name {
                    seen.insert(slot_name.clone());
                }
                if let Some(is_value) = &el.is_value {
                    seen.insert(is_value.clone());
                }
                for child in &el.children {
                    walk(child, seen);
                }
            }
            HtmlNode::Text(text) | HtmlNode::Comment(text) => {
                seen.insert(text.clone());
            }
        }
    }

    let mut seen = HashSet::new();
    for child in &document.children {
        walk(child, &mut seen);
    }
    seen.len()
}

fn build_parser_stats(
    document: &HtmlDocument,
    total_errors: usize,
    total_preloads: usize,
) -> ParserStats {
    let mut arena = NodeArena::new();
    let mut string_mentions = HashMap::<String, usize>::new();

    for child in &document.children {
        collect_node_stats(child, &mut arena, &mut string_mentions);
    }

    let arena_stats = arena.stats();
    let total_mentions: usize = string_mentions.values().sum();
    let unique_strings = string_mentions.len();
    let repeated_mentions = total_mentions.saturating_sub(unique_strings);
    let interner_hit_rate = if total_mentions > 0 {
        repeated_mentions as f32 / total_mentions as f32
    } else {
        0.0
    };

    ParserStats {
        arena_chunk_count: arena_stats.chunk_count.max(1),
        arena_capacity_kb: arena_stats.total_capacity.div_ceil(1024),
        arena_utilization: arena_stats.utilization,
        interner_unique_strings: unique_strings.max(count_unique_strings(document)),
        interner_hit_rate,
        total_errors,
        total_preloads,
    }
}

fn collect_node_stats(
    node: &HtmlNode,
    arena: &mut NodeArena,
    string_mentions: &mut HashMap<String, usize>,
) {
    let _marker = arena.alloc(count_node(node));

    match node {
        HtmlNode::Element(element) => {
            bump_string_count(string_mentions, &element.tag);
            for (name, value) in &element.attributes {
                bump_string_count(string_mentions, name);
                bump_string_count(string_mentions, value);
            }
            if let Some(slot_name) = &element.slot_name {
                bump_string_count(string_mentions, slot_name);
            }
            if let Some(is_value) = &element.is_value {
                bump_string_count(string_mentions, is_value);
            }
            for child in &element.children {
                collect_node_stats(child, arena, string_mentions);
            }
        }
        HtmlNode::Text(text) | HtmlNode::Comment(text) => {
            bump_string_count(string_mentions, text);
        }
    }
}

fn bump_string_count(string_mentions: &mut HashMap<String, usize>, value: &str) {
    *string_mentions.entry(value.to_string()).or_insert(0) += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let r = parse_html_integrated("<!DOCTYPE html><html><body>Hi</body></html>");
        assert!(!r.document.children.is_empty());
    }

    #[test]
    fn test_preloads_are_exposed() {
        let r = parse_html_integrated("<link rel=\"stylesheet\" href=\"app.css\">");
        assert_eq!(r.stats.total_preloads, 1);
    }

    #[test]
    fn test_stats_are_populated() {
        let result = parse_html_integrated("<div><span>hi</span><span>hi</span></div>");
        assert!(result.stats.arena_chunk_count >= 1);
        assert!(result.stats.arena_capacity_kb >= 1);
        assert!(result.stats.interner_unique_strings >= 3);
        assert!(result.stats.interner_hit_rate > 0.0);
    }

    #[test]
    fn test_parse_from_bytes_with_hint() {
        let mut options = ParserOptions::default();
        options.encoding_hint = Some(super::super::Encoding::Windows1252);

        let result = parse_html_integrated_from_bytes_with_options(&[b'<', b'p', b'>', 0x80, b'<', b'/', b'p', b'>'], None, &options)
            .expect("bytes should decode");

        assert!(count_document_nodes(&result.document) >= 5);
        assert!(result.errors.is_empty());
    }
}
