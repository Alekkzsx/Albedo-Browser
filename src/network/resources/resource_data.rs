{
                if let Some(comma_pos) = url_clone.find(',') {
                    let metadata = &url_clone[5..comma_pos];
                    let data_part = &url_clone[comma_pos + 1..];

                    let is_base64 = metadata.ends_with(";base64");
                    let content_type = if is_base64 {
                        metadata.trim_end_matches(";base64").to_string()
                    } else {
                        metadata.to_string()
                    };

                    let content_type = if content_type.is_empty() {
                        "text/plain;charset=US-ASCII".to_string()
                    } else {
                        content_type
                    };

                    let data = if is_base64 {
                        crate::utils::base64::decode(data_part).unwrap_or_default()
                    } else {
                        crate::ace::url::percent_encoding::decode(data_part).into_bytes()
                    };

                    let response = ResourceResponse {
                        url: url_clone,
                        data,
                        resource_type,
                        etag: None,
                        cache_control: None,
                        last_modified: None,
                        expires: None,
                        timestamp: std::time::SystemTime::now(),
                        content_type,
                        status_code: 200,
                        original_size: 0, // data URLs don't have original size in the same concept
                        compressed_with: crate::network::cache::CompressionMethod::None,
                        decoded_image: None,
                    };

                    Self::send_response(&tx, response);
                }
}
