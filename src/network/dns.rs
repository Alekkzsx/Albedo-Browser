use std::collections::HashSet;
use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex};
use std::thread;

/// Service for pre-resolving DNS domains
#[derive(Debug, Clone)]
pub struct DnsPrefetcher {
    resolved_hosts: Arc<Mutex<HashSet<String>>>,
}

impl DnsPrefetcher {
    pub fn new() -> Self {
        Self {
            resolved_hosts: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Prefetch DNS for a given hostname
    pub fn prefetch(&self, hostname: String) {
        let mut resolved = self.resolved_hosts.lock().unwrap();
        if resolved.contains(&hostname) {
            return;
        }
        resolved.insert(hostname.clone());

        // Spawn a background thread to resolve the DNS
        // This relies on the OS's DNS cache
        thread::spawn(move || {
            let address_with_port = format!("{}:80", hostname);
            if let Ok(iter) = address_with_port.to_socket_addrs() {
                // Just iterating warms up the OS cache
                for _ in iter {}
            }
        });
    }

    /// Scan text for potential hostnames and prefetch them
    pub fn scan_and_prefetch(&self, text: &str) {
        // Simple heuristic to find domains (very basic)
        // In a real browser, the HTML parser would trigger this for links
        // finding strings like "example.com" or "https://example.com"

        let mut start = 0;
        while let Some(idx) = text[start..].find("://") {
            let protocol_end = start + idx + 3;
            if let Some(end) = text[protocol_end..].find(&['/', '"', '\'', ' ', '<', '>'][..]) {
                let hostname = &text[protocol_end..protocol_end + end];
                if !hostname.is_empty() {
                    self.prefetch(hostname.to_string());
                }
                start = protocol_end + end;
            } else {
                break;
            }
        }
    }
}
