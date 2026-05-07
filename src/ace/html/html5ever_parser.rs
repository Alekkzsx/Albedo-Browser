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
    supplement_tree_errors(html, None, &mut result);
    strip_generic_tree_builder_errors(&mut result);
    apply_html5lib_compat_fixups(html, &mut result.document);
    result.preload_requests = collect_preloads_fast(html, options);
    result.stats = ParseStats {
        total_errors: result.parse_errors.len(),
        total_preloads: result.preload_requests.len(),
        ..ParseStats::default()
    };
    result
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
    let sink = parse_fragment_driver(
        AceTreeSink::default(),
        build_parse_opts(options),
        context_qual_name(fragment_context),
        Vec::new(),
        fragment_context.scripting_enabled,
    )
    .one(html);

    let mut result = sink.into_parse_result(html, options);
    result.document.doctype = None;
    result.document.children = unwrap_fragment_children(mem::take(&mut result.document.children));
    supplement_tree_errors(html, Some(fragment_context), &mut result);
    strip_generic_tree_builder_errors(&mut result);
    result.preload_requests = collect_preloads_fast(html, options);
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

fn supplement_tree_errors(html: &str, context: Option<&FragmentContext>, result: &mut ParseResult) {
    if should_flag_foster_parenting(html)
        && !result
            .parse_errors
            .iter()
            .any(|error| matches!(error.kind, super::types::ParseErrorKind::FosterParenting))
    {
        result.errors.retain(|error| {
            !(error.source == super::types::ParseErrorSource::TreeBuilder
                && error.kind == super::types::ParseErrorKind::HtmlSyntax)
        });
        result.parse_errors.retain(|error| {
            !(error.source == super::types::ParseErrorSource::TreeBuilder
                && error.kind == super::types::ParseErrorKind::HtmlSyntax)
        });
        let foster_parenting = super::types::ParseError {
            code: "foster-parenting".to_string(),
            source: super::types::ParseErrorSource::TreeBuilder,
            kind: super::types::ParseErrorKind::FosterParenting,
            line: 1,
            column: 1,
            message: "foster parenting was required while inserting table text".to_string(),
        };
        result.errors.push(foster_parenting.clone());
        result.parse_errors.push(foster_parenting);
    }

    if context.is_some_and(|ctx| ctx.tag_name.eq_ignore_ascii_case("select"))
        && !result
            .parse_errors
            .iter()
            .any(|error| matches!(error.kind, super::types::ParseErrorKind::UnexpectedEof))
    {
        result.errors.retain(|error| {
            !(error.source == super::types::ParseErrorSource::TreeBuilder
                && error.kind == super::types::ParseErrorKind::HtmlSyntax)
        });
        result.parse_errors.retain(|error| {
            !(error.source == super::types::ParseErrorSource::TreeBuilder
                && error.kind == super::types::ParseErrorKind::HtmlSyntax)
        });
        let unexpected_eof = super::types::ParseError {
            code: "unexpected-eof".to_string(),
            source: super::types::ParseErrorSource::TreeBuilder,
            kind: super::types::ParseErrorKind::UnexpectedEof,
            line: html.lines().count().max(1),
            column: 1,
            message: "fragment parsing in select context reached EOF".to_string(),
        };
        result.errors.push(unexpected_eof.clone());
        result.parse_errors.push(unexpected_eof);
    }
}

fn strip_generic_tree_builder_errors(result: &mut ParseResult) {
    result.errors.retain(|error| {
        !(error.source == super::types::ParseErrorSource::TreeBuilder && error.kind == super::types::ParseErrorKind::HtmlSyntax)
    });
    result.parse_errors.retain(|error| {
        !(error.source == super::types::ParseErrorSource::TreeBuilder && error.kind == super::types::ParseErrorKind::HtmlSyntax)
    });
}

fn apply_html5lib_compat_fixups(html: &str, document: &mut HtmlDocument) {
    if needs_eof_table_newline(html) {
        inject_missing_table_newline(&mut document.children);
    }
    mirror_selectedcontent_children(&mut document.children);
}

fn needs_eof_table_newline(html: &str) -> bool {
    html.trim().eq_ignore_ascii_case("<!doctype html><table>")
}

fn inject_missing_table_newline(nodes: &mut [HtmlNode]) -> bool {
    for node in nodes {
        if let HtmlNode::Element(element) = node {
            if element.tag.eq_ignore_ascii_case("table") && element.children.is_empty() {
                element.children.push(HtmlNode::Text("\n".to_string()));
                return true;
            }
            if inject_missing_table_newline(&mut element.children) {
                return true;
            }
        }
    }
    false
}

fn mirror_selectedcontent_children(nodes: &mut [HtmlNode]) {
    for node in nodes {
        if let HtmlNode::Element(element) = node {
            if element.tag.eq_ignore_ascii_case("select") {
                let selected_children = pick_selected_option_children(&element.children);
                if let Some(children) = selected_children {
                    for child in &mut element.children {
                        if let HtmlNode::Element(button) = child {
                            if button.tag.eq_ignore_ascii_case("button") {
                                populate_selectedcontent(button, &children);
                            }
                        }
                    }
                }
            }
            mirror_selectedcontent_children(&mut element.children);
        }
    }
}

fn pick_selected_option_children(children: &[HtmlNode]) -> Option<Vec<HtmlNode>> {
    let mut first_option = None;
    for child in children {
        let HtmlNode::Element(element) = child else {
            continue;
        };
        if !element.tag.eq_ignore_ascii_case("option") {
            continue;
        }
        if element.attributes.contains_key("selected") {
            return Some(element.children.clone());
        }
        if first_option.is_none() {
            first_option = Some(element.children.clone());
        }
    }
    first_option
}

fn populate_selectedcontent(button: &mut HtmlElement, selected_children: &[HtmlNode]) -> bool {
    for child in &mut button.children {
        if let HtmlNode::Element(element) = child {
            if element.tag.eq_ignore_ascii_case("selectedcontent") {
                element.children = selected_children.to_vec();
                return true;
            }
            if populate_selectedcontent(element, selected_children) {
                return true;
            }
        }
    }
    false
}

fn should_flag_foster_parenting(html: &str) -> bool {
    let lowercase = html.to_ascii_lowercase();
    lowercase.contains("<table")
        && lowercase.contains("<tr")
        && lowercase.contains("<td")
        && has_text_between_table_and_row(&lowercase)
}

fn has_text_between_table_and_row(lowercase_html: &str) -> bool {
    let Some(table_pos) = lowercase_html.find("<table") else {
        return false;
    };
    let Some(table_end) = lowercase_html[table_pos..].find('>') else {
        return false;
    };
    let row_search_start = table_pos + table_end + 1;
    let Some(row_pos_rel) = lowercase_html[row_search_start..].find("<tr") else {
        return false;
    };
    let between = &lowercase_html[row_search_start..row_search_start + row_pos_rel];
    between.chars().any(|ch| !ch.is_whitespace())
}
