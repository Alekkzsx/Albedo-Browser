use ace_net::parse_data_uri;

#[test]
fn test_parse_base64_data_uri() {
    // "Hello World" em base64 é "SGVsbG8gV29ybGQ="
    let uri = "data:text/plain;base64,SGVsbG8gV29ybGQ=";
    let parsed = parse_data_uri(uri).expect("Falha ao parsear Base64 limpo");

    assert_eq!(parsed.mime_type, "text/plain");
    assert_eq!(parsed.data, b"Hello World");
}

#[test]
fn test_parse_base64_with_spaces() {
    // HTML na web frequentemente tem quebras de linha e espaços no Base64
    let uri = "data:image/png;base64,SGVsb G8gV29\r\nybG Q=";
    let parsed = parse_data_uri(uri).expect("Falha ao lidar com quebras de linha");

    assert_eq!(parsed.mime_type, "image/png");
    assert_eq!(parsed.data, b"Hello World");
}

#[test]
fn test_parse_url_encoded_data_uri() {
    let uri = "data:text/html,%3Ch1%3EHello%20World%3C%2Fh1%3E";
    let parsed = parse_data_uri(uri).expect("Falha ao parsear Percent-Encoding");

    assert_eq!(parsed.mime_type, "text/html");
    assert_eq!(parsed.data, b"<h1>Hello World</h1>");
}

#[test]
fn test_parse_default_mime_type() {
    let uri = "data:,Hello%20World";
    let parsed = parse_data_uri(uri).expect("Falha ao herdar o MIME Type padrão");

    assert_eq!(parsed.mime_type, "text/plain;charset=US-ASCII");
    assert_eq!(parsed.data, b"Hello World");
}
