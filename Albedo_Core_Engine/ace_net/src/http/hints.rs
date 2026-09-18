//! # Resource Hints (Early Hints 103 & Link Preload)
//! 
//! Dispatcher de respostas parciais (103 Early Hints) e parser de cabeçalhos
//! `Link: <...>; rel=preload` para acelerar o First Contentful Paint.


use tokio::sync::mpsc;
use smol_str::SmolStr;
use url::Url;

/// Dica de recurso (Resource Hint) extraída da rede.
#[derive(Debug, Clone, PartialEq)]
pub struct EarlyHint {
    pub url: Url,
    pub rel: SmolStr,
    pub as_type: Option<SmolStr>,
    pub crossorigin: bool,
}

/// Canal para injetar hints de rede assincronamente no pipeline do DOM.
#[derive(Debug, Clone)]
pub struct EarlyHintsDispatcher {
    sender: mpsc::Sender<EarlyHint>,
}

impl EarlyHintsDispatcher {
    pub fn new(sender: mpsc::Sender<EarlyHint>) -> Self {
        Self { sender }
    }

    /// Faz o parse do cabeçalho `Link` (RFC 8288) e envia pelo canal.
    pub async fn dispatch_link_header(&self, base_url: &Url, link_value: &str) {
        // Exemplo: <https://example.com/style.css>; rel=preload; as=style
        for part in link_value.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            // Parser manual hiper-rápido (zero regex)
            let mut iter = part.split(';');
            let url_part = iter.next().unwrap_or("").trim();
            
            if !url_part.starts_with('<') || !url_part.ends_with('>') {
                continue;
            }
            
            let url_str = &url_part[1..url_part.len()-1];
            let resolved_url = match base_url.join(url_str) {
                Ok(u) => u,
                Err(_) => continue,
            };

            let mut rel = None;
            let mut as_type = None;
            let mut crossorigin = false;

            for param in iter {
                let param = param.trim();
                if let Some((k, v)) = param.split_once('=') {
                    let k = k.trim().to_lowercase();
                    let v = v.trim().trim_matches('"');
                    if k == "rel" {
                        rel = Some(SmolStr::new(v));
                    } else if k == "as" {
                        as_type = Some(SmolStr::new(v));
                    }
                } else {
                    let param_lower = param.to_lowercase();
                    if param_lower == "crossorigin" {
                        crossorigin = true;
                    }
                }
            }

            if let Some(r) = rel {
                if r == "preload" || r == "preconnect" || r == "dns-prefetch" {
                    let hint = EarlyHint {
                        url: resolved_url,
                        rel: r,
                        as_type,
                        crossorigin,
                    };
                    // Dispara e esquece. Backpressure natural é ignorado em hints.
                    let _ = self.sender.send(hint).await;
                }
            }
        }
    }
}
