use ace_core::collections::BloomFilter;

#[test]
fn test_bloom_filter_insert_and_query() {
    let mut filter = BloomFilter::<16>::new(); // 1024 bits
    assert!(filter.is_empty());
    assert_eq!(filter.count_ones(), 0);

    filter.insert_str("div");
    filter.insert_str("container");
    filter.insert_str("active");

    assert!(!filter.is_empty());
    assert!(filter.count_ones() > 0);

    // Deve conter com 100% de certeza os elementos inseridos (zero falsos negativos)
    assert!(filter.contains_str("div"));
    assert!(filter.contains_str("container"));
    assert!(filter.contains_str("active"));

    // Elementos não inseridos
    assert!(!filter.contains_str("span"));
    assert!(!filter.contains_str("header"));
    assert!(!filter.contains_str("footer"));

    filter.clear();
    assert!(filter.is_empty());
    assert!(!filter.contains_str("div"));
}
