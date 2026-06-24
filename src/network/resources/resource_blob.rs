                // Access global blob store
                if let Some(blob) = crate::network::blob::GLOBAL_BLOB_STORE.get_blob(&url_clone) {
                    let response = ResourceResponse {
                        url: url_clone,
                        data: blob.data,
                        resource_type,
                        etag: None,
                        cache_control: None,
                        last_modified: None,
                        expires: None,
                        timestamp: std::time::SystemTime::now(),
                        content_type: blob.content_type,
                        status_code: 200,
                        original_size: blob.size,
                        compressed_with: crate::network::cache::CompressionMethod::None,
                        decoded_image: None,
                    };
                    Self::send_response(&tx, response);
                } else {
                    // Blob not found (404)
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
            });
