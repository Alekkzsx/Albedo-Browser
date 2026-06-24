use super::*;
use super::runtime::JsRuntime;
use crate::ace::engine::AceEngine;
use std::sync::{Arc, Mutex};



pub(crate) fn get_origin(url: &str) -> &str {
    if let Some(pos) = url.find("://") {
        let rest = &url[pos + 3..];
        if let Some(slash_pos) = rest.find('/') {
            return &url[..pos + 3 + slash_pos];
        } else {
            return url;
        }
    }
    url
}
