/// Shared types between ace/ and network/ modules.
/// This module breaks circular dependencies by providing a neutral location
/// for types that both modules need to import.

pub mod url {
    pub use crate::ace::url::types::{Host, Url, UrlError};
}

pub mod json {
    pub use albedo_jit::contracts::core::JsonValue;
    pub use crate::ace::json::{parse, stringify};
}

pub mod intercept {
    pub use crate::ace::runtime::core::service_worker::InterceptResult;
}

pub mod security {
    pub use crate::network::security::Origin;
}

pub mod websocket {
    pub use crate::network::protocols::websocket::{WebSocketClient, WsEvent, WsCommand};
}
