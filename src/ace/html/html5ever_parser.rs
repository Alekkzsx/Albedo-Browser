use std::borrow::Cow;
use std::mem;

use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{
    ns, parse_document as parse_document_driver, parse_fragment as parse_fragment_driver,
    LocalName, ParseOpts, QualName,
};
use markup5ever::interface::tree_builder::{QuirksMode};

use super::types::{
    FragmentContext, HtmlDocument, HtmlNode, HtmlElement, Namespace, ParseResult,
    ParseStats, ParserOptions,
};
use super::sink::{AceTreeSink};
use super::preloads::collect_preloads_fast;

pub(crate) fn parse_document_html5ever(html: &str, options: &ParserOptions) -> ParseResult {
    let sink = parse_document_driver(AceTreeSink::default(), build_parse_opts(options)).one(html);
    let mut result = sink.into_parse_result(html, options);
    if options.scripting_enabled {
        result.document.children = super::transform_noscript(result.document.children);
    }
    strip_generic_tree_builder_errors(&mut result);
    result.preload_requests = collect_preloads_fast(html, options);
    result.stats = ParseStats {
        total_errors: result.parse_errors.len(),
        total_preloads: result.preload_requests.len(),
        ..ParseStats::default()
    };
    result
}

fn strip_generic_tree_builder_errors(result: &mut ParseResult) {
    result.errors.retain(|error| {
        !(error.source == super::types::ParseErrorSource::TreeBuilder && error.kind == super::types::ParseErrorKind::HtmlSyntax)
    });
    result.parse_errors.retain(|error| {
        !(error.source == super::types::ParseErrorSource::TreeBuilder && error.kind == super::types::ParseErrorKind::HtmlSyntax)
    });
}

pub(crate) fn parse_fragment_html5ever(
    html: &str,
    context: Option<&FragmentContext>,
    options: &ParserOptions,
) -> ParseResult {
    if context.is_none() {
        let mut result = parse_document_html5ever(html, options);
        let children = mem::take(&mut result.document.children);
        result.document = HtmlDocument {
            doctype: None,
            children,
        };
        result.stats = ParseStats {
            total_errors: result.parse_errors.len(),
            total_preloads: result.preload_requests.len(),
            ..ParseStats::default()
        };
        return result;
    }

    let fragment_context = context.expect("checked above");
    let mut options = options.clone();
    options.fragment_context = Some(fragment_context.clone());

    let sink = parse_fragment_driver(
        AceTreeSink::default(),
        build_parse_opts(&options),
        context_qual_name(fragment_context),
        Vec::new(),
        fragment_context.scripting_enabled,
    )
    .one(html);

    let mut result = sink.into_parse_result(html, &options);
    result.document.doctype = None;
    result.document.children = unwrap_fragment_children(mem::take(&mut result.document.children));
    if options.scripting_enabled {
        result.document.children = super::transform_noscript(result.document.children);
    }
    strip_generic_tree_builder_errors(&mut result);
    result.preload_requests = collect_preloads_fast(html, &options);
    result.stats = ParseStats {
        total_errors: result.parse_errors.len(),
        total_preloads: result.preload_requests.len(),
        ..ParseStats::default()
    };
    result
}

fn unwrap_fragment_children(mut children: Vec<HtmlNode>) -> Vec<HtmlNode> {
    if children.len() != 1 {
        return children;
    }

    match children.pop() {
        Some(HtmlNode::Element(element)) if element.tag.eq_ignore_ascii_case("html") => {
            element.children
        }
        Some(other) => vec![other],
        None => Vec::new(),
    }
}

fn build_parse_opts(options: &ParserOptions) -> ParseOpts {
    ParseOpts {
        tree_builder: TreeBuilderOpts {
            scripting_enabled: options.scripting_enabled,
            drop_doctype: false,
            exact_errors: false,
            ..TreeBuilderOpts::default()
        },
        ..ParseOpts::default()
    }
}

fn context_qual_name(context: &FragmentContext) -> QualName {
    let ns = match context.namespace {
        Namespace::Html => ns!(html),
        Namespace::Svg => ns!(svg),
        Namespace::MathMl => ns!(mathml),
    };

    QualName::new(None, ns, LocalName::from(context.tag_name.as_str()))
}

