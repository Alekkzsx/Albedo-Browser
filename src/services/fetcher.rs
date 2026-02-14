pub fn fetch_url(url: &str) -> (String, String) {
    println!("[Fetcher] Fetching external content via Reqwest from: {}", url);
    match reqwest::blocking::get(url) {
        Ok(resp) => {
            println!("[Fetcher] Response received (Status: {}).", resp.status());
            let title = url.to_string(); // Simple title for now, ideally parsed from HTML
            match resp.text() {
                Ok(text) => (title, text),
                Err(e) => {
                    println!("[Fetcher] Encoding Error: {}", e);
                    ("Encoding Error".to_string(), format!("<h1>Encoding Error</h1><p>{}</p>", e))
                },
            }
        },
        Err(e) => {
            println!("[Fetcher] Connection Error: {}", e);
            ("Connection Error".to_string(), format!("<h1>Connection Error</h1><p>{}</p>", e))
        },
    }
}
