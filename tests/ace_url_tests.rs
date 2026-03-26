use albedo::ace::url;

#[test]
fn test_basic_url_parse() {
    let u = url::parse("https://google.com/search?q=rust#top", None).unwrap();
    assert_eq!(u.scheme, "https");
    assert_eq!(u.host_str(), Some("google.com".to_string()));
    assert_eq!(u.path, vec!["search".to_string()]);
    assert_eq!(u.query, Some("q=rust".to_string()));
    assert_eq!(u.fragment, Some("top".to_string()));
}

#[test]
fn test_url_with_port() {
    let u = url::parse("http://localhost:8080/", None).unwrap();
    assert_eq!(u.scheme, "http");
    assert_eq!(u.host_str(), Some("localhost".to_string()));
    assert_eq!(u.port, Some(8080));
}

#[test]
fn test_search_params() {
    let mut params = url::UrlSearchParams::new(Some("a=1&b=2&a=3"));
    assert_eq!(params.get("a"), Some("1".to_string()));
    assert_eq!(params.get_all("a"), vec!["1".to_string(), "3".to_string()]);
    assert!(params.has("b"));
    
    params.set("b", "4");
    assert_eq!(params.get("b"), Some("4".to_string()));
    
    params.append("c", "5");
    params.delete("a");
    assert!(!params.has("a"));
    let entries: Vec<_> = params.entries().collect();
    assert_eq!(entries.len(), 2);
    assert_eq!(params.to_string(), "b=4&c=5");
}

#[test]
fn test_percent_decoding() {
    let decoded = albedo::ace::url::percent_encoding::decode("hello%20world%21");
    assert_eq!(decoded, "hello world!");
}

#[test]
fn test_search_params_complex() {
    let mut params = url::UrlSearchParams::new(Some("a=1&b=2&a=3"));
    params.set("a", "4");
    assert_eq!(params.get_all("a"), vec!["4".to_string()]);
    assert_eq!(params.to_string(), "a=4&b=2");
    
    params.append("a", "5");
    assert_eq!(params.get_all("a"), vec!["4".to_string(), "5".to_string()]);
    assert_eq!(params.to_string(), "a=4&b=2&a=5");
}

#[test]
fn test_url_as_str_integration() {
    let u = url::parse("https://example.com/p", None).unwrap();
    assert_eq!(u.as_str(), "https://example.com/p");
}
