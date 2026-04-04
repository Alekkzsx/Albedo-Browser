use albedo::ace::html::{
    decode_html_bytes, parse_document_from_bytes_with_errors_and_options,
    parse_document_with_errors_and_options, parse_html_integrated_from_bytes_with_options,
    parse_html_integrated_with_options, Encoding, ParserOptions, PreloadRequest,
    StreamingHtmlParser,
};

fn canonicalize_preloads(requests: &[PreloadRequest]) -> Vec<String> {
    let mut items = requests
        .iter()
        .map(|request| {
            format!(
                "{}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{}|{}|{}",
                request.url,
                request.resource_type,
                request.priority,
                request.crossorigin,
                request.rel,
                request.as_attribute,
                request.fetchpriority,
                request.loading,
                request.is_module,
                request.is_async,
                request.is_defer,
            )
        })
        .collect::<Vec<_>>();
    items.sort();
    items
}

#[test]
fn batch_and_integrated_string_paths_match() {
    let mut options = ParserOptions::default();
    options.base_url = Some("https://example.com/app/".to_string());

    let input = "<!DOCTYPE html><link rel=\"stylesheet\" href=\"theme.css\"><div><span>hi</span><span>hi</span></div>";

    let batch = parse_document_with_errors_and_options(input, &options);
    let integrated = parse_html_integrated_with_options(input, &options);

    assert_eq!(batch.document, integrated.document);
    assert_eq!(batch.parse_errors(), integrated.parse_errors);
    assert_eq!(
        canonicalize_preloads(&batch.preload_requests),
        canonicalize_preloads(&integrated.preload_requests)
    );
    assert_eq!(integrated.stats.total_errors, integrated.parse_errors.len());
}

#[test]
fn byte_paths_match_string_path_with_encoding_hint() {
    let mut options = ParserOptions::default();
    options.encoding_hint = Some(Encoding::Windows1252);

    let bytes = [b'<', b'p', b'>', 0x80, b'<', b'/', b'p', b'>'];
    let decoded = decode_html_bytes(&bytes, None, options.encoding_hint).expect("bytes should decode");
    let batch_from_string = parse_document_with_errors_and_options(&decoded.content, &options);
    let batch_from_bytes = parse_document_from_bytes_with_errors_and_options(&bytes, None, &options)
        .expect("batch bytes should decode");
    let integrated_from_bytes =
        parse_html_integrated_from_bytes_with_options(&bytes, None, &options)
            .expect("integrated bytes should decode");

    assert_eq!(batch_from_string.document, batch_from_bytes.document);
    assert_eq!(batch_from_string.parse_errors(), batch_from_bytes.parse_errors());
    assert_eq!(batch_from_bytes.document, integrated_from_bytes.document);
    assert_eq!(batch_from_bytes.parse_errors(), integrated_from_bytes.parse_errors);
    assert_eq!(integrated_from_bytes.stats.total_errors, integrated_from_bytes.parse_errors.len());
}

#[test]
fn streaming_path_matches_integrated_path_for_chunked_input() {
    let mut options = ParserOptions::default();
    options.base_url = Some("https://example.com/".to_string());

    let input = concat!(
        "<link rel=\"stylesheet\" href=\"app.css\">",
        "<table>before<tr><td>A</td></tr></table>",
        "<svg><foreignObject><div>ok</div></foreignObject></svg>",
    );

    let batch = parse_html_integrated_with_options(input, &options);
    let mut streaming = StreamingHtmlParser::with_options(options);

    for chunk in [
        "<link rel=\"stylesheet\" href=\"app.css\">",
        "<table>before",
        "<tr><td>A</td></tr></table>",
        "<svg><foreignObject><div>ok</div></foreignObject></svg>",
    ] {
        assert!(!matches!(
            streaming.feed(chunk),
            albedo::ace::html::streaming::ChunkResult::Error(_)
        ));
    }

    let streamed = streaming.end_with_parse_result();

    assert_eq!(batch.document, streamed.document);
    assert_eq!(batch.parse_errors, streamed.parse_errors);
    assert_eq!(
        canonicalize_preloads(&batch.preload_requests),
        canonicalize_preloads(&streamed.preload_requests)
    );
    assert_eq!(batch.stats.total_errors, streamed.stats.total_errors);
    assert_eq!(batch.stats.total_preloads, streamed.stats.total_preloads);
}
