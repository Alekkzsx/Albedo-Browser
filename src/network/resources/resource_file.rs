            tokio::spawn(async move {
                let path_buf = if let Ok(_parsed_url) = crate::ace::url::parse(&url_clone, None) {
                    // Nota: WHATWG URL Standard trata caminhos de forma diferente.
                    // Para o MVP de migração, simulamos a extração de caminho de arquivo.
                    let path_str = url_clone.trim_start_matches("file://");
                    let mut p = crate::ace::url::percent_encoding::decode(path_str);
                    if cfg!(windows) && p.starts_with('/') {
                        let chars: Vec<char> = p.chars().collect();
                        if chars.len() > 3 && chars[1].is_ascii_alphabetic() && chars[2] == ':'
                        {
                            p = p[1..].to_string();
                        }
                    }
                    std::path::PathBuf::from(p)
                } else {
                    let path_str = url_clone.trim_start_matches("file://");
                    let mut p = crate::ace::url::percent_encoding::decode(path_str);
                    if cfg!(windows) && p.starts_with('/') {
                        let chars: Vec<char> = p.chars().collect();
                        if chars.len() > 3 && chars[1].is_ascii_alphabetic() && chars[2] == ':' {
                            p = p[1..].to_string();
                        }
                    }
                    std::path::PathBuf::from(p)
                };

                let path = path_buf.as_path();

                match tokio::fs::read(path).await {
                    Ok(data) => {
                        // Guess content type manually
                        let extension = path
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let content_type = match extension.as_str() {
                            "html" | "htm" => "text/html",
                            "css" => "text/css",
                            "js" => "application/javascript",
                            "json" => "application/json",
                            "png" => "image/png",
                            "jpg" | "jpeg" => "image/jpeg",
                            "svg" => "image/svg+xml",
                            "txt" => "text/plain",
                            _ => "application/octet-stream",
                        }
                        .to_string();

                        let response = ResourceResponse {
                            url: url_clone,
                            data: data.clone(),
                            resource_type,
                            etag: None,
                            cache_control: None,
                            last_modified: None,
                            expires: None,
                            timestamp: std::time::SystemTime::now(),
                            content_type,
                            status_code: 200,
                            original_size: data.len(),
                            compressed_with: crate::network::cache::CompressionMethod::None,
                            decoded_image: None,
                        };

                        Self::send_response(&tx, response);
                    }
                    Err(e) => {
                        tracing::error!(path = %path.display(), ?e, "Failed to read file");
                        let response = ResourceResponse {
                            url: url_clone,
                            data: Vec::new(),
                            resource_type,
                            etag: None,
                            cache_control: None,
                            last_modified: None,
                            expires: None,
                            timestamp: std::time::SystemTime::now(),
                            content_type: "text/plain".to_string(),
                            status_code: 404,
                            original_size: 0,
                            compressed_with: crate::network::cache::CompressionMethod::None,
                            decoded_image: None,
                        };
                        Self::send_response(&tx, response);
                    }
                }
            });
