use ace_core::text::LineIndex;

#[test]
fn test_line_index_conversions_and_ranges() {
    let source = "function add(a, b) {\n    return a + b;\n}\n";
    let index = LineIndex::new(source);

    assert_eq!(index.line_count(), 4);

    // Linha 1: "function add(a, b) {\n"
    assert_eq!(index.line_col(0), (1, 1));
    assert_eq!(index.line_col(8), (1, 9)); // ' '

    // Linha 2: "    return a + b;\n"
    let line2_start = 21; // após '\n' da linha 1
    assert_eq!(index.line_col(line2_start), (2, 1));
    assert_eq!(index.line_col(line2_start + 4), (2, 5)); // 'r' de return

    // Conversão inversa
    assert_eq!(index.offset(1, 1), Some(0));
    assert_eq!(index.offset(2, 5), Some(line2_start + 4));

    // Intervalo de linha
    let range = index.line_range(1).unwrap();
    assert_eq!(&source[range.0..range.1], "function add(a, b) {\n");
}
