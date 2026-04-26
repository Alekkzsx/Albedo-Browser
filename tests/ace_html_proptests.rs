use albedo::ace::html::{
    parse_document_from_bytes_with_errors_and_options, parse_html_integrated_with_options,
    ParserOptions, StreamingHtmlParser,
};
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_streaming_matches_batch_for_random_chunks(
        input in ".{0,2048}",
        chunk_sizes in prop::collection::vec(1usize..256, 0..32)
    ) {
        let options = ParserOptions::default();
        let batch = parse_html_integrated_with_options(&input, &options);
        let mut streaming = StreamingHtmlParser::with_options(options);

        let bytes = input.as_bytes();
        let mut cursor = 0usize;
        for size in chunk_sizes {
            if cursor >= bytes.len() {
                break;
            }
            let end = (cursor + size).min(bytes.len());
            let outcome = streaming.feed_bytes(&bytes[cursor..end]);
            prop_assert!(
                !matches!(outcome, albedo::ace::html::streaming::ChunkResult::Error(_)),
                "streaming feed failed at chunk {}..{}",
                cursor,
                end
            );
            cursor = end;
        }

        if cursor < bytes.len() {
            let outcome = streaming.feed_bytes(&bytes[cursor..]);
            prop_assert!(
                !matches!(outcome, albedo::ace::html::streaming::ChunkResult::Error(_)),
                "streaming feed failed at final chunk {}..{}",
                cursor,
                bytes.len()
            );
        }

        let streamed = streaming.end_with_parse_result();
        prop_assert_eq!(batch.document, streamed.document);
        prop_assert_eq!(batch.parse_errors, streamed.parse_errors);
    }
}

proptest! {
    #[test]
    fn prop_byte_path_never_panics_and_matches_string_path(
        input in ".{0,1024}"
    ) {
        let options = ParserOptions::default();
        let from_string = parse_html_integrated_with_options(&input, &options);
        let from_bytes = parse_document_from_bytes_with_errors_and_options(input.as_bytes(), None, &options)
            .expect("byte path should decode");

        prop_assert_eq!(from_string.document, from_bytes.document);
    }
}
