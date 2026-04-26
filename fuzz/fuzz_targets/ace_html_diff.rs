#![no_main]

use albedo::ace::html::{
    parse_document_from_bytes_with_errors_and_options, parse_html_integrated_with_options,
    ParserOptions, StreamingHtmlParser,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data).to_string();
    let options = ParserOptions::default();

    let batch = parse_html_integrated_with_options(&input, &options);
    let from_bytes = parse_document_from_bytes_with_errors_and_options(data, None, &options);
    if let Ok(from_bytes) = from_bytes {
        assert_eq!(batch.document, from_bytes.document);
    }

    let mut streaming = StreamingHtmlParser::with_options(options);
    let mut cursor = 0usize;
    while cursor < data.len() {
        let end = (cursor + 64).min(data.len());
        let _ = streaming.feed_bytes(&data[cursor..end]);
        cursor = end;
    }
    let streamed = streaming.end_with_parse_result();
    assert_eq!(batch.document, streamed.document);
});
